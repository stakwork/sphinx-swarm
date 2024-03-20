use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use chrono::{Local};
use rusoto_core::Region;
use rusoto_s3::{S3, S3Client, PutObjectRequest, ListObjectsV2Request, DeleteObjectsRequest, ObjectIdentifier};
use tokio::task;

// use crate::utils::user;
use anyhow::{anyhow, Context, Result};
use bollard::container::{Config, Stats, StatsOptions};
use bollard::container::{
    CreateContainerOptions, DownloadFromContainerOptions, ListContainersOptions, LogOutput,
    LogsOptions, RemoveContainerOptions, StopContainerOptions, UploadToContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::network::CreateNetworkOptions;
use bollard::service::{ContainerSummary, VolumeListResponse};
use bollard::volume::CreateVolumeOptions;
use bollard::Docker;
use futures_util::{Stream, StreamExt, TryStreamExt};
use rocket::tokio;
use serde::Serialize;

use crate::utils::{domain, sleep_ms};

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
) -> Result<(Option<String>, bool)> {
    let hostname = c.hostname.clone().context("expected hostname")?;
    let current_id = id_by_name(docker, &hostname).await;
    if skip {
        log::info!("=> skip {}", &hostname);
        if let Some(id) = current_id {
            return Ok((Some(id), false));
        } else {
            // dont make the client
            return Ok((None, false));
        }
    }

    // first create volume with the same name, if needed

    if let Some(id) = current_id {
        log::info!("=> {} already exists", &hostname);
        return Ok((Some(id), false));
    }

    create_volume(&docker, &hostname).await?;

    let img_tag = c.image.clone().context("expected image")?;
    // if it contains a "/" its from the registry
    let local_sphinx_image = is_local_sphinx_image(&img_tag);
    if !local_sphinx_image {
        create_image(&docker, &c).await?;
    }
    let id = create_container(&docker, c.clone()).await?;
    log::info!("=> created {}", &hostname);
    Ok((Some(id), true))
}

// returns container id
pub async fn create_and_start(
    docker: &Docker,
    c: Config<String>,
    skip: bool,
) -> Result<Option<String>> {
    let (id_opt, need_to_start) = create_and_init(docker, c, skip).await?;
    if need_to_start {
        let id = id_opt.clone().unwrap_or("".to_string());
        start_container(&docker, &id).await?;
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

pub async fn download_from_container(docker: &Docker, id: &str, path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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
        return Err(format!("Path {} not found", path).into());
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

pub async fn backup_to_s3(proxy_path: &str, relay_path: &str, bucket: &str) -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_local_defaults()?;

    // Download and zip proxy volume
    let proxy_zip_data = download_and_zip_from_container(&docker, "proxy_container_id", proxy_path).await?;
    let proxy_zip_file_name = format!("proxy_data_{}_{}.zip", Local::now().format("%Y%m%d_%H%M%S"));
    let proxy_zip_file = PathBuf::from(&proxy_zip_file_name);
    tokio::fs::write(&proxy_zip_file, proxy_zip_data).await?;

    // Download and zip relay volume
    let relay_zip_data = download_and_zip_from_container(&docker, "relay_container_id", relay_path).await?;
    let relay_zip_file_name = format!("relay_data_{}_{}.zip", Local::now().format("%Y%m%d_%H%M%S"));
    let relay_zip_file = PathBuf::from(&relay_zip_file_name);
    tokio::fs::write(&relay_zip_file, relay_zip_data).await?;

    // Upload proxy zip file to S3
    upload_to_s3(&bucket, &proxy_zip_file_name, proxy_zip_file).await?;

    // Upload relay zip file to S3
    upload_to_s3(&bucket, &relay_zip_file_name, relay_zip_file).await?;

    Ok(())
}

async fn download_and_zip_from_container(docker: &Docker, id: &str, path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let tar_stream = docker.download_from_container(id, Some(DownloadFromContainerOptions { path: path.into() })).await?;
    let mut zip_data = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_data));

    let mut tar = tar::Archive::new(tar_stream);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;

        zip.start_file(path.to_string_lossy().into_owned(), zip::write::FileOptions::default())?;
        zip.write_all(&data)?;
    }

    zip.finish()?;
    Ok(zip_data)
}

async fn zip_data(data: Vec<u8>, name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let current_time = Local::now();
    let zip_file_name = format!("{}_{}.zip", name, current_time.format("%Y%m%d_%H%M%S"));

    let mut zip_file = zip::ZipWriter::new(File::create(&zip_file_name)?);
    let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip_file.start_file(name, options)?;
    zip_file.write_all(&data)?;

    Ok(PathBuf::from(zip_file_name))
}

// Uploads the zip file to S3
async fn upload_to_s3(bucket: &str, zip_file: PathBuf) -> Result<(), Box<dyn Error>> {
    let s3_client = S3Client::new(Region::default());

    let file = File::open(&zip_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let key = zip_file.file_name().and_then(|n| n.to_str()).unwrap_or("backup.zip");

    let request = PutObjectRequest {
        bucket: bucket.to_owned(),
        key: key.to_owned(),
        body: Some(buffer.into()),
        ..Default::default()
    };

    s3_client.put_object(request).await?;
    Ok(())
}

// Deletes old backups from the S3 bucket
async fn delete_old_backups(bucket: &str, prefix: &str, retention_days: i64) -> Result<(), Box<dyn Error>> {
    let s3_client = S3Client::new(Region::default());

    let current_time = Local::now();
    let cutoff_time = current_time - chrono::Duration::days(retention_days);

    let list_request = ListObjectsV2Request {
        bucket: bucket.to_owned(),
        prefix: Some(prefix.to_owned()),
        ..Default::default()
    };

    let objects = s3_client.list_objects_v2(list_request).await?;

    let objects_to_delete: Vec<ObjectIdentifier> = objects.contents
        .unwrap_or_default()
        .iter()
        .filter(|obj| obj.last_modified.as_ref().map_or(false, |lm| lm < &cutoff_time))
        .map(|obj| ObjectIdentifier {
            key: obj.key.clone(),
            ..Default::default()
        })
        .collect();

    if !objects_to_delete.is_empty() {
        let delete_request = DeleteObjectsRequest {
            bucket: bucket.to_owned(),
            delete: rusoto_s3::Delete {
                objects: objects_to_delete,
                ..Default::default()
            },
            ..Default::default()
        };

        s3_client.delete_objects(delete_request).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bucket = "your-bucket-name";
    let zip_data = vec![1, 2, 3, 4, 5]; // Example data to be zipped

    let zip_file = zip_data(zip_data, "backup_data").await?;
    println!("Zip file created: {:?}", zip_file);

    upload_to_s3(bucket, zip_file.clone()).await?;
    println!("Zip file uploaded to S3");

    delete_old_backups(bucket, "backup_prefix", 30).await?;
    println!("Old backups deleted from S3");

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

        println!("==> {:?}", container_stats);
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
