use anyhow::{anyhow, Result};
use bollard::Docker;
use sphinx_swarm::cmd::UpdateNode;
use sphinx_swarm::config::Node;
use sphinx_swarm::dock::{create_and_start, stop_and_remove};
use sphinx_swarm::images::DockerConfig;

pub async fn update_node(docker: &Docker, un: &UpdateNode, nodes: &mut Vec<Node>) -> Result<()> {
    let pos = nodes.iter().position(|n| n.name() == un.id);
    let hostname = format!("{}.sphinx", &un.id);
    if let None = pos {
        return Err(anyhow!("cannot find node in stack"));
    }
    let pos = pos.unwrap();

    // stop the node
    stop_and_remove(docker, &hostname).await?;

    nodes[pos].set_version(&un.version)?;

    let theimg = nodes[pos].as_internal()?;
    let theconfig = theimg.make_config(nodes, docker).await?;

    create_and_start(docker, theconfig, false).await?;

    Ok(())
}
