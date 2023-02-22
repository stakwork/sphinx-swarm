// use crate::utils::user;
use anyhow::{anyhow, Context, Result};
use bollard::container::Config;
use bollard::container::{
    CreateContainerOptions, DownloadFromContainerOptions, ListContainersOptions, LogOutput,
    LogsOptions, RemoveContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::network::CreateNetworkOptions;
use bollard::service::{ContainerSummary, VolumeListResponse};
use bollard::volume::CreateVolumeOptions;
use bollard::Docker;
use futures_util::{Stream, StreamExt, TryStreamExt};
use rocket::tokio;
use std::env;

use crate::utils::{domain, sleep_ms};

pub fn dockr() -> Docker {
    Docker::connect_with_unix_defaults().unwrap()
}

// retuns container id
pub async fn create_and_start(
    docker: &Docker,
    c: Config<String>,
    skip: bool,
) -> Result<Option<String>> {
    let hostname = c.hostname.clone().context("expected hostname")?;
    let current_id = id_by_name(docker, &hostname).await;
    if skip {
        log::info!("=> skip {}", &hostname);
        if let Some(id) = current_id {
            return Ok(Some(id));
        } else {
            // dont make the client
            return Ok(None);
        }
    }

    // first create volume with the same name, if needed

    if let Some(id) = current_id {
        log::info!("=> {} already exists", &hostname);
        return Ok(Some(id));
    }

    create_volume(&docker, &hostname).await?;

    let img_tag = c.image.clone().context("expected image")?;
    // if it contains a "/" its from the registry
    let local_sphinx_image = img_tag.contains("sphinx-") && !img_tag.contains("/");
    if !local_sphinx_image {
        create_image(&docker, &c).await?;
    }
    let id = create_container(&docker, c.clone()).await?;
    start_container(&docker, &id).await?;
    log::info!("=> created {}", &hostname);
    Ok(Some(id))
}

pub async fn create_image(docker: &Docker, c: &Config<String>) -> Result<()> {
    docker
        .create_image::<String>(
            Some(CreateImageOptions {
                from_image: c.image.clone().context("expected image")?.into(),
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
    let name: String = c.hostname.clone().context("expected hostname")?.into();
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
    Ok(docker
        .list_containers::<String>(Some(ListContainersOptions {
            all: true,
            ..Default::default()
        }))
        .await?)
}

pub async fn id_by_name(docker: &Docker, the_name: &str) -> Option<String> {
    let cs = match list_containers(docker).await {
        Err(_) => return None,
        Ok(co) => co,
    };
    for c in cs {
        if let Some(names) = c.names.clone() {
            if let Some(name) = names.get(0) {
                if name.contains(the_name) {
                    return c.id;
                }
            }
        };
    }
    None
}

pub async fn stop_and_remove(docker: &Docker, id: &str) -> Result<()> {
    stop_container(docker, id).await?;
    remove_container(&docker, &id).await?;
    Ok(())
}

pub async fn stop_container(docker: &Docker, id: &str) -> Result<()> {
    docker
        .stop_container(id, Some(StopContainerOptions { t: 9 }))
        .await?;
    Ok(())
}

pub async fn remove_container(docker: &Docker, id: &str) -> Result<()> {
    docker
        .remove_container(
            id,
            Some(RemoveContainerOptions {
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

pub async fn download_from_container(docker: &Docker, id: &str, path: &str) -> Result<Vec<u8>> {
    let mut tar = docker.download_from_container::<String>(
        id,
        Some(DownloadFromContainerOptions { path: path.into() }),
    );
    let mut ret: Vec<u8> = Vec::new();
    while let Some(bytes_res) = tar.next().await {
        if let Ok(bytes) = bytes_res {
            ret.extend_from_slice(&bytes);
        }
    }
    if ret.len() == 0 {
        return Err(anyhow::anyhow!("path {} not found", path));
    }
    Ok(unzip_tar_single_file(ret)?)
}

fn unzip_tar_single_file(bytes: Vec<u8>) -> Result<Vec<u8>> {
    use std::io::Read;
    use tar::Archive;
    let mut a = Archive::new(&bytes[..]);
    for file in a.entries().unwrap() {
        if let Err(e) = file {
            return Err(anyhow::anyhow!(format!("failed to unzip tar: {}", e)));
        }
        let mut file = file.unwrap();
        // Inspect metadata about the file
        // println!("file path: {:?}", file.header().path().unwrap());
        // files implement the Read trait
        let mut s = Vec::new();
        return match file.read_to_end(&mut s) {
            Ok(_) => Ok(s),
            Err(e) => Err(anyhow::anyhow!(format!("failed to read tar file: {}", e))),
        };
        // println!("=====> FILE <======");
        // println!("{}", s);
    }
    Err(anyhow::anyhow!("no tar file found"))
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

pub async fn create_volume(docker: &Docker, name: &str) -> Result<()> {
    if let Ok(_v) = docker.inspect_volume(name).await {
        return Ok(());
    }
    let vconf = CreateVolumeOptions {
        name: name.to_string(),
        driver: "local".to_string(),
        ..Default::default()
    };
    // if let Some(u) = user() {
    //     let mut driver_opts = HashMap::new();
    //     driver_opts.insert("uid".to_string(), u);
    //     vconf.driver_opts = driver_opts;
    // }
    docker.create_volume(vconf).await?;
    Ok(())
}

pub async fn remove_volume(docker: &Docker, name: &str) -> Result<()> {
    if let Err(_e) = docker.inspect_volume(name).await {
        return Ok(());
    }
    docker.remove_volume(name, None).await?;
    Ok(())
}

pub async fn list_volumes(docker: &Docker) -> Result<VolumeListResponse> {
    Ok(docker.list_volumes::<String>(None).await?)
}

pub const DEFAULT_NETWORK: &str = "sphinx-swarm";

pub async fn create_network(docker: &Docker, name: Option<&str>) -> Result<String> {
    let name = name.unwrap_or(DEFAULT_NETWORK);
    if let Ok(_v) = docker.inspect_network::<String>(name, None).await {
        return Ok(name.to_string());
    }
    let vconf = CreateNetworkOptions {
        name: name.to_string(),
        ..Default::default()
    };
    docker.create_network(vconf).await?;
    Ok(name.to_string())
}

pub async fn remove_network(docker: &Docker, name: Option<&str>) -> Result<String> {
    let name = name.unwrap_or(DEFAULT_NETWORK);
    if let Err(_) = docker.inspect_network::<String>(name, None).await {
        return Ok(name.to_string());
    }
    docker.remove_network(name).await?;
    Ok(name.to_string())
}

pub async fn try_dl(docker: &Docker, name: &str, path: &str) -> Result<Vec<u8>> {
    for _ in 0..60 {
        if let Ok(bytes) = download_from_container(docker, &domain(name), path).await {
            return Ok(bytes);
        }
        sleep_ms(500).await;
    }
    Err(anyhow!(format!(
        "try_dl failed to find {} in {}",
        path, name
    )))
}
