use crate::service::public_ip::handle_check_public_ip_via_cron;
use crate::utils::getenv;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_cron_scheduler::{Job, JobScheduler};

pub static CHECK_PUBLIC_IP: AtomicBool = AtomicBool::new(false);

pub async fn check_public_ip() -> Result<JobScheduler> {
    log::info!(":Check Public IP!!");
    let sched = JobScheduler::new().await?;

    let cron_time = match getenv("CHECK_PUBLIC_IP_CRON") {
        Ok(env) => env,
        Err(_) => "0 */5 * * * *".to_string(),
    };

    sched
        .add(Job::new_async(cron_time.as_str(), |_uuid, _l| {
            Box::pin(async move {
                if !CHECK_PUBLIC_IP.load(Ordering::Relaxed) {
                    CHECK_PUBLIC_IP.store(true, Ordering::Relaxed);
                }
            })
        })?)
        .await?;

    sched.start().await?;

    tokio::spawn(async move {
        loop {
            let go = CHECK_PUBLIC_IP.load(Ordering::Relaxed);
            if go {
                if let Err(e) = handle_check_public_ip_via_cron().await {
                    log::error!("{:?}", e);
                }

                CHECK_PUBLIC_IP.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}
