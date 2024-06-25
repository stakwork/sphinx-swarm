use bollard::container::DownloadFromContainerOptions;
use bollard::Docker;
// use chrono::Local;
use std::fs::{self, File};
use std::io::{Read, Write};
use tar::Archive;
use tokio::io::AsyncWriteExt;
use zip::ZipWriter;
// use rusoto_core::Region;
// use rusoto_s3::{
//     DeleteObjectsRequest, ListObjectsV2Request, ObjectIdentifier, PutObjectRequest, S3Client,
// };
// use std::error::Error;
// use std::path::PathBuf;
use futures_util::stream::TryStreamExt;
use std::io::Cursor;
use zip::write::FileOptions;

use crate::config::STATE;
use crate::utils::domain;

// pub async fn backup_to_s3(
//     proxy_path: &str,
//     relay_path: &str,
//     bucket: &str,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let docker = Docker::connect_with_local_defaults()?;

//     // Download and zip proxy volume
//     let proxy_zip_data =
//         download_and_zip_from_container(&docker, "proxy_container_id", proxy_path).await?;
//     let proxy_zip_file_name = format!("proxy_data_{}_{}.zip", Local::now().format("%Y%m%d_%H%M%S"));
//     let proxy_zip_file = PathBuf::from(&proxy_zip_file_name);
//     fs::write(&proxy_zip_file, proxy_zip_data).await?;

//     // Download and zip relay volume
//     let relay_zip_data =
//         download_and_zip_from_container(&docker, "relay_container_id", relay_path).await?;
//     let relay_zip_file_name = format!("relay_data_{}_{}.zip", Local::now().format("%Y%m%d_%H%M%S"));
//     let relay_zip_file = PathBuf::from(&relay_zip_file_name);
//     fs::write(&relay_zip_file, relay_zip_data).await?;

//     // Upload proxy zip file to S3
//     upload_to_s3(&bucket, &proxy_zip_file_name, proxy_zip_file).await?;

//     // Upload relay zip file to S3
//     upload_to_s3(&bucket, &relay_zip_file_name, relay_zip_file).await?;

//     Ok(())
// }

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
}

// async fn zip_data(data: Vec<u8>, name: &str) -> Result<PathBuf, Box<dyn Error>> {
//     let current_time = Local::now();
//     let zip_file_name = format!("{}_{}.zip", name, current_time.format("%Y%m%d_%H%M%S"));

//     let mut zip_file = zip::ZipWriter::new(File::create(&zip_file_name)?);
//     let options =
//         zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

//     zip_file.start_file(name, options)?;
//     zip_file.write_all(&data)?;

//     Ok(PathBuf::from(zip_file_name))
// }

// Uploads the zip file to S3
// async fn upload_to_s3(
//     bucket: &str,
//     zip_file_name: &str,
//     zip_file: PathBuf,
// ) -> Result<(), Box<dyn Error>> {
//     let s3_client = S3Client::new(Region::default());

//     let file = File::open(&zip_file)?;
//     let mut buffer = Vec::new();
//     file.read_to_end(&mut buffer)?;

//     let key = zip_file_name;

//     let request = PutObjectRequest {
//         bucket: bucket.to_owned(),
//         key: key.to_owned(),
//         body: Some(buffer.into()),
//         ..Default::default()
//     };

//     s3_client.put_object(request).await?;
//     Ok(())
// }

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

// async fn test_backup() -> Result<(), Box<dyn Error>> {
//     let bucket = "your-bucket-name";
//     let zip_data = vec![1, 2, 3, 4, 5]; // Example data to be zipped

//     let zip_file = zip_data(zip_data, "backup_data").await?;
//     println!("Zip file created: {:?}", zip_file);

//     upload_to_s3(bucket, "backup_data.zip", zip_file.clone()).await?;
//     println!("Zip file uploaded to S3");

//     delete_old_backups(bucket, "backup_prefix", 30).await?;
//     println!("Old backups deleted from S3");

//     Ok(())
// }
