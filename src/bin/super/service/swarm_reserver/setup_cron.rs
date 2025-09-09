use anyhow::Result;
use sphinx_swarm::utils::getenv;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_cron_scheduler::Job;
use tokio_cron_scheduler::JobScheduler;

use crate::service::swarm_reserver::reserve_swarm::handle_reserve_swarms;

pub static SWARM_RESERVER: AtomicBool = AtomicBool::new(false);

pub async fn swarm_reserver_cron() -> Result<JobScheduler> {
    log::info!(":Swarm Reserver Cron");
    let sched = JobScheduler::new().await?;

    let swarm_reserver_cron_time =
        getenv("SWARM_RESERVER_CRON_TIME").unwrap_or("0 1/5 * * * *".to_string());
    log::info!("Swarm Reserver Cron Time: {}", swarm_reserver_cron_time);

    // this runs very 5 mins
    sched
        .add(Job::new_async(
            swarm_reserver_cron_time.as_str(),
            |_uuid, _l| {
                Box::pin(async move {
                    if !SWARM_RESERVER.load(Ordering::Relaxed) {
                        SWARM_RESERVER.store(true, Ordering::Relaxed);
                    }
                })
            },
        )?)
        .await?;

    sched.start().await?;

    tokio::spawn(async move {
        loop {
            let go = SWARM_RESERVER.load(Ordering::Relaxed);
            if go {
                if let Err(e) = handle_reserve_swarms().await {
                    log::error!("{:?}", e);
                }
                SWARM_RESERVER.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}
