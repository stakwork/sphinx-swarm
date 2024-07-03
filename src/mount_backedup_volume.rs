use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::Client;
// use bollard::volume::CreateVolumeOptions;
// use bollard::Docker;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
// use std::time::Duration;
// use tokio::time::sleep;
use zip::ZipArchive;

use crate::utils::getenv;

pub async fn download_from_s3(
    bucket: &str,
    key: &str,
    output_path: &str,
) -> Result<(), Box<dyn Error>> {
    let region = getenv("AWS_S3_REGION_NAME")?;

    // Create a region provider chain
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    let resp = client.get_object().bucket(bucket).key(key).send().await?;

    let mut file = File::create(output_path)?;
    let data = resp.body.collect().await?;
    file.write_all(&data.into_bytes())?;

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
