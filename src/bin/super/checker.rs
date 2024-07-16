use anyhow::Result;
use rocket::tokio;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::state;

pub static SWARM_CHECKER: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BotMsgBody {
    action: String,
    bot_id: String,
    bot_secret: String,
    chat_uuid: String,
    content: String,
}

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
    let mut message = "".to_string();

    for swarm in state.stacks.iter() {
        hosts.push(swarm.host.clone())
    }

    drop(state);

    // loop through all swarms
    for host in hosts.iter() {
        // figure out what the correct host is for boltwall
        match get_boltwall_and_navfiber_url(host.clone()) {
            Ok((navfiber_url, boltwall_url)) => {
                // ping each of the services for their current status
                let boltwall_status = get_boltwall_or_jarvis_status(boltwall_url.clone()).await?;
                let jarvis_status =
                    get_boltwall_or_jarvis_status(format!("{}stats", boltwall_url.clone())).await?;
                let navfiber_status = get_navfiber_status(navfiber_url.clone()).await?;

                // if any is not responding configure error message
                let new_message =
                    configure_error_msg(boltwall_status, jarvis_status, navfiber_status, &host);
                if !new_message.is_empty() {
                    if message.is_empty() {
                        message = format!("{}", new_message)
                    } else {
                        message = format!("{}\n\n{}", message, new_message)
                    }
                }
            }
            Err(err) => {
                log::error!("Unable to get boltwall and navfiber url: {}", err)
            }
        }
    }

    // send to tribe
    println!("{}", message);
    if !message.is_empty() {
        send_message_to_tribe(message).await?;
    }
    Ok(())
}

fn get_boltwall_and_navfiber_url(host: String) -> Result<(String, String)> {
    if host.contains("swarm") {
        return Ok((
            format!("https://nav.{}/", host.clone()),
            format!("https://boltwall.{}/api/", host.clone()),
        ));
    }

    return Ok((
        format!("https://{}/", host),
        format!("https://{}/api/", host),
    ));
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

    let response = client.get(&url).send().await?;

    if response.status() == 200 || response.status() == 401 {
        return Ok(true);
    }

    Ok(false)
}

async fn get_navfiber_status(url: String) -> Result<bool> {
    let client = make_client();

    let response = client.get(&url).send().await?;

    if response.status() == 200 {
        return Ok(true);
    }

    Ok(false)
}

fn configure_error_msg(
    boltwall_status: bool,
    jarvis_status: bool,
    navfiber_status: bool,
    host: &str,
) -> String {
    let sub_heading = format!("The following services are down for {}", host);
    let mut message = "".to_string();

    if !boltwall_status {
        message = configure_msg("Boltwall", message, &host);
    }

    if !jarvis_status {
        message = configure_msg("Jarvis", message, &host);
    }

    if !navfiber_status {
        message = configure_msg("Navfiber", message, &host);
    }

    message
}

fn configure_msg(service: &str, mut message: String, host: &str) -> String {
    let sub_heading = format!("The following services are down for {}", host);

    if message.is_empty() {
        message = format!("{}", sub_heading.clone());
    }
    message = format!("{}\n{}", message, service);

    message
}

async fn send_message_to_tribe(message: String) -> Result<()> {
    let client = make_client();

    let route = "http://localhost:3001/action";

    let body = BotMsgBody {
        content: message,
        bot_id: "171E8407B2F7FE69719D7186".to_string(),
        bot_secret: "58ABD069FA0B16142F23EA51B20B57DA".to_string(),
        chat_uuid: "Zpbish9buc-mp4PuAxkXP1JM3Cj0XvLenRY5wypHonb0ep4qLWyaRBIjzPXITR0eg6fF0kgp0EfP12b62DADhklupWTx".to_string(),
        action: "broadcast".to_string(),
    };

    let response = client.post(route).json(&body).send().await?;

    println!("{:?}", response.json().await?);
    Ok(())
}
