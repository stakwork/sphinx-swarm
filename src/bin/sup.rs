//! Generates `scripts/sup.sh`: a shell script that updates one second-brain
//! service in place (`./scripts/sup.sh repo2graph`).
//!
//! Re-run whenever an image is added under `src/images/` or a service in
//! `src/secondbrain.rs` changes:
//!
//!     cargo run --bin sup
//!
//! The service names, container names and image tags are read from the same
//! Rust definitions the swarm uses to run the stack, so the script cannot
//! drift from them.

use sphinx_swarm::images::{DockerHubImage, Image};
use sphinx_swarm::secondbrain::second_brain_imgs;
use sphinx_swarm::utils::domain;
use std::fs;
use std::path::{Path, PathBuf};

const OUTPUT: &str = "scripts/sup.sh";
const COMPOSE_FILE: &str = "second-brain-2.yml";
const SWARM_CONTAINER: &str = "sphinx-swarm";

struct Service {
    /// The argument to `sup.sh`, e.g. `repo2graph`
    name: String,
    /// The docker container name, e.g. `repo2graph.sphinx`
    container: String,
    /// The full pull target, e.g. `ghcr.io/stakwork/stakgraph-mcp:latest`
    image: String,
}

impl Service {
    fn from_image(img: &Image) -> Self {
        let name = img.name();
        Self {
            container: domain(&name),
            image: format!("{}:{}", img.image(), img.version()),
            name,
        }
    }
}

/// Every service a second-brain swarm could be running, sorted by name.
fn second_brain_services() -> Vec<Service> {
    // bot and llama are optional members of the stack, gated on the lightning
    // provider and LOCAL_LLAMA. Turn both on so the script knows about them.
    std::env::set_var("LOCAL_LLAMA", "true");
    let mut services: Vec<Service> = second_brain_imgs(None, "bot")
        .iter()
        .map(Service::from_image)
        .collect();
    services.sort_by(|a, b| a.name.cmp(&b.name));
    services
}

const TEMPLATE: &str = r#"#!/bin/bash
# GENERATED FILE - do not edit by hand.
# Regenerate with:  cargo run --bin sup
# Source of truth:  src/secondbrain.rs (services) and src/images/*.rs (image repos)
#
# Usage:  ./scripts/sup.sh <service>
#
# Pulls the service's image, recreates the swarm container, removes the
# service's container so the swarm brings it back up on the new image,
# then follows the service's logs.

cd "$(dirname "$0")/.." || exit 1

case "$1" in
@CASES@
  *)
    echo "=> invalid service name! '$1'"
    echo "=> valid services: @NAMES@"
    exit 1
    ;;
esac

echo "=> pull $IMAGE"
docker pull "$IMAGE" || exit 1

echo "=> stop @SWARM@"
docker stop @SWARM@ && docker rm @SWARM@

echo "=> stop $CONTAINER"
docker stop "$CONTAINER" && docker rm "$CONTAINER"

echo "=> start @SWARM@"
docker-compose -f @COMPOSE@ up @SWARM@ -d

# the swarm recreates the container on startup; wait for it before tailing
echo "=> waiting for $CONTAINER"
for _ in $(seq 1 120); do
  docker inspect "$CONTAINER" >/dev/null 2>&1 && break
  sleep 1
done

docker logs "$CONTAINER" --follow
"#;

fn render(services: &[Service]) -> String {
    let cases: Vec<String> = services
        .iter()
        .map(|s| {
            format!(
                "  {})\n    IMAGE=\"{}\"\n    CONTAINER=\"{}\"\n    ;;",
                s.name, s.image, s.container
            )
        })
        .collect();
    let names: Vec<&str> = services.iter().map(|s| s.name.as_str()).collect();
    TEMPLATE
        .replace("@CASES@", &cases.join("\n"))
        .replace("@NAMES@", &names.join(" "))
        .replace("@SWARM@", SWARM_CONTAINER)
        .replace("@COMPOSE@", COMPOSE_FILE)
}

fn output_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(OUTPUT)
}

fn main() -> anyhow::Result<()> {
    let services = second_brain_services();
    let out = output_path();
    fs::write(&out, render(&services))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&out, fs::Permissions::from_mode(0o755))?;
    }
    println!("=> wrote {} ({} services)", out.display(), services.len());
    for s in &services {
        println!("   {:<13} {:<20} {}", s.name, s.container, s.image);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_maps_each_service_to_its_container_and_image() {
        let services = second_brain_services();
        let script = render(&services);
        let r2g = services
            .iter()
            .find(|s| s.name == "repo2graph")
            .expect("repo2graph is a second-brain service");
        assert_eq!(r2g.container, "repo2graph.sphinx");
        assert_eq!(r2g.image, "ghcr.io/stakwork/stakgraph-mcp:latest");
        assert!(script.contains("  repo2graph)\n    IMAGE=\"ghcr.io/stakwork/stakgraph-mcp:latest\"\n    CONTAINER=\"repo2graph.sphinx\"\n"));
        assert!(script.contains("docker-compose -f second-brain-2.yml up sphinx-swarm -d"));
        assert!(!script.contains('@'), "every placeholder was substituted");
    }

    #[test]
    fn the_checked_in_script_is_up_to_date() {
        let on_disk = fs::read_to_string(output_path())
            .expect("scripts/sup.sh exists; run `cargo run --bin sup`");
        assert_eq!(
            on_disk,
            render(&second_brain_services()),
            "scripts/sup.sh is stale: run `cargo run --bin sup`"
        );
    }
}
