use std::collections::HashMap;
use std::env;
use std::path::Path;

// use crate::utils::user;
use anyhow::{anyhow, Context, Result};
use bollard::container::{Config, Stats, StatsOptions};
use bollard::container::{
    CreateContainerOptions, DownloadFromContainerOptions, ListContainersOptions, LogOutput,
    LogsOptions, RemoveContainerOptions, StopContainerOptions, UploadToContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::{CreateImageOptions, PruneImagesOptions};
use bollard::network::CreateNetworkOptions;
use bollard::service::{ContainerSummary, VolumeListResponse};
use bollard::volume::CreateVolumeOptions;
use bollard::Docker;
use futures_util::{Stream, StreamExt, TryStreamExt};
use rocket::tokio;
use serde::Deserialize;
use serde::Serialize;
use std::default::Default;
use std::error::Error;
use std::time::Duration;
use tokio::io::AsyncReadExt;

use crate::backup::bucket_name;
use crate::builder::{find_img, make_client};
use crate::config::{Node, State, STATE};
use crate::conn::swarm::SwarmResponse;
use crate::images::{DockerConfig, DockerHubImage};
use crate::mount_backedup_volume::download_from_s3;
use crate::utils::{domain, getenv, sleep_ms};
use bollard::models::ImageInspect;
use tokio::fs::File;

pub fn dockr() -> Docker {
    Docker::connect_with_unix_defaults().unwrap()
}

fn is_local_sphinx_image(img_tag: &str) -> bool {
    !img_tag.contains("/") && (img_tag.contains("sphinx-") || img_tag.contains("-sphinx"))
}

pub async fn create_and_init(
    docker: &Docker,
    c: Config<String>,
    skip: bool,
) -> Result<(Option<String>, bool, bool)> {
    let hostname = c.hostname.clone().context("expected hostname")?;
    let current_id = id_by_name(docker, &hostname).await;
    if skip {
        log::info!("=> skip {}", &hostname);
        if let Some(id) = current_id {
            return Ok((Some(id), false, false));
        } else {
            // dont make the client
            return Ok((None, false, false));
        }
    }

    // first create volume with the same name, if needed

    if let Some(id) = current_id {
        log::info!("=> {} already exists", &hostname);
        return Ok((Some(id), false, false));
    }

    let created_new_volume = create_volume(&docker, &hostname).await?;

    let img_tag = c.image.clone().context("expected image")?;
    // if it contains a "/" its from the registry
    let local_sphinx_image = is_local_sphinx_image(&img_tag);
    if !local_sphinx_image {
        create_image(&docker, &c).await?;
    }
    let id = create_container(&docker, c.clone()).await?;
    log::info!("=> created {}", &hostname);
    Ok((Some(id), true, created_new_volume))
}

pub async fn pull_image(docker: &Docker, c: Config<String>) -> Result<bool> {
    let img_tag = c.image.clone().context("expected image")?;
    // if it contains a "/" its from the registry
    let local_sphinx_image = is_local_sphinx_image(&img_tag);
    if !local_sphinx_image {
        create_image(&docker, &c).await?;
    }

    Ok(true)
}

// returns container id
pub async fn create_and_start(
    docker: &Docker,
    c: Config<String>,
    skip: bool,
) -> Result<Option<String>> {
    let (id_opt, need_to_start, created_new_volume) = create_and_init(docker, c, skip).await?;
    if need_to_start {
        let id = id_opt.clone().unwrap_or("".to_string());
        start_container(&docker, &id).await?;

        if created_new_volume {
            // download from s3 if it does not exist already, unzip and copy to volume
            restore_backup_if_exist(docker, &id).await?;
            //restart container
            restart_container(&docker, &id).await?;
        }
    }
    Ok(id_opt)
}

fn m1_not_supported(from_image: &str) -> bool {
    let vs = vec!["/sphinx-lss".to_string(), "/cln-sphinx".to_string()];
    let mut b = false;
    for v in vs {
        if from_image.contains(&v) {
            b = true;
        }
    }
    b
}

pub async fn create_image(docker: &Docker, c: &Config<String>) -> Result<()> {
    let from_image = c.image.clone().context("expected image")?;
    let mut opts = CreateImageOptions {
        from_image: from_image.to_string(),
        ..Default::default()
    };
    if m1_not_supported(&from_image) {
        log::info!("running {} on linux/x86_64", &from_image);
        opts.platform = "linux/x86_64".to_string();
    }
    docker
        .create_image::<String>(Some(opts), None, None)
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

pub async fn restart_container(docker: &Docker, id: &str) -> Result<()> {
    docker.restart_container(&id, None).await?;

    Ok(())
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

pub async fn restart_node_container(
    docker: &Docker,
    node_name: &str,
    state: &mut State,
    proj: &str,
) -> Result<()> {
    let img = find_img(node_name, &state.stack.nodes)?;
    let hostname = domain(&node_name);

    let theconfig = img.make_config(&state.stack.nodes, docker).await?;

    img.remove_client(&mut state.clients);

    stop_and_remove(docker, &hostname).await?;

    let new_id = create_container(&docker, theconfig).await?;
    log::info!("=> created {}", &hostname);

    if let Err(e) = img.pre_startup(docker).await {
        log::warn!("pre_startup failed {} {:?}", &new_id, e);
    }

    match start_container(&docker, &new_id).await {
        Ok(()) => {
            log::info!("Container started successfully");
            img.post_startup(proj, docker).await?;

            let _ = match make_client(proj, docker, &img, state).await {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow!("FAILED TO MAKE CLIENT {:?}", e)),
            };
        }
        Err(err) => log::error!("Error starting container: {}", err.to_string()),
    }

    Ok(())
}

pub async fn upload_to_container(
    docker: &Docker,
    img_name: &str,
    path: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<()> {
    let tar = make_tar_from_file(bytes, filename)?;
    Ok(docker
        .upload_to_container::<String>(
            &domain(img_name),
            Some(UploadToContainerOptions {
                path: path.into(),
                ..Default::default()
            }),
            tar.into(),
        )
        .await?)
}

fn make_tar_from_file(bytes: &[u8], filename: &str) -> Result<Vec<u8>> {
    use tar::{Builder, Header};
    let mut header = Header::new_gnu();
    header.set_path(filename)?;
    header.set_size(bytes.len() as u64);
    header.set_cksum();
    let mut ar = Builder::new(Vec::new());
    ar.append(&header, bytes)?;
    let data = ar.into_inner()?;
    Ok(data)
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
        return Err(anyhow!("path {} not found", path));
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

pub async fn create_volume(docker: &Docker, name: &str) -> Result<bool> {
    if let Ok(_v) = docker.inspect_volume(name).await {
        return Ok(false);
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
    Ok(true)
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

// returns container id
pub async fn get_container_statistics(
    docker: &Docker,
    container_filter: Option<String>,
) -> Result<Vec<ContainerStat>> {
    let mut filter = HashMap::new();
    filter.insert(String::from("status"), vec![String::from("running")]);
    if let Some(cn) = container_filter {
        filter.insert(String::from("name"), vec![cn]);
    }
    let containers = &docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters: filter,
            ..Default::default()
        }))
        .await?;

    if containers.is_empty() {
        return Err(anyhow::anyhow!("no running containers"));
    } else {
        let mut container_stats = Vec::new();
        for container in containers {
            let container_id = container.id.as_ref().unwrap();
            let stream = &mut docker
                .stats(
                    container_id,
                    Some(StatsOptions {
                        stream: false,
                        ..Default::default()
                    }),
                )
                .take(1);

            if let Some(Ok(stats)) = stream.next().await {
                let container_name = sphinx_container(&container.names);
                if let Some(cont_name) = container_name {
                    let container_stat =
                        ContainerStat::new(container_id, cont_name, container.image.clone(), stats);
                    container_stats.push(container_stat);
                }
            }
        }

        log::info!("==> {:?}", container_stats);
        Ok(container_stats)
    }
}

// only containers with domains that end in .sphinx
pub fn sphinx_container(names: &Option<Vec<String>>) -> Option<String> {
    if let Some(names) = names.clone() {
        if let Some(name) = names.get(0) {
            if name.ends_with(".sphinx") {
                let mut n = name.clone();
                if n.starts_with("/") {
                    n.remove(0);
                }
                return Some(n);
            }
        }
    };
    None
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetImageDigestResponse {
    pub success: bool,
    pub digest: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetImageActualVersionResponse {
    pub success: bool,
    pub data: Option<Vec<ImageVersion>>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DockerHubImageResult {
    pub name: String,
    pub digest: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DockerHubResponse {
    pub results: Vec<DockerHubImageResult>,
    pub next: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageVersion {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ParsedNode {
    pub name: String,
    pub org: String,
    pub host: String,
}

// get image digest
pub async fn get_image_digest(image_name: &str) -> Result<GetImageDigestResponse> {
    let docker = Docker::connect_with_local_defaults()?;

    let image_info: bollard_stubs::models::ImageInspect = docker.inspect_image(image_name).await?;

    let error_response = GetImageDigestResponse {
        success: false,
        digest: "".to_string(),
        message: "digest does not exist".to_string(),
    };

    if let Some(digests) = image_info.repo_digests {
        if let Some(first_digest) = digests.get(0) {
            return Ok(GetImageDigestResponse {
                success: true,
                digest: first_digest.to_string(),
                message: "digest retrieved successfully".to_string(),
            });
        } else {
            return Ok(error_response);
        }
    } else {
        return Ok(error_response);
    }
}

pub async fn get_image_actual_version(nodes: &Vec<Node>) -> Result<GetImageActualVersionResponse> {
    let docker = Docker::connect_with_local_defaults()?;

    let mut parsed_node: Vec<ParsedNode> = vec![ParsedNode {
        name: "swarm".to_string(),
        host: "sphinx-swarm".to_string(),
        org: "".to_string(),
    }];

    for node in nodes.iter() {
        let node_name = node.name();
        let host = domain(&node_name);

        let node_image = find_img(&node_name, nodes)?;
        let org = node_image.repo().org;

        parsed_node.push(ParsedNode {
            name: node_name,
            org: org,
            host: host,
        })
    }

    let mut images_version: Vec<ImageVersion> = Vec::new();

    for node in parsed_node.iter() {
        let node_name = node.name.clone();
        let image_id = get_image_id(&docker, &node.host).await;
        if image_id.is_empty() {
            images_version.push(ImageVersion {
                name: node_name,
                version: "unavaliable".to_string(),
            });
            continue;
        }

        let digest = get_image_digest_from_image_id(&docker, &image_id).await;
        if digest.is_empty() {
            log::error!("Error getting {} image digest", node_name);
            images_version.push(ImageVersion {
                name: node_name,
                version: "unavaliable".to_string(),
            });
            continue;
        }

        let digest_parts: Vec<&str> = digest.split("@").collect();

        let mut image_name = digest_parts[0].to_string();
        let checksome = digest_parts[1];

        if node.org == "library" {
            image_name = format!("{}/{}", node.org, image_name);
        }

        let version = get_image_version_from_digest(&image_name, checksome).await;

        images_version.push(ImageVersion {
            name: node_name,
            version: version,
        });
    }

    Ok(GetImageActualVersionResponse {
        success: true,
        message: "image actual versions".to_string(),
        data: Some(images_version),
    })
}

async fn get_image_id(docker: &Docker, container_name: &str) -> String {
    match docker.inspect_container(container_name, None).await {
        Ok(container_info) => {
            if container_info.image.is_none() {
                return "".to_string();
            }
            return container_info.image.unwrap();
        }
        Err(err) => {
            log::error!("Container image is unavailable: {:?}", err);
            return "".to_string();
        }
    }
}

async fn get_image_digest_from_image_id(docker: &Docker, image_id: &str) -> String {
    let image_info: ImageInspect = match docker.inspect_image(image_id).await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Error getting name and digest: {:?}", err);
            return "".to_string();
        }
    };

    let image_digest = image_info
        .repo_digests
        .as_ref()
        .and_then(|digests| digests.first().cloned());

    if image_digest.is_none() {
        return "".to_string();
    }

    image_digest.unwrap()
}

pub async fn get_image_version_from_digest(image_name: &str, digest: &str) -> String {
    let docker_url = format!(
        "https://hub.docker.com/v2/repositories/{}/tags?page=1&page_size=100",
        image_name,
    );
    let version = get_version_from_docker_hub(&docker_url, digest).await;
    version
}

pub async fn get_version_from_docker_hub(docker_url: &str, digest: &str) -> String {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(40))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build swarm updater reqwest client");

    let response = match client.get(docker_url).send().await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Error making request to {}: {:?}", docker_url, err);
            return "".to_string();
        }
    };

    let response_json: DockerHubResponse = match response.json().await {
        Ok(parsed) => parsed,
        Err(err) => {
            log::error!("Error parsing response from {}: {:?}", docker_url, err);
            return "".to_string();
        }
    };

    for image in response_json.results {
        if let Some(image_digest) = image.digest {
            if image_digest == digest && image.name != "latest" {
                return image.name;
            }
        }
    }
    if let Some(next_url) = response_json.next {
        if !next_url.is_empty() {
            return Box::pin(get_version_from_docker_hub(&next_url, digest)).await;
        }
    }

    return "".to_string();
}

pub async fn prune_images(docker: &Docker) {
    let mut filters = HashMap::new();
    // By setting dangling to `["false"]`, it will consider both dangling and non-dangling images
    filters.insert("dangling", vec!["true"]);

    let prune_options = PruneImagesOptions { filters };

    match docker.prune_images(Some(prune_options)).await {
        Ok(prune_result) => {
            log::info!("Pruned images: {:?}", prune_result);
        }
        Err(e) => {
            log::error!("Error pruning images: {}", e);
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ContainerStat {
    container_id: String,
    container_name: String,
    container_image: String,
    cpu_total_usage: u64,
    system_cpu_usage: u64,
    memory_usage: u64,
    memory_max_usage: u64,
}

impl ContainerStat {
    pub fn new(
        container_id: &str,
        container_name: String,
        container_image: Option<String>,
        stats: Stats,
    ) -> Self {
        Self {
            container_id: container_id.to_owned(),
            container_name: container_name.to_owned(),
            container_image: container_image.unwrap_or("".to_string()),
            cpu_total_usage: stats.cpu_stats.cpu_usage.total_usage,
            system_cpu_usage: stats.cpu_stats.system_cpu_usage.unwrap_or(0),
            memory_usage: stats.memory_stats.usage.unwrap_or(0),
            memory_max_usage: stats.memory_stats.max_usage.unwrap_or(0),
        }
    }
}

async fn copy_data_to_volume(
    docker: &Docker,
    name: &str,
    inner_root_path: &str,
    root_volume: &str,
    data_path: &str,
) -> Result<(), Box<dyn Error>> {
    let host = domain(name);

    let temp_root = format!("/temp_{}", &name);

    exec(docker, &host, &format!("mkdir -p {}", &temp_root)).await?;

    // Open the tar file
    let mut tar_file = File::open(data_path).await?;

    // Read the tar file into a buffer
    let mut buffer = Vec::new();
    tar_file.read_to_end(&mut buffer).await?;

    // Upload the tar file to the container
    docker
        .upload_to_container(
            &host,
            Some(UploadToContainerOptions {
                path: temp_root.clone(),
                ..Default::default()
            }),
            buffer.into(),
        )
        .await?;

    new_exec(
        &docker,
        &host,
        &format!(
            "rm -rf {}/* && mv -f {}/{}/* {}",
            root_volume, temp_root, inner_root_path, root_volume
        ),
    )
    .await?;

    Ok(())
}

fn directory_exists(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir()
}

fn file_exists(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.is_file()
}

pub async fn restore_backup_if_exist(docker: &Docker, name: &str) -> Result<bool> {
    let state = STATE.lock().await;

    let nodes = state.stack.nodes.clone();

    let to_backup = if let Some(services) = state.stack.backup_services.clone() {
        services
    } else {
        return Ok(false);
    };

    drop(state);

    //check if backup s3 link is passed
    if to_backup.contains(&name.to_string()) {
        log::info!("About to restore: {}", name);
        let img = find_img(&name, &nodes)?;

        if let Ok(backup_link) = getenv("BACKUP_KEY") {
            if !directory_exists(&backup_link) {
                let _ = download_from_s3(&bucket_name(), &backup_link).await;
            } else {
                log::info!("Directory exist");
            }

            let root_volume = img.repo().root_volume.clone();

            let data_path = format!("{}/{}/{}.tar", &backup_link, &name, &name);

            let inner_root_path = get_last_segment(&root_volume);

            log::info!("Current Output path: {}", &data_path);

            if file_exists(&data_path) {
                //create temporary container
                match copy_data_to_volume(
                    &docker,
                    &name,
                    &inner_root_path,
                    &root_volume,
                    &data_path,
                )
                .await
                {
                    Ok(_) => {
                        log::info!("Copied data to volume successfully");
                        return Ok(true);
                    }
                    Err(error) => {
                        log::error!("Error details: {:?}", error);
                        log::error!("Error copying data to volume");
                        return Ok(false);
                    }
                }
            } else {
                log::error!("Could not find file with this data path: {}", data_path);
            }
        }
        return Ok(false);
    }
    return Ok(false);
}

pub async fn new_exec(
    docker: &Docker,
    id: &str,
    cmd: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let txts: Vec<&str> = vec!["sh", "-c", cmd];

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
        return Err("Failed to attach to the exec instance".into());
    }

    let output = ret.join("\n");
    log::info!("Command output: {}", output);

    Ok(output)
}

pub fn get_last_segment(path: &str) -> &str {
    match path.rfind('/') {
        Some(pos) => &path[pos + 1..],
        None => path,
    }
}

pub async fn get_env_variables_by_container_name(docker: &Docker, id: &str) -> SwarmResponse {
    let host = domain(id);
    match docker.inspect_container(&host, None).await {
        Ok(container_info) => {
            if let Some(config) = container_info.config {
                if let Some(envs) = config.env {
                    let mut env_vars: HashMap<String, String> = HashMap::new();
                    for env in envs {
                        let parts: Vec<&str> = env.splitn(2, '=').collect();

                        if let Some(env_value) = parts.get(1) {
                            env_vars.insert(parts[0].to_string(), env_value.to_string());
                        } else {
                            env_vars.insert(parts[0].to_string(), "".to_string());
                        }
                    }
                    let env_vars_json = match serde_json::to_value(env_vars) {
                        Ok(json) => json,
                        Err(err) => {
                            log::error!("Error converting hashmap to Value: {}", err.to_string());
                            return SwarmResponse {
                                success: false,
                                message: format!(
                                    "Error converting hashmap to Value: {}",
                                    err.to_string()
                                ),
                                data: None,
                            };
                        }
                    };
                    SwarmResponse {
                        success: true,
                        message: "Environment variables".to_string(),
                        data: Some(env_vars_json),
                    }
                } else {
                    log::error!("Could not find env");
                    SwarmResponse {
                        success: false,
                        message: "Could not find env".to_string(),
                        data: None,
                    }
                }
            } else {
                log::error!("Could not find container configs");
                SwarmResponse {
                    success: false,
                    message: "Could not find container configs".to_string(),
                    data: None,
                }
            }
        }
        Err(err) => {
            log::error!("Error inspecting container: {:?}", err);
            SwarmResponse {
                success: false,
                message: "Error inspecting container".to_string(),
                data: None,
            }
        }
    }
}
