use crate::api::*;
use anyhow::Result;
use bollard::Docker;

pub async fn run(docker: &Docker) -> Result<()> {
    let all = list_containers(docker).await?;
    if all.len() == 0 {
        log::info!("=> no running containers");
    }
    for c in all {
        if let Some(id) = c.id {
            log::info!("=> pulling down {:?}", c.names.unwrap().get(0).unwrap());
            remove_container(docker, id.as_str()).await?;
        }
    }
    Ok(())
}
