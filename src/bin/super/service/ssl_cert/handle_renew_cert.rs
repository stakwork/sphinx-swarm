use anyhow::{anyhow, Error, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::Client;
use chrono::{DateTime, Utc};
use sphinx_swarm::utils::getenv;

pub async fn handle_renew_ssl_cert() -> Result<()> {
    // check that how many days remaining for cert to expire
    let days_left = get_cert_days_left().await?;
    // check if the days is less than 15
    if days_left > 15 {
        log::info!("We have {} until cert expity", days_left);
        return Ok(());
    }
    // if less then 15 renew cert and upload a new one
    // if greater than 15 we just log how much days remain and proceed
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

    let resp = client.head_object().bucket(bucket).key(key).send().await?;

    // get last date modified
    if let None = resp.last_modified() {
        return Err(anyhow!("Unable to get last date modified from s3 bucket"));
    }

    let last_modified = resp.last_modified().unwrap();
    let now = Utc::now();

    let last_modified_chrono = DateTime::<Utc>::from_timestamp(last_modified.secs(), 0)
        .ok_or_else(|| anyhow!("Failed to convert last_modified to chrono::DateTime"))?;

    let diff = now.signed_duration_since(last_modified_chrono);

    let days_diff = diff.num_days();

    Ok(90 - days_diff)
}
