use anyhow::Result;
use rocket::tokio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::state;

pub static SWARM_CHECKER: AtomicBool = AtomicBool::new(false);

pub async fn swarm_checker() -> Result<JobScheduler> {
    log::info!(":Swarm Checker");
    let sched = JobScheduler::new().await?;
    // every day at 2 am
    // 0 2 * * *
    // every 6 hours
    // 0 */6 * * *
    // every hour
    // 0 0 * * * *
    sched
        .add(Job::new_async("@daily", |_uuid, _l| {
            Box::pin(async move {
                if !SWARM_CHECKER.load(Ordering::Relaxed) {
                    SWARM_CHECKER.store(true, Ordering::Relaxed);
                }
            })
        })?)
        .await?;

    sched.start().await?;

    tokio::spawn(async move {
        loop {
            let go = SWARM_CHECKER.load(Ordering::Relaxed);
            if go {
                if let Err(e) = check_all_swarms().await {
                    log::error!("{:?}", e);
                }
                SWARM_CHECKER.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}

pub async fn check_all_swarms() -> Result<()> {
    // get all current swarms
    let state = state::STATE.lock().await;

    let mut hosts: Vec<String> = vec![];

    for swarm in state.stacks.iter() {
        hosts.push(swarm.host.clone())
    }

    drop(state);

    // loop through all swarms
    for host in hosts.iter() {
        // figure out what the correct host is for boltwall
        match get_boltwall_and_navfiber_url(host.clone()) {
            Ok((navfiber_url, boltwall_url)) => {
                log::info!("Navfiber URL: {}", navfiber_url);
                log::info!("Boltwall URL: {}", boltwall_url);
                let boltwall_status = get_boltwall_or_jarvis_status(boltwall_url).await?;

                log::info!("Boltwall STATUS: {}", boltwall_status);
            }
            Err(err) => {
                log::error!("Unable to get boltwall and navfiber url: {}", err)
            }
        }
    }
    // ping each of the services for their current status
    // if any is not responding configure error message
    // send to tribe
    Ok(())
}

fn get_boltwall_and_navfiber_url(host: String) -> Result<(String, String)> {
    if host.contains("swarm") {
        return Ok((
            format!("nav.{}", host.clone()),
            format!("boltwall.{}", host.clone()),
        ));
    }

    return Ok((format!("{}", host), format!("{}/api", host)));
}

fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build super admin reqwest client")
}

async fn get_boltwall_or_jarvis_status(url: String) -> Result<bool> {
    let client = make_client();

    let response = client.get(url).send().await?;

    if response.status() == 200 || response.status() == 401 {
        return Ok(true);
    }

    Ok(false)
}

fn get_navfiber_status(url: String) {}
