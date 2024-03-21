use chrono::Local;
use rocket::tokio::fs;
use rusoto_core::Region;
use rusoto_s3::{
    DeleteObjectsRequest, ListObjectsV2Request, ObjectIdentifier, PutObjectRequest, S3Client,
};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub async fn backup_to_s3(
    proxy_path: &str,
    relay_path: &str,
    bucket: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_local_defaults()?;

    // Download and zip proxy volume
    let proxy_zip_data =
        download_and_zip_from_container(&docker, "proxy_container_id", proxy_path).await?;
    let proxy_zip_file_name = format!("proxy_data_{}_{}.zip", Local::now().format("%Y%m%d_%H%M%S"));
    let proxy_zip_file = PathBuf::from(&proxy_zip_file_name);
    fs::write(&proxy_zip_file, proxy_zip_data).await?;

    // Download and zip relay volume
    let relay_zip_data =
        download_and_zip_from_container(&docker, "relay_container_id", relay_path).await?;
    let relay_zip_file_name = format!("relay_data_{}_{}.zip", Local::now().format("%Y%m%d_%H%M%S"));
    let relay_zip_file = PathBuf::from(&relay_zip_file_name);
    fs::write(&relay_zip_file, relay_zip_data).await?;

    // Upload proxy zip file to S3
    upload_to_s3(&bucket, &proxy_zip_file_name, proxy_zip_file).await?;

    // Upload relay zip file to S3
    upload_to_s3(&bucket, &relay_zip_file_name, relay_zip_file).await?;

    Ok(())
}

async fn download_and_zip_from_container(
    docker: &Docker,
    id: &str,
    path: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let tar_stream = docker
        .download_from_container(id, Some(DownloadFromContainerOptions { path: path.into() }))
        .await?;
    let mut zip_data = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));

    let mut tar = tar::Archive::new(tar_stream);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;

        zip.start_file(
            path.to_string_lossy().into_owned(),
            zip::write::FileOptions::default(),
        )?;
        zip.write_all(&data)?;
    }

    zip.finish()?;
    Ok(zip_data)
}

async fn zip_data(data: Vec<u8>, name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let current_time = Local::now();
    let zip_file_name = format!("{}_{}.zip", name, current_time.format("%Y%m%d_%H%M%S"));

    let mut zip_file = zip::ZipWriter::new(File::create(&zip_file_name)?);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip_file.start_file(name, options)?;
    zip_file.write_all(&data)?;

    Ok(PathBuf::from(zip_file_name))
}

// Uploads the zip file to S3
async fn upload_to_s3(
    bucket: &str,
    zip_file_name: &str,
    zip_file: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let s3_client = S3Client::new(Region::default());

    let file = File::open(&zip_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let key = zip_file_name;

    let request = PutObjectRequest {
        bucket: bucket.to_owned(),
        key: key.to_owned(),
        body: Some(buffer.into()),
        ..Default::default()
    };

    s3_client.put_object(request).await?;
    Ok(())
}

// Deletes old backups from the S3 bucket
async fn delete_old_backups(
    bucket: &str,
    prefix: &str,
    retention_days: i64,
) -> Result<(), Box<dyn Error>> {
    let s3_client = S3Client::new(Region::default());

    let current_time = Local::now();
    let cutoff_time = current_time - chrono::Duration::days(retention_days);

    let list_request = ListObjectsV2Request {
        bucket: bucket.to_owned(),
        prefix: Some(prefix.to_owned()),
        ..Default::default()
    };

    let objects = s3_client.list_objects_v2(list_request).await?;

    let objects_to_delete: Vec<ObjectIdentifier> = objects
        .contents
        .unwrap_or_default()
        .iter()
        .filter(|obj| {
            obj.last_modified
                .as_ref()
                .map_or(false, |lm| lm < &cutoff_time)
        })
        .map(|obj| ObjectIdentifier {
            key: obj.key.clone(),
            ..Default::default()
        })
        .collect();

    if !objects_to_delete.is_empty() {
        let delete_request = DeleteObjectsRequest {
            bucket: bucket.to_owned(),
            delete: rusoto_s3::Delete {
                objects: objects_to_delete,
                ..Default::default()
            },
            ..Default::default()
        };

        s3_client.delete_objects(delete_request).await?;
    }

    Ok(())
}

async fn test_backup() -> Result<(), Box<dyn Error>> {
    let bucket = "your-bucket-name";
    let zip_data = vec![1, 2, 3, 4, 5]; // Example data to be zipped

    let zip_file = zip_data(zip_data, "backup_data").await?;
    println!("Zip file created: {:?}", zip_file);

    upload_to_s3(bucket, "backup_data.zip", zip_file.clone()).await?;
    println!("Zip file uploaded to S3");

    delete_old_backups(bucket, "backup_prefix", 30).await?;
    println!("Old backups deleted from S3");

    Ok(())
}
