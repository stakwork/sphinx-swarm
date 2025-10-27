use anyhow::Result;
use sphinx_swarm::utils::getenv;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_cron_scheduler::Job;
use tokio_cron_scheduler::JobScheduler;

use crate::service::ssl_cert::handle_renew_cert::handle_renew_ssl_cert;

pub static SSL_CERT_RENEWAL: AtomicBool = AtomicBool::new(false);

pub async fn ssl_cert_renewal_cron() -> Result<JobScheduler> {
    log::info!(":SSL CERT RENEWAL CRON");
    let sched = JobScheduler::new().await?;

    let cert_renewal_cron_time =
        getenv("SSL_CERT_RENEWAL_CRON_TIME").unwrap_or("@daily".to_string());
    log::info!("SSL Cert Renewal Cron Time: {}", cert_renewal_cron_time);

    // this runs very 5 mins
    sched
        .add(Job::new_async(
            cert_renewal_cron_time.as_str(),
            |_uuid, _l| {
                Box::pin(async move {
                    if !SSL_CERT_RENEWAL.load(Ordering::Relaxed) {
                        SSL_CERT_RENEWAL.store(true, Ordering::Relaxed);
                    }
                })
            },
        )?)
        .await?;

    sched.start().await?;

    tokio::spawn(async move {
        loop {
            let go = SSL_CERT_RENEWAL.load(Ordering::Relaxed);
            if go {
                if let Err(e) = handle_renew_ssl_cert().await {
                    log::error!("{:?}", e);
                }
                SSL_CERT_RENEWAL.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}
