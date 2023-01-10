use crate::utils::user;
use anyhow::Result;
use bollard::container::Config;
use bollard::container::{CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::service::ContainerSummary;
use bollard::volume::CreateVolumeOptions;
use bollard::Docker;
use futures_util::{Stream, StreamExt, TryStreamExt};
use rocket::tokio;
use std::collections::HashMap;
use std::env;

pub fn er() -> Docker {
    Docker::connect_with_socket_defaults().unwrap()
}

pub async fn create_volume(docker: &Docker, name: &str) -> Result<()> {
    let mut vconf = CreateVolumeOptions {
        name: name.to_string(),
        driver: "local".to_string(),
        ..Default::default()
    };
    if let Some(u) = user() {
        let mut driver_opts = HashMap::new();
        driver_opts.insert("uid".to_string(), u);
        vconf.driver_opts = driver_opts;
    }
    docker.create_volume(vconf).await?;
    Ok(())
}

pub async fn create_and_start(docker: &Docker, c: Config<String>) -> Result<String> {
    // if it contains a "/" its from the registry
    if c.image.clone().unwrap().contains("/") {
        create_image(&docker, &c).await?;
    }
    let id = create_container(&docker, c).await?;
    start_container(&docker, &id).await?;
    Ok(id)
}

pub async fn create_image(docker: &Docker, c: &Config<String>) -> Result<()> {
    docker
        .create_image::<String>(
            Some(CreateImageOptions {
                from_image: c.image.clone().unwrap().into(),
                ..Default::default()
            }),
            None,
            None,
        )
        .try_collect::<Vec<_>>()
        .await?;
    Ok(())
}

pub async fn create_container(docker: &Docker, c: Config<String>) -> Result<String> {
    let name: String = c.hostname.clone().unwrap().into();
    let create_opts = CreateContainerOptions { name };
    let id = docker
        .create_container::<String, String>(Some(create_opts), c)
        .await?
        .id;
    Ok(id)
}

pub async fn start_container(docker: &Docker, id: &str) -> Result<()> {
    Ok(docker.start_container::<String>(id, None).await?)
}

pub async fn list_containers(docker: &Docker) -> Result<Vec<ContainerSummary>> {
    Ok(docker.list_containers::<String>(None).await?)
}

pub async fn remove_container(docker: &Docker, id: &str) -> Result<()> {
    docker
        .remove_container(
            id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

pub async fn container_logs(docker: &Docker, name: &str) -> Vec<String> {
    let tail_name = "LOG_TAIL_LENGTH";
    let tail = env::var(tail_name).unwrap_or(100.to_string());

    let options = Some(LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail,
        ..Default::default()
    });

    let mut stream = docker.logs(name, options);
    let mut ret: Vec<String> = Vec::new();
    while let Some(lg) = stream.next().await {
        if let Ok(log) = lg {
            let msg = match log {
                LogOutput::StdOut { message } => message,
                LogOutput::StdErr { message } => message,
                LogOutput::Console { message } => message,
                LogOutput::StdIn { message } => message,
            };
            ret.push(String::from_utf8_lossy(&msg).to_string());
        }
    }
    ret
}

pub fn logs_stream(
    docker: &Docker,
    name: &str,
) -> impl Stream<Item = Result<LogOutput, bollard::errors::Error>> {
    let options = Some(LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        ..Default::default()
    });
    docker.logs(name, options)
}
pub fn match_stream(log_output: Result<LogOutput, bollard::errors::Error>) -> Option<Vec<u8>> {
    match log_output {
        Ok(lo) => match lo {
            LogOutput::StdOut { message } => Some(message.to_vec()),
            LogOutput::StdErr { message } => Some(message.to_vec()),
            LogOutput::Console { message } => Some(message.to_vec()),
            LogOutput::StdIn { message } => Some(message.to_vec()),
        },
        Err(_) => None,
    }
}

pub async fn exec(docker: &Docker, id: &str, cmd: &str) -> Result<String> {
    let txts = cmd.split(" ").filter(|t| t.len() > 0).collect();
    let exec = docker
        .create_exec(
            id,
            CreateExecOptions {
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                cmd: Some(txts),
                ..Default::default()
            },
        )
        .await?
        .id;
    let started = docker.start_exec(&exec, None).await?;
    let mut ret = Vec::new();
    sleep(400).await;
    if let StartExecResults::Attached { mut output, .. } = started {
        while let Some(Ok(msg)) = output.next().await {
            ret.push(msg.to_string());
        }
    } else {
        unreachable!();
    }
    Ok(ret.join("/n"))
}

pub async fn sleep(millis: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(millis)).await;
}
