use bollard::container::DownloadFromContainerOptions;
use bollard::Docker;
use chrono::Local;
use std::fs::{self, File};
// use tar::Archive;
// use tokio::io::AsyncWriteExt;
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{Client, Error};
use futures_util::stream::TryStreamExt;
use std::io::Cursor;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tokio::fs::remove_dir_all;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::config::STATE;
use crate::utils::{domain, getenv};

pub async fn backup_containers() {
    let state = STATE.lock().await;
    let nodes = state.stack.nodes.clone();

    let mut containers: Vec<(String, String, String)> = Vec::new();

    for node in nodes.iter() {
        let node_name = node.name();
        let hostname = domain(&node_name);
        if node_name == "relay".to_string() {
            containers.push((
                hostname.clone(),
                "/relay/data".to_string(),
                node_name.clone(),
            ))
        }
        if node_name == "proxy".to_string() {
            containers.push((
                hostname.clone(),
                "/app/proxy".to_string(),
                node_name.clone(),
            ))
        }
        if node_name == "neo4j".to_string() {
            containers.push((hostname.clone(), "/data".to_string(), node_name.clone()));
        }
        if node_name == "boltwall".to_string() {
            containers.push((
                hostname.to_string(),
                "/boltwall".to_string(),
                node_name.clone(),
            ))
        }
    }

    println!("Node_names: {:?}", containers);

    download_and_zip_from_container(containers).await;
}

pub async fn download_and_zip_from_container(containers: Vec<(String, String, String)>) {
    // Initialize the Docker client
    let docker = Docker::connect_with_local_defaults().unwrap();

    // Define the parent directory where all the container volumes will be saved
    let parent_directory = "downloaded_volumes";

    // Create the parent directory if it doesn't exist
    fs::create_dir_all(parent_directory).unwrap();

    // Iterate over each container and download its volume
    for (container_id, volume_path, sub_directory) in containers {
        // Options for downloading the volume
        let options = DownloadFromContainerOptions { path: volume_path };

        // Stream the tar content from the container
        let mut stream = docker.download_from_container(&container_id, Some(options));

        // Collect the streamed data into a vector
        let mut tar_data = Vec::new();
        while let Some(chunk) = stream.try_next().await.unwrap() {
            tar_data.extend(&chunk);
        }

        // Create a cursor for the tar data
        let tar_cursor = Cursor::new(tar_data);

        // Create a tar archive from the cursor
        let mut archive = tar::Archive::new(tar_cursor);

        // Define the subdirectory for the current container
        let subdirectory = format!("{}/{}", parent_directory, sub_directory);

        // Create the subdirectory if it doesn't exist
        fs::create_dir_all(&subdirectory).unwrap();

        // Create a ZIP file to save the content
        let zip_file_path = format!("{}/{}.zip", subdirectory, container_id);
        let zip_file = File::create(zip_file_path).unwrap();
        let mut zip_writer = ZipWriter::new(zip_file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Iterate over the entries in the tar archive and write them to the ZIP file
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_owned();

            if path.is_dir() {
                zip_writer
                    .add_directory(path.to_string_lossy(), options)
                    .unwrap();
            } else {
                zip_writer
                    .start_file(path.to_string_lossy(), options)
                    .unwrap();
                let mut buffer = Vec::new();
                entry.read_to_end(&mut buffer).unwrap();
                zip_writer.write_all(&buffer).unwrap();
            }
        }

        zip_writer.finish().unwrap();

        println!(
            "Volume from container {} downloaded and saved as a ZIP file in directory {}",
            container_id, subdirectory
        );
    }

    let current_timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let parent_zip = format!("{}_{}.zip", parent_directory, current_timestamp);

    zip_directory(parent_directory, &parent_zip).unwrap();
    let parent_zip_file = PathBuf::from(&parent_zip);
    let response = upload_to_s3("sphinx-swarm", &parent_zip, parent_zip_file).await;

    match response {
        Ok(status) => {
            if status == true {
                let _ = fs::remove_file(parent_zip);

                let _ = remove_dir_all(parent_directory).await;
            }
        }
        Err(_) => {
            println!("Error occured while sending file to S3 bucket")
        }
    }
}

fn zip_directory(src_dir: &str, zip_file: &str) -> io::Result<()> {
    let file = File::create(zip_file)?;
    let mut zip = ZipWriter::new(file);
    let options = zip::write::FileOptions::default().compression_method(CompressionMethod::Stored);

    for entry in WalkDir::new(src_dir) {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(src_dir).unwrap().to_str().unwrap();

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if path.is_dir() {
            zip.add_directory(name, options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

// Uploads the zip file to S3
async fn upload_to_s3(bucket: &str, zip_file_name: &str, zip_file: PathBuf) -> Result<bool, Error> {
    // Read the custom region environment variable
    let region = getenv("AWS_S3_REGION_NAME").expect("AWS_S3_REGION_NAME must be set in .env file");

    // Create a region provider chain
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    // Load the AWS configuration
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    // Read the file into a ByteStream
    match ByteStream::from_path(&zip_file).await {
        Ok(body) => {
            // Prepare the PutObjectRequest
            let request = client
                .put_object()
                .bucket(bucket)
                .key(zip_file_name)
                .body(body);

            // Send the request
            request.send().await?;
            return Ok(true);
        }
        Err(_) => {
            println!("Error streaming zip file");
            return Ok(false);
        }
    };
}

// Deletes old backups from the S3 bucket
// async fn delete_old_backups(
//     bucket: &str,
//     prefix: &str,
//     retention_days: i64,
// ) -> Result<(), Box<dyn Error>> {
//     let s3_client = S3Client::new(Region::default());

//     let current_time = Local::now();
//     let cutoff_time = current_time - chrono::Duration::days(retention_days);

//     let list_request = ListObjectsV2Request {
//         bucket: bucket.to_owned(),
//         prefix: Some(prefix.to_owned()),
//         ..Default::default()
//     };

//     let objects = s3_client.list_objects_v2(list_request).await?;

//     let objects_to_delete: Vec<ObjectIdentifier> = objects
//         .contents
//         .unwrap_or_default()
//         .iter()
//         .filter(|obj| {
//             obj.last_modified
//                 .as_ref()
//                 .map_or(false, |lm| lm < &cutoff_time)
//         })
//         .map(|obj| ObjectIdentifier {
//             key: obj.key.clone(),
//             ..Default::default()
//         })
//         .collect();

//     if !objects_to_delete.is_empty() {
//         let delete_request = DeleteObjectsRequest {
//             bucket: bucket.to_owned(),
//             delete: rusoto_s3::Delete {
//                 objects: objects_to_delete,
//                 ..Default::default()
//             },
//             ..Default::default()
//         };

//         s3_client.delete_objects(delete_request).await?;
//     }

//     Ok(())
// }
