use sphinx_swarm::dock::*;

#[rocket::main]
pub async fn main() -> anyhow::Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let mut vol = false;
    if let Some(arg) = std::env::args().nth(1) {
        if arg == "-v" {
            vol = true
        }
    }

    let all = list_containers(&docker).await?;
    if all.len() == 0 {
        log::info!("=> no running containers");
    }
    for c in all {
        if let Some(name) = should_go(&c.names) {
            if let Some(id) = c.id {
                log::info!("=> pulling down {:?}", &name);
                stop_and_remove(&docker, id.as_str()).await?;
            }
        }
    }

    let vols = list_volumes(&docker).await?;
    if let Some(vols) = vols.volumes {
        if vol && vols.len() > 0 {
            for v in vols {
                if v.name.ends_with(".sphinx") {
                    log::info!("=> removing volume {:?}", &v.name);
                    remove_volume(&docker, &v.name).await?;
                }
            }
        }
    }

    Ok(())
}

// only containers with domains that end in .sphinx
fn should_go(names: &Option<Vec<String>>) -> Option<String> {
    if let Some(names) = names.clone() {
        if let Some(name) = names.get(0) {
            if name.ends_with(".sphinx") {
                return Some(name.clone());
            }
        }
    };
    None
}
