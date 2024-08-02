use crate::config::STATE;
use crate::images::DockerHubImage;
use crate::utils::{domain, getenv};
use anyhow::{Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart, Delete, ObjectIdentifier};
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::{ByteStream, Length};
use aws_smithy_types::retry::RetryConfig;
use bollard::container::DownloadFromContainerOptions;
use bollard::Docker;
use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use futures_util::stream::TryStreamExt;
use std::fs::{self, File};
use std::io::Cursor;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs::remove_dir_all;
use tokio_cron_scheduler::{Job, JobScheduler};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

pub static BACK_AND_DELETE: AtomicBool = AtomicBool::new(false);

pub fn bucket_name() -> String {
    getenv("AWS_S3_BUCKET_NAME").unwrap_or("sphinx-swarm".to_string())
}

fn backup_retention_days() -> i64 {
    match getenv("BACKUP_RETENTION_DAYS")
        .unwrap_or("10".to_string())
        .parse()
    {
        Ok(float_value) => return float_value,
        Err(e) => {
            log::error!("Unable to parse BACKUP_RETENTION_DAYS: {}", e);
            return 10;
        }
    }
}

pub async fn backup_containers() -> Result<()> {
    let state = STATE.lock().await;
    let nodes = state.stack.nodes.clone();
    drop(state);

    let mut containers: Vec<(String, String, String)> = Vec::new();

    for node in nodes.iter() {
        let node_name = node.name();
        let hostname = domain(&node_name);
        match node.as_internal() {
            Ok(img) => {
                let to_backup = vec!["relay", "proxy", "neo4j", "boltwall"];
                if to_backup.contains(&node_name.as_str()) {
                    containers.push((hostname.clone(), img.repo().root_volume, node_name.clone()))
                }
            }
            Err(_) => (),
        }
    }

    let (parent_directory, parent_zip) = download_and_zip_from_container(containers).await?;

    // delete parent directory content after zip
    let _ = remove_dir_all(&parent_directory).await;

    upload_final_zip_to_s3(parent_zip).await?;

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
        let options = DownloadFromContainerOptions { path: &volume_path };

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
        let subdirectory = format!("{}/{}", &parent_directory, &sub_directory);

        // Create the subdirectory if it doesn't exist
        fs::create_dir_all(&subdirectory)?;

        // Create a ZIP file to save the content
        let zip_file_path = format!("{}/{}.zip", subdirectory, &sub_directory);
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

        log::info!(
            "Volume from container {} downloaded and saved as a ZIP file in directory {}",
            container_id,
            subdirectory
        );
    }

    let current_timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let parent_zip = format!("{}_{}.zip", &parent_directory, current_timestamp);

    zip_directory(&parent_directory, &parent_zip)?;

    Ok((parent_directory, parent_zip))
}

async fn upload_final_zip_to_s3(parent_zip: String) -> Result<()> {
    match upload_to_s3_multi(&bucket_name(), &parent_zip.clone()).await {
        Ok(status) => {
            if status == true {
                let _ = fs::remove_file(parent_zip);
            }
        }
        Err(err) => {
            log::error!("We are getting somewhere: {}", err)
        }
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

async fn upload_to_s3_multi(bucket: &str, key: &str) -> Result<bool> {
    //In bytes, minimum chunk size of 150MB.
    const CHUNK_SIZE: u64 = 1024 * 1024 * 150;
    const MAX_CHUNKS: u64 = 10000;

    // Read the custom region environment variable
    let region = match getenv("AWS_S3_REGION_NAME") {
        Ok(value) => value,
        Err(_msg) => {
            log::error!("AWS_S3_REGION_NAME is not provided in environment variable");
            return Ok(false);
        }
    };

    // Create a region provider chain
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    // Load the AWS configuration
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    let client = Client::new(&config);

    let multipart_upload_res: CreateMultipartUploadOutput = client
        .create_multipart_upload()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let upload_id = match multipart_upload_res.upload_id() {
        Some(id) => id,
        None => {
            log::error!("Upload ID not found");
            return Ok(false);
        }
    };

    let path = Path::new(&key);
    let file_size = tokio::fs::metadata(path)
        .await
        .expect("it exists I swear")
        .len();

    let mut chunk_count = (file_size / CHUNK_SIZE) + 1;
    let mut size_of_last_chunk = file_size % CHUNK_SIZE;
    if size_of_last_chunk == 0 {
        size_of_last_chunk = CHUNK_SIZE;
        chunk_count -= 1;
    }

    if file_size == 0 {
        log::error!("Invalid file, file size is 0");
        return Ok(false);
    }
    if chunk_count > MAX_CHUNKS {
        log::error!("Too many chunks! Try increasing your chunk size.");
        return Ok(false);
    }

    log::info!("Total number of chunks: {}", chunk_count);

    let mut upload_parts: Vec<CompletedPart> = Vec::new();

    for chunk_index in 0..chunk_count {
        let this_chunk = if chunk_count - 1 == chunk_index {
            size_of_last_chunk
        } else {
            CHUNK_SIZE
        };
        let stream = ByteStream::read_from()
            .path(path)
            .offset(chunk_index * CHUNK_SIZE)
            .length(Length::Exact(this_chunk))
            .build()
            .await?;

        //Chunk index needs to start at 0, but part numbers start at 1.
        let part_number = (chunk_index as i32) + 1;

        let upload_part_res = match client
            .upload_part()
            .key(key)
            .bucket(bucket)
            .upload_id(upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                log::error!("Error uploading part: {}", e);
                return Ok(false);
            }
        };
        upload_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part_res.e_tag.unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
    }

    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();

    let _complete_multipart_upload_res = client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await?;

    Ok(true)
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

    let object_prefix = getenv("HOST")?;

    // List objects in the bucket
    let resp = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(object_prefix)
        .send()
        .await?;

    let objects = resp.contents();

    if objects.len() > 3 {
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

            log::info!(
                "Deleted {} old objects from bucket {}",
                delete_request.deleted().len(),
                bucket
            );
        } else {
            log::info!("No old objects to delete in bucket {}", bucket);
        }
    }

    Ok(())
}

pub async fn backup_and_delete_volumes_cron() -> Result<JobScheduler> {
    log::info!(":backup and delete volumes");
    let sched = JobScheduler::new().await?;

    sched
        .add(Job::new_async("@daily", |_uuid, _l| {
            Box::pin(async move {
                if !BACK_AND_DELETE.load(Ordering::Relaxed) {
                    BACK_AND_DELETE.store(true, Ordering::Relaxed);
                }
            })
        })?)
        .await?;

    sched.start().await?;

    tokio::spawn(async move {
        loop {
            let go = BACK_AND_DELETE.load(Ordering::Relaxed);
            if go {
                if let Err(e) = backup_containers().await {
                    log::error!("Backup Volumes: {:?}", e);
                }
                if let Err(e) = delete_old_backups(&bucket_name(), backup_retention_days()).await {
                    log::error!("Delete Old backup volumes: {:?}", e);
                }

                BACK_AND_DELETE.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}
