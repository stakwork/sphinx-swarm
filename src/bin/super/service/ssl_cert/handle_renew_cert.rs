use std::io::{Cursor, Read};
use std::time::Duration;

use anyhow::{anyhow, Error, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::Client;
use chrono::{DateTime, Utc};
use sphinx_swarm::utils::getenv;
use x509_parser::pem::parse_x509_pem;

use crate::{
    cmd::{SllCertExpiryDaysResponse, SuperRestarterResponse, SuperSwarmResponse},
    service::update_super_admin::UpdateSuperAdminBody,
};

pub async fn handle_renew_ssl_cert() -> Result<()> {
    // check that how many days remaining for cert to expire
    let days_left = get_cert_days_left().await?;
    // check if the days is less than 15
    if days_left > 15 {
        log::info!("We have {} until cert expity", days_left);
        return Ok(());
    }
    // if less then 15 renew cert and upload a new one
    let renew_cert_res = renew_cert().await?;

    log::info!("Renew cert response: {:#?}", renew_cert_res);

    if !renew_cert_res.ok {
        return Err(anyhow!(
            "Failed to renew cert: {}",
            renew_cert_res.error.unwrap_or_default()
        ));
    }

    let upload_cert_res = upload_cert_to_s3().await?;

    log::info!("Upload cert response: {:#?}", upload_cert_res);

    if !upload_cert_res.ok {
        return Err(anyhow!(
            "Failed to upload cert to s3: {}",
            upload_cert_res.error.unwrap_or_default()
        ));
    }
    Ok(())
}

pub async fn get_cert_days_left() -> Result<i64, Error> {
    // get cert from s3
    let region = getenv("AWS_REGION")?;
    let bucket = getenv("CERT_BUCKET")?;
    let key = "data.zip"; // we can move this to env at will

    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);

    let resp = client.get_object().bucket(bucket).key(key).send().await?;
    let zip_bytes = resp.body.collect().await?.into_bytes();

    // read the expiry from the cert itself, not the upload date
    let not_after = cert_not_after(&zip_bytes)?;

    let diff = not_after.signed_duration_since(Utc::now());

    Ok(diff.num_days())
}

fn cert_not_after(zip_bytes: &[u8]) -> Result<DateTime<Utc>, Error> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))?;

    let crt_name = archive
        .file_names()
        .find(|name| name.ends_with("sphinx.chat.crt"))
        .map(|name| name.to_string())
        .ok_or_else(|| anyhow!("sphinx.chat.crt not found in data.zip"))?;

    let mut pem_bytes = Vec::new();
    archive.by_name(&crt_name)?.read_to_end(&mut pem_bytes)?;

    // fullchain.pem: the first cert is the leaf
    let (_, pem) =
        parse_x509_pem(&pem_bytes).map_err(|e| anyhow!("Failed to parse cert pem: {:?}", e))?;
    let cert = pem
        .parse_x509()
        .map_err(|e| anyhow!("Failed to parse x509 cert: {:?}", e))?;

    DateTime::<Utc>::from_timestamp(cert.validity().not_after.timestamp(), 0)
        .ok_or_else(|| anyhow!("Failed to convert cert expiry to chrono::DateTime"))
}

pub async fn renew_cert() -> Result<SuperRestarterResponse, Error> {
    // call restart script to renew cert
    let password = std::env::var("SUPER_ADMIN_UPDATER_PASSWORD").unwrap_or(String::new());

    let client = reqwest::Client::builder()
        // certbot's dns challenge can take well over 20s
        .timeout(Duration::from_secs(180))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build renew cert reqwest client");

    let route = format!("http://172.17.0.1:3003/renew-cert");

    let body = UpdateSuperAdminBody {
        password: password.to_string(),
    };

    let response = client.post(route.as_str()).json(&body).send().await?;

    let data: SuperRestarterResponse = response.json().await?;

    Ok(data)
}

pub async fn upload_cert_to_s3() -> Result<SuperRestarterResponse, Error> {
    // call restart script to renew cert
    let password = std::env::var("SUPER_ADMIN_UPDATER_PASSWORD").unwrap_or(String::new());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build upload cert reqwest client");

    let route = format!("http://172.17.0.1:3003/upload-cert");

    let body = UpdateSuperAdminBody {
        password: password.to_string(),
    };

    let response = client.post(route.as_str()).json(&body).send().await?;

    let data: SuperRestarterResponse = response.json().await?;

    Ok(data)
}

pub async fn handle_get_ssl_cert_expiry() -> SuperSwarmResponse {
    let res = match get_cert_days_left().await {
        Ok(day) => SllCertExpiryDaysResponse { day },
        Err(err) => {
            return SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            }
        }
    };

    match serde_json::to_value(res) {
        Ok(res) => SuperSwarmResponse {
            success: true,
            message: "expiry days gotten successfully".to_string(),
            data: Some(res),
        },
        Err(err) => SuperSwarmResponse {
            success: false,
            message: err.to_string(),
            data: None,
        },
    }
}
