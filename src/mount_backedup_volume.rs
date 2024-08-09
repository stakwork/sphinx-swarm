use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::ByteStream;
use aws_smithy_types::retry::RetryConfig;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tar::Builder;
use tokio::fs::remove_dir_all;
use zip::ZipArchive;

use crate::utils::getenv;

pub async fn download_from_s3(bucket: &str, key: &str) -> Result<(), Box<dyn Error>> {
    let region = getenv("AWS_S3_REGION_NAME")?;

    // Create a region provider chain
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    let client = Client::new(&config);

    match client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(key)
        .send()
        .await
    {
        Ok(list_resp) => {
            log::info!("Lists: {:?}", &list_resp);
            for object in list_resp.contents() {
                let key = object.key().unwrap();
                log::info!("This is the key: {}", key);

                // Download the file
                download_file(&client, bucket, &key, &key).await?;
            }
        }
        Err(err) => {
            log::error!("Get objects error: {}", err)
        }
    }

    Ok(())
}

pub fn unzip_file(zip_path: &str, output_dir: &str) -> Result<(), Box<dyn Error>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(output_dir).join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            // Check if the extracted file is a zip file
            if outpath.extension().and_then(|e| e.to_str()) == Some("zip") {
                // Recursively unzip the nested zip file
                unzip_file(
                    outpath.to_str().unwrap(),
                    outpath.parent().unwrap().to_str().unwrap(),
                )?;
                // Optionally, delete the nested zip file after extraction
                std::fs::remove_file(outpath)?;
            }
        }
    }

    Ok(())
}

pub fn create_tar(data_path: &str) -> Result<String, Box<dyn Error>> {
    let tar_path = format!("{}.tar", data_path);
    let tar_file = File::create(&tar_path)?;

    let mut tar = Builder::new(tar_file);

    tar.append_dir_all(".", data_path)?;
    tar.finish()?;

    Ok(tar_path)
}

pub async fn delete_zip_and_upzipped_files() -> Result<(), Box<dyn std::error::Error>> {
    let backup_link = getenv("BACKUP_KEY")?;
    let unzipped_directory = "unzipped";

    if Path::new(&backup_link).exists() {
        fs::remove_file(&backup_link)?;
    }

    // Check if the directory exists before trying to remove it
    if Path::new(&unzipped_directory).exists() {
        remove_dir_all(unzipped_directory).await?;
    }

    Ok(())
}

async fn download_file(
    client: &Client,
    bucket: &str,
    key: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let resp: GetObjectOutput = client.get_object().bucket(bucket).key(key).send().await?;
    let data: ByteStream = resp.body;

    if let Some(parent_dir) = Path::new(output_path).parent() {
        fs::create_dir_all(parent_dir)?;
    }

    let mut file = File::create(output_path)?;
    let bytes = data.collect().await?.into_bytes();
    file.write_all(&bytes)?;

    Ok(())
}
