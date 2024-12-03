use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{anyhow, Error, Result};
use bollard::Docker;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{config::STATE, dock::restart_node_container, utils::getenv};

pub static RESTART_SERVICES: AtomicBool = AtomicBool::new(false);

pub async fn auto_restart_cron(
    proj: String,
    docker: Docker,
    auto_restart_services: Vec<String>,
) -> Result<JobScheduler> {
    log::info!("Auto Restart Services");
    let sched = JobScheduler::new().await?;

    let cron_time = match getenv("AUTO_RESTART_CRON_TIME") {
        Ok(env) => env,
        Err(_) => "@daily".to_string(),
    };

    sched
        .add(Job::new_async(cron_time.as_str(), |_uuid, _l| {
            Box::pin(async move {
                if !RESTART_SERVICES.load(Ordering::Relaxed) {
                    RESTART_SERVICES.store(true, Ordering::Relaxed);
                }
            })
        })?)
        .await?;

    sched.start().await?;

    tokio::spawn(async move {
        loop {
            let go = RESTART_SERVICES.load(Ordering::Relaxed);
            if go {
                if let Err(e) =
                    auto_restart_services_handler(&proj, &docker, auto_restart_services.clone())
                        .await
                {
                    log::error!("Error auto restarting services: {:?}", e);
                }

                RESTART_SERVICES.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}

async fn auto_restart_services_handler(
    proj: &str,
    docker: &Docker,
    auto_restart_services: Vec<String>,
) -> Result<(), Error> {
    let mut state = STATE.lock().await;
    let mut err_vec: Vec<String> = Vec::new();
    for service in auto_restart_services {
        log::info!("About to auto restart {}", service);
        match restart_node_container(docker, &service, &mut state, proj).await {
            Ok(()) => {}
            Err(err) => err_vec.push(format!("{}: {}", service, err.to_string())),
        }
    }
    if err_vec.is_empty() {
        return Ok(());
    }

    Err(anyhow!(err_vec.join("\n")))
}
