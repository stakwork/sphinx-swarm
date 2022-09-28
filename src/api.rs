use anyhow::Result;
use bollard::container::Config;
use bollard::container::{CreateContainerOptions, RemoveContainerOptions};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::TryStreamExt;
use serde::Serialize;
use std::hash::Hash;

pub async fn create_image<T: Into<String> + Eq + Hash + Clone>(
    docker: &Docker,
    c: &Config<T>,
) -> Result<()> {
    docker
        .create_image(
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

pub async fn create_container<T: Into<String> + Eq + Hash + Clone + Serialize>(
    docker: &Docker,
    c: Config<T>,
) -> Result<String> {
    let name: &str = &c.hostname.clone().unwrap().into();
    let create_opts = CreateContainerOptions { name };
    let id = docker
        .create_container::<&str, T>(Some(create_opts), c)
        .await?
        .id;
    Ok(id)
}

pub async fn start_container(docker: &Docker, id: &str) -> Result<()> {
    Ok(docker.start_container::<String>(id, None).await?)
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
