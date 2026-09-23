use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{anyhow, Error, Result};
use bollard::Docker;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{
    dock::{container_running, restart_node_container_global},
    utils::{domain, getenv},
};

pub static RESTART_SERVICES: AtomicBool = AtomicBool::new(false);

/// How many times a service restart is attempted per cron run before giving up.
const DEFAULT_MAX_ATTEMPTS: u32 = 3;
/// Pause between attempts, so a transient Docker error has time to clear.
const DEFAULT_RETRY_DELAY_SECS: u64 = 30;

pub async fn auto_restart_cron(
    proj: String,
    docker: Docker,
    auto_restart_services: Vec<String>,
) -> Result<JobScheduler> {
    log::info!("Auto Restart Services");
    let sched = JobScheduler::new().await?;

    let cron_time = match getenv("AUTO_RESTART_CRON_TIME") {
        Ok(env) => env,
        Err(_) => "0 0 2 * * *".to_string(),
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

fn max_attempts() -> u32 {
    getenv("AUTO_RESTART_MAX_ATTEMPTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n: &u32| *n >= 1)
        .unwrap_or(DEFAULT_MAX_ATTEMPTS)
}

fn retry_delay_secs() -> u64 {
    getenv("AUTO_RESTART_RETRY_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RETRY_DELAY_SECS)
}

async fn auto_restart_services_handler(
    proj: &str,
    docker: &Docker,
    auto_restart_services: Vec<String>,
) -> Result<(), Error> {
    let mut err_vec: Vec<String> = Vec::new();
    for service in auto_restart_services {
        log::info!("About to auto restart {}", service);
        if let Err(err) = restart_with_retries(docker, &service, proj).await {
            err_vec.push(format!("{}: {}", service, err));
        }
    }
    if err_vec.is_empty() {
        return Ok(());
    }

    Err(anyhow!(err_vec.join("\n")))
}

/// Restarts one service, retrying on failure. A failed `stop_and_remove`
/// (e.g. Docker's "unable to remove filesystem ... directory not empty") used
/// to abort the whole restart and leave the service down until someone
/// noticed. Each attempt runs the full remove + create + start sequence, and
/// the final state of the container is logged so a service that never came
/// back is visible in the log as an error rather than as a missing line.
async fn restart_with_retries(docker: &Docker, service: &str, proj: &str) -> Result<()> {
    let attempts = max_attempts();
    let delay = retry_delay_secs();
    let mut last_err: Option<Error> = None;

    for attempt in 1..=attempts {
        match restart_node_container_global(docker, service, proj).await {
            Ok(()) => {
                if attempt > 1 {
                    log::info!(
                        "auto restart of {} succeeded on attempt {}/{}",
                        service,
                        attempt,
                        attempts
                    );
                }
                return Ok(());
            }
            Err(e) => {
                log::error!(
                    "auto restart of {} failed (attempt {}/{}): {}",
                    service,
                    attempt,
                    attempts,
                    e
                );
                last_err = Some(e);
                if attempt < attempts {
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    }

    let hostname = domain(service);
    match container_running(docker, &hostname).await {
        Ok(true) => log::warn!(
            "{} is running despite {} failed restart attempt(s); it may be the old container",
            hostname,
            attempts
        ),
        Ok(false) => log::error!(
            "{} IS NOT RUNNING after {} restart attempt(s) — manual intervention required",
            hostname,
            attempts
        ),
        Err(e) => log::error!(
            "{} could not be inspected after {} restart attempt(s): {}",
            hostname,
            attempts,
            e
        ),
    }

    Err(last_err.unwrap_or_else(|| anyhow!("restart failed")))
}
