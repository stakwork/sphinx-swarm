use sphinx_swarm::dock::*;

#[rocket::main]
pub async fn main() -> anyhow::Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let all = list_containers(&docker).await?;
    if all.len() == 0 {
        log::info!("=> no running containers");
    }
    for c in all {
        let mut skip = true;
        if let Some(names) = c.names.clone() {
            if let Some(name) = names.get(0) {
                if name.ends_with(".sphinx") {
                    skip = false
                }
            }
        };
        if !skip {
            if let Some(id) = c.id {
                log::info!("=> pulling down {:?}", c.names.unwrap().get(0).unwrap());
                stop_and_remove(&docker, id.as_str()).await?;
            }
        }
    }
    Ok(())
}
