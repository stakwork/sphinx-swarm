use crate::config;
use crate::images::DockerHubImage;
use crate::utils::{domain, getenv};
use anyhow::{bail, Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart, Delete, ObjectIdentifier};
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::{ByteStream, Length};
use aws_smithy_types::retry::RetryConfig;
use bollard::container::DownloadFromContainerOptions;
use bollard::Docker;
use bytes::Bytes;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, Utc};
use futures_util::stream::TryStreamExt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::AsyncReadExt;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::io::StreamReader;
use walkdir::WalkDir;
use zip::CompressionMethod;
use zip::ZipWriter;

pub static BACK_AND_DELETE: AtomicBool = AtomicBool::new(false);

const S3_MULTIPART_CHUNK_SIZE: u64 = 1024 * 1024 * 150;
const S3_MULTIPART_MAX_CHUNKS: u64 = 10000;

pub fn bucket_name() -> String {
    getenv("AWS_S3_BUCKET_NAME").unwrap_or("sphinx-swarm".to_string())
}

fn swarm_prefix_from_host() -> Result<String> {
    let host = getenv("HOST")?;
    let host_slug = host.replace('.', "-");
    Ok(format!("swarm-{}", host_slug))
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

/// True when `name` is a backup staging directory: starts with `swarm`, and the
/// segment after the final `_` parses as `YYYY-MM-DD`. The swarm identifier is
/// not required to be numeric (`swarmprod_2024-01-01` matches; `swarm7` does not).
fn is_backup_staging_dir_name(name: &str) -> bool {
    if !name.starts_with("swarm") {
        return false;
    }
    match name.rsplit_once('_') {
        Some((_, date_seg)) => NaiveDate::parse_from_str(date_seg, "%Y-%m-%d").is_ok(),
        None => false,
    }
}

/// Drain leftover `swarm*_YYYY-MM-DD` staging dirs from prior failed runs.
/// Runs on a blocking thread so a large sweep cannot stall the tokio runtime.
pub async fn sweep_stale_backup_staging_dirs() -> Result<()> {
    let removed = tokio::task::spawn_blocking(|| {
        sweep_stale_backup_staging_dirs_in(&["/vol", "/"])
    })
    .await
    .context("backup staging sweep task panicked")?;
    log::info!(
        "Backup staging sweep: removed {} stale dir(s): {:?}",
        removed.len(),
        removed
    );
    Ok(())
}

fn sweep_stale_backup_staging_dirs_in(roots: &[&str]) -> Vec<String> {
    let mut removed = Vec::new();
    for root in roots {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("Could not read {} for stale staging sweep: {}", root, e);
                continue;
            }
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !is_backup_staging_dir_name(name) {
                continue;
            }
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            match fs::remove_dir_all(&path) {
                Ok(()) => removed.push(path.display().to_string()),
                Err(e) => {
                    log::error!(
                        "Failed to remove stale staging dir {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }
    removed
}

/// Map an `upload_to_s3_multi` result to success or a surfaced error.
/// `Ok(true)` is the only success; `Ok(false)` and `Err` both fail the caller.
fn interpret_s3_upload(result: Result<bool>, key: &str) -> Result<()> {
    match result {
        Ok(true) => Ok(()),
        Ok(false) => {
            log::error!("S3 upload failed for {}: upload reported failure", key);
            bail!("S3 upload failed for {}: upload reported failure", key);
        }
        Err(err) => {
            log::error!("S3 upload failed for {}: {}", key, err);
            Err(err)
        }
    }
}

fn container_backup_s3_key(parent_directory: &str, current_date: &str, sub_directory: &str) -> String {
    format!(
        "{}/{}/{}/{}.tar",
        parent_directory, current_date, sub_directory, sub_directory
    )
}

fn aggregate_backup_failures(failures: Vec<String>) -> Result<()> {
    if failures.is_empty() {
        Ok(())
    } else {
        bail!(
            "Backup failed for {} service(s): {}",
            failures.len(),
            failures.join("; ")
        )
    }
}

pub async fn backup_containers(backup_services: Vec<String>) -> Result<()> {
    let nodes = config::stack_read(|s| s.nodes.clone()).await;

    let mut containers: Vec<(String, String, String)> = Vec::new();

    log::info!("About to start get backup containers");

    for node in nodes.iter() {
        let node_name = node.name();
        let hostname = domain(&node_name);
        match node.as_internal() {
            Ok(img) => {
                if backup_services.contains(&node_name) {
                    containers.push((hostname.clone(), img.repo().root_volume, node_name.clone()))
                }
            }
            Err(_) => (),
        }
    }

    log::info!("Containers to be backed up: {:?}", containers);

    download_and_zip_from_container(containers).await?;

    Ok(())
}

pub async fn download_and_zip_from_container(
    containers: Vec<(String, String, String)>,
) -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    let swarm_number = getenv("SWARM_NUMBER")?;
    let parent_directory = format!("swarm{}", swarm_number);
    let current_date = Local::now().format("%Y-%m-%d").to_string();

    let mut failures: Vec<String> = Vec::new();
    for (container_id, volume_path, sub_directory) in containers {
        let key = container_backup_s3_key(&parent_directory, &current_date, &sub_directory);
        match stream_container_volume_to_s3(&docker, &container_id, &volume_path, &key).await {
            Ok(()) => {
                log::info!(
                    "Volume from container {} streamed to s3://{}/{}",
                    container_id,
                    bucket_name(),
                    key
                );
            }
            Err(e) => {
                log::error!("Backup failed for service {}: {:?}", sub_directory, e);
                failures.push(format!("{}: {}", sub_directory, e));
            }
        }
    }

    aggregate_backup_failures(failures)
}

async fn stream_container_volume_to_s3(
    docker: &Docker,
    container_id: &str,
    volume_path: &str,
    key: &str,
) -> Result<()> {
    let options = DownloadFromContainerOptions { path: volume_path };
    let stream = docker.download_from_container(container_id, Some(options));
    let body_with_io_error =
        stream.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(body_with_io_error);
    futures::pin_mut!(body_reader);
    let result = upload_reader_to_s3_multi(&bucket_name(), &mut body_reader, key).await;
    interpret_s3_upload(result, key)
}

pub fn zip_directory(src_dir: &str, zip_file: &str) -> Result<()> {
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

async fn s3_client_from_env() -> Option<Client> {
    let region = match getenv("AWS_REGION") {
        Ok(value) => value,
        Err(_msg) => {
            log::error!("AWS_REGION is not provided in environment variable");
            return None;
        }
    };

    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    Some(Client::new(&config))
}

async fn abort_multipart(client: &Client, bucket: &str, key: &str, upload_id: &str) {
    if let Err(e) = client
        .abort_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
    {
        log::error!("Failed to abort multipart upload for {}: {:?}", key, e);
    }
}

async fn upload_source_file_size(path: &Path, file_path: &str) -> Result<u64> {
    let meta = tokio::fs::metadata(path).await.with_context(|| {
        format!(
            "unable to find file to upload in this path: {}",
            file_path
        )
    })?;
    Ok(meta.len())
}

async fn upload_part_bytes(
    client: &Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    part_number: i32,
    data: &[u8],
) -> Result<CompletedPart> {
    let stream = ByteStream::from(Bytes::copy_from_slice(data));
    let upload_part_res = client
        .upload_part()
        .key(key)
        .bucket(bucket)
        .upload_id(upload_id)
        .body(stream)
        .part_number(part_number)
        .send()
        .await
        .with_context(|| format!("uploading S3 part {} for {}", part_number, key))?;
    Ok(CompletedPart::builder()
        .e_tag(upload_part_res.e_tag.unwrap_or_default())
        .part_number(part_number)
        .build())
}

async fn complete_multipart(
    client: &Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    upload_parts: Vec<CompletedPart>,
) -> Result<bool> {
    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();

    match client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await
    {
        Ok(_) => Ok(true),
        Err(err) => {
            log::error!("Error completing multipart: {:?}", err);
            abort_multipart(client, bucket, key, upload_id).await;
            Ok(false)
        }
    }
}

async fn start_multipart(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<Option<String>> {
    let result = client
        .create_multipart_upload()
        .bucket(bucket)
        .key(key)
        .send()
        .await;

    let multipart_upload_res = match result {
        Ok(response) => response,
        Err(err) => {
            log::error!("Error creating multipart: {:?}", err);
            return Ok(None);
        }
    };

    match multipart_upload_res.upload_id() {
        Some(id) => Ok(Some(id.to_string())),
        None => {
            log::error!("Upload ID not found");
            Ok(None)
        }
    }
}

/// Stream an `AsyncRead` body into S3 multipart without writing it to disk.
async fn upload_reader_to_s3_multi<R>(bucket: &str, reader: &mut R, key: &str) -> Result<bool>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
{
    let chunk_size = S3_MULTIPART_CHUNK_SIZE as usize;
    let Some(client) = s3_client_from_env().await else {
        return Ok(false);
    };
    let Some(upload_id) = start_multipart(&client, bucket, key).await? else {
        return Ok(false);
    };

    let mut upload_parts: Vec<CompletedPart> = Vec::new();
    let mut buf = vec![0u8; chunk_size];
    let mut filled = 0usize;
    let mut part_number: i32 = 1;
    let mut total: u64 = 0;

    loop {
        let n = match reader.read(&mut buf[filled..]).await {
            Ok(n) => n,
            Err(e) => {
                abort_multipart(&client, bucket, key, &upload_id).await;
                return Err(e.into());
            }
        };
        if n == 0 {
            break;
        }
        filled += n;
        total += n as u64;
        if filled < chunk_size {
            continue;
        }
        if (part_number as u64) > S3_MULTIPART_MAX_CHUNKS {
            log::error!("Too many chunks! Try increasing your chunk size.");
            abort_multipart(&client, bucket, key, &upload_id).await;
            return Ok(false);
        }
        match upload_part_bytes(&client, bucket, key, &upload_id, part_number, &buf[..filled]).await
        {
            Ok(part) => upload_parts.push(part),
            Err(e) => {
                log::error!("Error uploading part: {:?}", e);
                abort_multipart(&client, bucket, key, &upload_id).await;
                return Ok(false);
            }
        }
        log::info!(
            "S3 upload: {} part {} ({}MB so far)",
            key,
            part_number,
            total / (1024 * 1024)
        );
        part_number += 1;
        filled = 0;
    }

    if total == 0 {
        log::error!("Invalid file, file size is 0");
        abort_multipart(&client, bucket, key, &upload_id).await;
        return Ok(false);
    }

    if filled > 0 {
        if (part_number as u64) > S3_MULTIPART_MAX_CHUNKS {
            log::error!("Too many chunks! Try increasing your chunk size.");
            abort_multipart(&client, bucket, key, &upload_id).await;
            return Ok(false);
        }
        match upload_part_bytes(&client, bucket, key, &upload_id, part_number, &buf[..filled]).await
        {
            Ok(part) => upload_parts.push(part),
            Err(e) => {
                log::error!("Error uploading part: {:?}", e);
                abort_multipart(&client, bucket, key, &upload_id).await;
                return Ok(false);
            }
        }
        log::info!(
            "S3 upload: {} part {} ({}MB total)",
            key,
            part_number,
            total / (1024 * 1024)
        );
    }

    complete_multipart(&client, bucket, key, &upload_id, upload_parts).await
}

async fn upload_to_s3_multi(bucket: &str, file_path: &str, key: &str) -> Result<bool> {
    let Some(client) = s3_client_from_env().await else {
        return Ok(false);
    };
    let Some(upload_id) = start_multipart(&client, bucket, key).await? else {
        return Ok(false);
    };

    let path = Path::new(&file_path);
    let file_size = match upload_source_file_size(path, file_path).await {
        Ok(size) => size,
        Err(e) => {
            abort_multipart(&client, bucket, key, &upload_id).await;
            return Err(e);
        }
    };

    let mut chunk_count = (file_size / S3_MULTIPART_CHUNK_SIZE) + 1;
    let mut size_of_last_chunk = file_size % S3_MULTIPART_CHUNK_SIZE;
    if size_of_last_chunk == 0 {
        size_of_last_chunk = S3_MULTIPART_CHUNK_SIZE;
        chunk_count -= 1;
    }

    if file_size == 0 {
        log::error!("Invalid file, file size is 0");
        abort_multipart(&client, bucket, key, &upload_id).await;
        return Ok(false);
    }
    if chunk_count > S3_MULTIPART_MAX_CHUNKS {
        log::error!("Too many chunks! Try increasing your chunk size.");
        abort_multipart(&client, bucket, key, &upload_id).await;
        return Ok(false);
    }

    let file_size_mb = file_size / (1024 * 1024);
    log::info!(
        "S3 upload: {} ({}MB, {} chunks)",
        key,
        file_size_mb,
        chunk_count
    );

    let mut upload_parts: Vec<CompletedPart> = Vec::new();

    for chunk_index in 0..chunk_count {
        let this_chunk = if chunk_count - 1 == chunk_index {
            size_of_last_chunk
        } else {
            S3_MULTIPART_CHUNK_SIZE
        };
        let stream = ByteStream::read_from()
            .path(path)
            .offset(chunk_index * S3_MULTIPART_CHUNK_SIZE)
            .length(Length::Exact(this_chunk))
            .build()
            .await?;

        let part_number = (chunk_index as i32) + 1;

        let upload_part_res = match client
            .upload_part()
            .key(key)
            .bucket(bucket)
            .upload_id(&upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                log::error!("Error uploading part: {:?}", e);
                abort_multipart(&client, bucket, key, &upload_id).await;
                return Ok(false);
            }
        };
        let progress = ((chunk_index + 1) as f64 / chunk_count as f64) * 100.0;
        log::info!(
            "S3 upload: {}/{} chunks ({:.0}%)",
            chunk_index + 1,
            chunk_count,
            progress
        );
        upload_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part_res.e_tag.unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
    }

    complete_multipart(&client, bucket, key, &upload_id, upload_parts).await
}

pub async fn delete_old_backups(bucket: &str, retention_days: i64) -> Result<()> {
    let swarm_number = getenv("SWARM_NUMBER")?;
    let prefix = format!("swarm{}", swarm_number);
    delete_old_backups_with_prefix(bucket, retention_days, &prefix).await
}

async fn delete_old_backups_with_prefix(
    bucket: &str,
    retention_days: i64,
    prefix: &str,
) -> Result<()> {
    let region = getenv("AWS_REGION")?;

    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    let object_prefix = format!("{}/", prefix);

    let resp = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(object_prefix)
        .send()
        .await?;

    let objects = resp.contents();

    if objects.len() > 12 {
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

pub async fn backup_and_delete_volumes_cron(backup_services: Vec<String>) -> Result<JobScheduler> {
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
                if let Err(e) = backup_containers(backup_services.clone()).await {
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

struct BackupFileEntry {
    name: String,
    file_path: String,
    cron: String,
}

fn parse_backup_file_entry(entry: &str) -> Option<BackupFileEntry> {
    let parts: Vec<&str> = entry.splitn(3, ' ').collect();
    if parts.len() < 2 {
        log::error!("Invalid backup_files entry: {}", entry);
        return None;
    }
    let name = parts[0].to_string();
    let file_path = parts[1].to_string();
    let cron = if parts.len() >= 3 {
        parts[2].to_string()
    } else {
        "@daily".to_string()
    };
    Some(BackupFileEntry {
        name,
        file_path,
        cron,
    })
}

async fn finish_single_file_backup(
    upload_result: Result<bool>,
    key: &str,
    backup_path: &str,
) -> Result<()> {
    let outcome = interpret_s3_upload(upload_result, key);
    let _ = tokio::fs::remove_file(backup_path).await;
    outcome
}

async fn backup_single_file(entry: &BackupFileEntry) -> Result<()> {
    let volume_name = domain(&entry.name);
    let src_path = format!(
        "/var/lib/docker/volumes/{}/_data/{}",
        volume_name, &entry.file_path
    );
    let backup_path = format!("{}.backup", &src_path);

    log::info!("backup_file: copying {} to {}", &src_path, &backup_path);
    let bytes_copied = tokio::fs::copy(&src_path, &backup_path).await?;
    let mb_copied = bytes_copied / (1024 * 1024);
    log::info!("backup_file: copy complete ({}MB)", mb_copied);

    let current_date = Local::now().format("%Y-%m-%d").to_string();
    let parent_directory = swarm_prefix_from_host()?;
    let file_name = Path::new(&entry.file_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("backup");
    let s3_key = format!(
        "{}/{}/{}/{}",
        &parent_directory, &current_date, &entry.name, file_name
    );

    log::info!(
        "backup_file: uploading {} to s3://{}",
        &backup_path,
        &s3_key
    );
    let upload_result = upload_to_s3_multi(&bucket_name(), &backup_path, &s3_key).await;
    finish_single_file_backup(upload_result, &s3_key, &backup_path).await?;

    log::info!(
        "backup_file: completed backup of {} from {}",
        &entry.file_path,
        &entry.name
    );
    Ok(())
}

pub async fn backup_files_cron(backup_files: Vec<String>) -> Result<Vec<JobScheduler>> {
    log::info!("backup_files: setting up cron schedules");
    let mut schedulers = Vec::new();

    for entry_str in backup_files {
        let entry = match parse_backup_file_entry(&entry_str) {
            Some(e) => e,
            None => continue,
        };

        let cron_expr = entry.cron.clone();
        let name = entry.name.clone();
        let file_path = entry.file_path.clone();

        log::info!(
            "backup_files: scheduling {} {} with cron '{}'",
            &name,
            &file_path,
            &cron_expr
        );

        let sched = JobScheduler::new().await?;
        let trigger_flag = std::sync::Arc::new(AtomicBool::new(false));
        let trigger_for_cron = trigger_flag.clone();
        let trigger_for_loop = trigger_flag.clone();

        sched
            .add(Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
                let flag = trigger_for_cron.clone();
                Box::pin(async move {
                    flag.store(true, Ordering::Relaxed);
                })
            })?)
            .await?;

        sched.start().await?;

        tokio::spawn(async move {
            let entry = BackupFileEntry {
                name,
                file_path,
                cron: cron_expr,
            };
            loop {
                if trigger_for_loop.load(Ordering::Relaxed) {
                    if let Err(e) = backup_single_file(&entry).await {
                        log::error!(
                            "backup_file error for {} {}: {:?}",
                            &entry.name,
                            &entry.file_path,
                            e
                        );
                    }
                    if let Ok(prefix) = swarm_prefix_from_host() {
                        if let Err(e) = delete_old_backups_with_prefix(
                            &bucket_name(),
                            backup_retention_days(),
                            &prefix,
                        )
                        .await
                        {
                            log::error!("Delete old backup_file backups: {:?}", e);
                        }
                    }
                    trigger_for_loop.store(false, Ordering::Relaxed);
                }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        });

        schedulers.push(sched);
    }

    Ok(schedulers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "sphinx-swarm-backup-{}-{}-{}",
            std::process::id(),
            nanos,
            label
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn test_temp_file(label: &str) -> PathBuf {
        let dir = test_temp_dir(label);
        let path = dir.join("file.tar");
        fs::write(&path, b"data").unwrap();
        path
    }

    fn function_body<'a>(src: &'a str, signature: &str, next_markers: &[&str]) -> &'a str {
        let fn_start = src.find(signature).unwrap_or_else(|| panic!("{}", signature));
        let rest = &src[fn_start..];
        let mut fn_end = rest.len();
        for marker in next_markers {
            if let Some(i) = rest[1..].find(marker) {
                fn_end = fn_end.min(i + 1);
            }
        }
        &rest[..fn_end]
    }

    #[test]
    fn s3_key_layout_unchanged() {
        let key = container_backup_s3_key("swarm7", "2024-01-01", "neo4j");
        assert_eq!(key, "swarm7/2024-01-01/neo4j/neo4j.tar");
        assert!(!key.starts_with("/vol/"));
    }

    #[test]
    fn staging_dir_matcher_accepts_date_suffix() {
        assert!(is_backup_staging_dir_name("swarm7_2024-01-01"));
        assert!(is_backup_staging_dir_name("swarmprod_2024-01-01"));
    }

    #[test]
    fn staging_dir_matcher_rejects_non_staging() {
        assert!(!is_backup_staging_dir_name("swarm7"));
        assert!(!is_backup_staging_dir_name("vol"));
        assert!(!is_backup_staging_dir_name("swarm7_notadate"));
        assert!(!is_backup_staging_dir_name("arbitrary"));
        assert!(!is_backup_staging_dir_name("swarm7_2024-13-01"));
        assert!(!is_backup_staging_dir_name("config.json"));
    }

    #[test]
    fn sweep_removes_only_matching_dirs() {
        let root = test_temp_dir("sweep");
        fs::create_dir_all(root.join("swarm7_2024-01-01")).unwrap();
        fs::create_dir_all(root.join("swarmprod_2024-01-01")).unwrap();
        fs::create_dir_all(root.join("swarm7")).unwrap();
        fs::create_dir_all(root.join("vol")).unwrap();
        fs::create_dir_all(root.join("swarm7_notadate")).unwrap();
        fs::write(root.join("keep.txt"), b"x").unwrap();

        let removed = sweep_stale_backup_staging_dirs_in(&[root.to_str().unwrap()]);
        assert_eq!(removed.len(), 2);
        assert!(!root.join("swarm7_2024-01-01").exists());
        assert!(!root.join("swarmprod_2024-01-01").exists());
        assert!(root.join("swarm7").exists());
        assert!(root.join("vol").exists());
        assert!(root.join("swarm7_notadate").exists());
        assert!(root.join("keep.txt").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sweep_uses_spawn_blocking() {
        let src = include_str!("backup.rs");
        let body = function_body(
            src,
            "pub async fn sweep_stale_backup_staging_dirs",
            &["\nfn sweep_stale_backup_staging_dirs_in"],
        );
        assert!(body.contains("spawn_blocking"));
    }

    #[test]
    fn cron_does_not_sweep_per_tick_or_at_setup() {
        let src = include_str!("backup.rs");
        let body = function_body(
            src,
            "pub async fn backup_and_delete_volumes_cron",
            &["\nstruct BackupFileEntry"],
        );
        assert!(!body.contains("sweep_stale_backup_staging_dirs"));
    }

    #[test]
    fn stack_sweeps_unconditionally_before_backup_gate() {
        let src = include_str!("bin/stack/mod.rs");
        let sweep_idx = src
            .find("sweep_stale_backup_staging_dirs")
            .expect("stack must call sweep at startup");
        let backup_idx = src
            .find("if let Some(backup_services)")
            .expect("backup_services gate");
        assert!(
            sweep_idx < backup_idx,
            "sweep must run even when backup is disabled"
        );
    }

    #[test]
    fn container_backup_streams_without_disk_staging() {
        let src = include_str!("backup.rs");
        let body = function_body(
            src,
            "pub async fn download_and_zip_from_container",
            &["\nasync fn stream_container_volume_to_s3"],
        );
        assert!(!body.contains("create_dir_all"));
        assert!(!body.contains("/vol/swarm"));
        assert!(body.contains("stream_container_volume_to_s3"));
        assert!(body.contains("aggregate_backup_failures"));
    }

    #[test]
    fn one_failed_service_does_not_drop_siblings() {
        let err = aggregate_backup_failures(vec![
            "neo4j: S3 upload failed for swarm7/k: upload reported failure".to_string(),
        ])
        .unwrap_err()
        .to_string();
        assert!(err.contains("1 service(s)"));
        assert!(err.contains("neo4j"));
        assert!(aggregate_backup_failures(vec![]).is_ok());
    }

    #[test]
    fn download_loop_collects_failures_instead_of_bailing() {
        let src = include_str!("backup.rs");
        let body = function_body(
            src,
            "pub async fn download_and_zip_from_container",
            &["\nasync fn stream_container_volume_to_s3"],
        );
        assert!(body.contains("failures.push"));
        let stream_call = body
            .split("stream_container_volume_to_s3")
            .nth(1)
            .unwrap_or("");
        assert!(
            !stream_call.contains(".await?;"),
            "a failed service must not abort remaining services"
        );
    }

    #[test]
    fn interpret_s3_upload_ok_true() {
        assert!(interpret_s3_upload(Ok(true), "swarm7/k.tar").is_ok());
    }

    #[test]
    fn interpret_s3_upload_ok_false_returns_err() {
        let result = interpret_s3_upload(Ok(false), "mykey");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("S3 upload failed for mykey: upload reported failure"));
    }

    #[test]
    fn interpret_s3_upload_err_returns_err() {
        let result = interpret_s3_upload(Err(anyhow!("network down")), "mykey");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("network down"));
        assert!(!msg.contains("We are getting somewhere"));
    }

    #[tokio::test]
    async fn backup_single_file_ok_true_cleans_temp() {
        let path = test_temp_file("single-ok");
        let path_str = path.to_str().unwrap().to_string();
        let result = finish_single_file_backup(Ok(true), "k", &path_str).await;
        assert!(result.is_ok());
        assert!(!path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[tokio::test]
    async fn backup_single_file_ok_false_propagates_err() {
        let path = test_temp_file("single-false");
        let path_str = path.to_str().unwrap().to_string();
        let result = finish_single_file_backup(Ok(false), "filekey", &path_str).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("S3 upload failed for filekey: upload reported failure"));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[tokio::test]
    async fn backup_single_file_err_propagates_err() {
        let path = test_temp_file("single-err");
        let path_str = path.to_str().unwrap().to_string();
        let result = finish_single_file_backup(Err(anyhow!("timeout")), "filekey", &path_str).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout"));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[tokio::test]
    async fn upload_source_file_size_returns_err_not_panic() {
        let result = upload_source_file_size(
            Path::new("/no/such/backup/file.tar"),
            "/no/such/backup/file.tar",
        )
        .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unable to find file to upload in this path"));
    }
}
