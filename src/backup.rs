use anyhow::{Context, Result};
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use bollard::container::DownloadFromContainerOptions;
use bollard::Docker;
use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use std::fs::{self, File};
// use tar::Archive;
// use tokio::io::AsyncWriteExt;
use crate::images::DockerHubImage;
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use futures_util::stream::TryStreamExt;
use std::io::Cursor;
use std::io::{Read, Write};
use std::path::PathBuf;
use tokio::fs::remove_dir_all;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::config::STATE;
use crate::utils::{domain, getenv};

fn bucket_name() -> String {
    getenv("AWS_S3_BUCKET_NAME").unwrap_or("sphinx-swarm".to_string())
}

pub async fn backup_containers() -> Result<()> {
    let state = STATE.lock().await;
    let nodes = state.stack.nodes.clone();
    drop(state);

    let mut containers: Vec<(String, String, String)> = Vec::new();

    for node in nodes.iter() {
        let node_name = node.name();
        let hostname = domain(&node_name);
        let img = node.as_internal()?;
        let to_backup = vec!["relay", "proxy", "neo4j", "boltwall"];
        if to_backup.contains(&node_name.as_str()) {
            containers.push((hostname.clone(), img.repo().root_volume, node_name.clone()))
        }
    }

    println!("Node_names: {:?}", containers);

    let (parent_directory, parent_zip) = download_and_zip_from_container(containers).await?;

    upload_final_zip_to_s3(parent_directory, parent_zip).await?;

    Ok(())
}

pub async fn download_and_zip_from_container(
    containers: Vec<(String, String, String)>,
) -> Result<(String, String)> {
    // Initialize the Docker client
    let docker = Docker::connect_with_local_defaults()?;

    // Define the parent directory where all the container volumes will be saved
    let parent_directory = getenv("HOST")?;

    // Create the parent directory if it doesn't exist
    fs::create_dir_all(&parent_directory)?;

    // Iterate over each container and download its volume
    for (container_id, volume_path, sub_directory) in containers {
        // Options for downloading the volume
        let options = DownloadFromContainerOptions { path: volume_path };

        // Stream the tar content from the container
        let mut stream = docker.download_from_container(&container_id, Some(options));

        // Collect the streamed data into a vector
        let mut tar_data = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            tar_data.extend(&chunk);
        }

        // Create a cursor for the tar data
        let tar_cursor = Cursor::new(tar_data);

        // Create a tar archive from the cursor
        let mut archive = tar::Archive::new(tar_cursor);

        // Define the subdirectory for the current container
        let subdirectory = format!("{}/{}", &parent_directory, sub_directory);

        // Create the subdirectory if it doesn't exist
        fs::create_dir_all(&subdirectory)?;

        // Create a ZIP file to save the content
        let zip_file_path = format!("{}/{}.zip", subdirectory, container_id);
        let zip_file = File::create(zip_file_path)?;
        let mut zip_writer = ZipWriter::new(zip_file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Iterate over the entries in the tar archive and write them to the ZIP file
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_owned();

            if path.is_dir() {
                zip_writer.add_directory(path.to_string_lossy(), options)?;
            } else {
                zip_writer.start_file(path.to_string_lossy(), options)?;
                let mut buffer = Vec::new();
                entry.read_to_end(&mut buffer)?;
                zip_writer.write_all(&buffer)?;
            }
        }

        zip_writer.finish()?;

        println!(
            "Volume from container {} downloaded and saved as a ZIP file in directory {}",
            container_id, subdirectory
        );
    }

    let current_timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let parent_zip = format!("{}_{}.zip", &parent_directory, current_timestamp);

    zip_directory(&parent_directory, &parent_zip)?;

    Ok((parent_directory, parent_zip))
}

async fn upload_final_zip_to_s3(parent_directory: String, parent_zip: String) -> Result<()> {
    let parent_zip_file = PathBuf::from(&parent_zip);
    let status = upload_to_s3(&bucket_name(), &parent_zip, parent_zip_file).await?;

    if status == true {
        let _ = fs::remove_file(parent_zip);
        let _ = remove_dir_all(parent_directory).await;
    }
    Ok(())
}

fn zip_directory(src_dir: &str, zip_file: &str) -> Result<()> {
    let file = File::create(zip_file)?;
    let mut zip = ZipWriter::new(file);
    let options = zip::write::FileOptions::default().compression_method(CompressionMethod::Stored);

    for entry in WalkDir::new(src_dir) {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .strip_prefix(src_dir)?
            .to_str()
            .context("non-UTF-8 file name")?;

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
async fn upload_to_s3(bucket: &str, zip_file_name: &str, zip_file: PathBuf) -> Result<bool> {
    // Read the custom region environment variable
    let region = getenv("AWS_S3_REGION_NAME")?;

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
pub async fn delete_old_backups(bucket: &str, retention_days: i64) -> Result<()> {
    // Read the custom region environment variable
    let region = getenv("AWS_S3_REGION_NAME")?;

    // Create a region provider chain
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    // Load the AWS configuration with the custom region
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    // List objects in the bucket
    let resp = client.list_objects_v2().bucket(bucket).send().await?;

    let objects = resp.contents();

    // Filter objects older than retention_days
    let retention_date = Utc::now() - Duration::days(retention_days);
    let mut objects_to_delete = Vec::new();

    for obj in objects {
        if let Some(last_modified) = obj.last_modified {
            let last_modified_timestamp = last_modified.secs();
            let naive_datetime = NaiveDateTime::from_timestamp_opt(last_modified_timestamp, 0)
                .context("Invalid timestamp")?;
            let last_modified_chrono: DateTime<Utc> =
                DateTime::from_naive_utc_and_offset(naive_datetime, Utc);

            if last_modified_chrono < retention_date {
                if let Some(key) = &obj.key {
                    let object_identifier_result = ObjectIdentifier::builder().key(key).build();
                    match object_identifier_result {
                        Ok(object_identifier) => {
                            objects_to_delete.push(object_identifier);
                        }
                        Err(_) => {
                            print!("Could not build object correctly")
                        }
                    }
                }
            }
        }
    }

    if !objects_to_delete.is_empty() {
        // Delete old objects
        let delete_request = client
            .delete_objects()
            .bucket(bucket)
            .delete(
                Delete::builder()
                    .set_objects(Some(objects_to_delete))
                    .build()?,
            )
            .send()
            .await?;

        println!(
            "Deleted {} old objects from bucket {}",
            delete_request.deleted().len(),
            bucket
        );
    } else {
        println!("No old objects to delete in bucket {}", bucket);
    }

    Ok(())
}
