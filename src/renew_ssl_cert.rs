use crate::{
    config,
    conn::swarm::{SwarmRestarterRes, UpdateSslCertSwarmBody},
    utils::{getenv, is_using_port_based_ssl},
};
use anyhow::{anyhow, Error, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_s3::Client;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio_cron_scheduler::{Job, JobScheduler};

pub static CHECK_SSL_CERT: AtomicBool = AtomicBool::new(false);
pub async fn upload_new_ssl_cert_cron() -> Result<JobScheduler> {
    log::info!(":check for new ssl cert");
    let sched = JobScheduler::new().await?;

    sched
        .add(Job::new_async("@daily", |_uuid, _l| {
            Box::pin(async move {
                if !CHECK_SSL_CERT.load(Ordering::Relaxed) {
                    CHECK_SSL_CERT.store(true, Ordering::Relaxed);
                }
            })
        })?)
        .await?;

    sched.start().await?;

    tokio::spawn(async move {
        loop {
            let go = CHECK_SSL_CERT.load(Ordering::Relaxed);
            if go {
                if let Err(e) = handle_update_ssl_cert("stack").await {
                    log::error!("Checking for SSL CERT: {:?}", e);
                }

                CHECK_SSL_CERT.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}

/// Check S3 for updated SSL cert, download if newer, persist timestamp via stack_write.
pub async fn handle_update_ssl_cert(proj: &str) -> Result<(), Error> {
    if !is_using_port_based_ssl() {
        return Err(anyhow!("Current swarm does not support port based ssl"));
    }

    let region = getenv("AWS_REGION")?;
    let bucket = getenv("CERT_BUCKET").unwrap_or("sphinx-swarm-superadmin".to_string());
    let key = "data.zip";

    // 1. Read current timestamp (brief read lock)
    let current_last_modified = config::stack_read(|s| s.ssl_cert_last_modified).await;

    // 2. S3 check (no lock held)
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let aws_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&aws_config);

    let resp = client
        .head_object()
        .bucket(bucket.clone())
        .key(key)
        .send()
        .await?;

    let last_modified = resp
        .last_modified()
        .ok_or_else(|| anyhow!("Unable to get last date modified from s3 bucket"))?
        .secs();

    if current_last_modified == Some(last_modified) {
        return Err(anyhow!("cert is upto date!"));
    }

    // 3. Download + apply cert (no lock held)
    let res = update_ssl_cert(bucket.clone()).await?;

    if let Some(err) = res.error {
        return Err(anyhow!(err));
    }
    if let Some(false) = res.ok {
        return Err(anyhow!(
            "An unexpected error occured while trying to update ssl certificate"
        ));
    }

    // 4. Persist new timestamp (brief write lock + disk save)
    config::stack_write(proj, |s| {
        s.ssl_cert_last_modified = Some(last_modified);
    })
    .await;

    Ok(())
}

pub async fn update_ssl_cert(bucket_name: String) -> Result<SwarmRestarterRes, Error> {
    let password = std::env::var("SWARM_UPDATER_PASSWORD").unwrap_or(String::new());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build update ssl cert reqwest client");

    let route = format!("http://172.17.0.1:3003/update-ssl-cert");

    let body = UpdateSslCertSwarmBody {
        password: password.to_string(),
        cert_bucket_name: bucket_name,
    };
    let response = client.post(route.as_str()).json(&body).send().await?;

    let response_json: SwarmRestarterRes = response.json().await?;

    Ok(response_json)
}
