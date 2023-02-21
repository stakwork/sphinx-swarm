use anyhow::Context;
use bollard::container::Config;
use bollard::Docker;
use reqwest::Url;
use rocket::tokio::sync::MutexGuard;
use sphinx_swarm::cmd::UpdateNode;
use sphinx_swarm::config::{ExternalNodeType, Node, State};
use sphinx_swarm::conn::lnd::utils::{dl_cert, dl_macaroon, strip_pem_prefix_suffix};
use sphinx_swarm::dock::stop_and_remove;
use sphinx_swarm::images::lnd::{to_lnd_network};
use sphinx_swarm::images::Image::{
    BoltWall, Btc, Cache, Jarvis, Lnd, NavFiber, Neo4j, Proxy, Relay,
};
use sphinx_swarm::images::{self, LinkedImages};
use url::Host;

pub struct UpdateNodeData {
    pub node_index: Option<usize>,
    pub new_node: Option<Config<String>>,
    pub node_update: Option<Node>,
}

pub async fn update_node(
    docker: &Docker,
    node: &UpdateNode,
    state: &MutexGuard<'_, State>,
) -> Result<UpdateNodeData, anyhow::Error> {
    let nodes = &state.stack.nodes;

    /* Check if the npde is a running node
     * if it does not return error
     */
    let action_node = state
        .stack
        .nodes
        .iter()
        .find(|n| n.name() == node.id.clone())
        .context("Node not found")?
        .as_internal()?;

    let node_id = format!("{}.sphinx", &node.id);

    stop_and_remove(docker, &node_id).await?;

    let mut new_node: Option<Config<String>> = None;
    let mut node_index: Option<usize> = None;
    let mut node_update: Option<Node> = None;

    match action_node.typ().as_str() {
        "Btc" => {
            let mut btc = action_node.as_btc()?;
            btc.version = node.version.clone();

            node_update = Some(Node::Internal(Btc(btc.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::btc::btc(&btc));
        }
        "Lnd" => {
            let mut lnd = action_node.as_lnd()?;
            lnd.version = node.version.clone();

            let li = LinkedImages::from_nodes(lnd.links.clone(), &nodes);
            let btc = li.find_btc().context("BTC required for LND")?;

            node_update = Some(Node::Internal(Lnd(lnd.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::lnd::lnd(&lnd, &btc));
        }
        "Relay" => {
            let mut relay = action_node.as_relay()?;

            relay.version = node.version.clone();

            let li = LinkedImages::from_nodes(relay.links.clone(), &nodes);
            let lnd = li.find_lnd().context("LND required for Relay")?;
            let proxy = li.find_proxy();

            node_update = Some(Node::Internal(Relay(relay.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::relay::relay(&relay, &lnd, proxy));
        }
        "Proxy" => {
            let mut proxy = action_node.as_proxy()?;
            proxy.version = node.version.clone();

            let li = LinkedImages::from_nodes(proxy.links.clone(), &nodes);
            let lnd = li.find_lnd().context("LND required for Proxy")?;

            node_update = Some(Node::Internal(Proxy(proxy.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::proxy::proxy(&proxy, &lnd));
        }
        "Cache" => {
            let mut cache = action_node.as_cache()?;
            cache.version = node.version.clone();

            let memes = nodes
                .iter()
                .find(|n| n.is_ext_of_type(ExternalNodeType::Meme))
                .context("No Memes")?
                .as_external()?;

            let memes_url = Url::parse(format!("https://{}", memes.url).as_str())?;
            let memes_host = memes_url.host().unwrap_or(Host::Domain("")).to_string();

            let tribes = nodes
                .iter()
                .find(|n| n.is_ext_of_type(ExternalNodeType::Tribes))
                .context("No Tribes")?
                .as_external()?;

            let tribes_url = Url::parse(format!("https://{}", tribes.url).as_str())?;
            let tribe_host = tribes_url.host().unwrap_or(Host::Domain("")).to_string();

            node_update = Some(Node::Internal(Cache(cache.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::cache::cache(&cache, &memes_host, &tribe_host));
        }
        "Neo4j" => {
            let mut neo4j = action_node.as_neo4j()?;
            neo4j.version = node.version.clone();

            node_update = Some(Node::Internal(Neo4j(neo4j.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::neo4j::neo4j(&neo4j));
        }
        "NavFiber" => {
            let mut nav = action_node.as_navfiber()?;
            nav.version = node.version.clone();

            node_update = Some(Node::Internal(NavFiber(nav.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::navfiber::navfiber(&nav));
        }
        "JarvisBackend" => {
            let mut jarvis = action_node.as_jarvis()?;
            jarvis.version = node.version.clone();

            let neo4j = get_iternal_node(nodes, "neo4j")?.as_neo4j()?;

            node_update = Some(Node::Internal(Jarvis(jarvis.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::jarvis::jarvis(&jarvis, &neo4j));
        }
        "Boltwall" => {
            let mut bolt = action_node.as_boltwall()?;
            bolt.version = node.version.clone();

            let lnd = get_iternal_node(nodes, "lnd")?.as_lnd()?;
            let jarvis = get_iternal_node(nodes, "jarvis")?.as_jarvis()?;

            let cert_path = "/home/.lnd/tls.cert";
            let cert_full = dl_cert(docker, &lnd.name, cert_path).await?;
            let cert64 = strip_pem_prefix_suffix(&cert_full);
            let netwk = to_lnd_network(lnd.network.as_str());
            let macpath = format!("/home/.lnd/data/chain/bitcoin/{}/admin.macaroon", netwk);
            let mac = dl_macaroon(docker, &lnd.name, &macpath).await?;

            node_update = Some(Node::Internal(BoltWall(bolt.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::boltwall::boltwall(
                &bolt, &mac, &cert64, &lnd, &jarvis,
            ));
        }
        _ => {
            new_node = None;
            println!("Not a swarm node")
        }
    }

    Ok(UpdateNodeData {
        node_index,
        new_node,
        node_update,
    })
}

fn get_iternal_node(nodes: &Vec<Node>, name: &str) -> Result<images::Image, anyhow::Error> {
    let err_msg = format!("No {}", name);
    Ok(nodes
        .iter()
        .find(|n| n.name() == name)
        .context(err_msg)?
        .as_internal()?)
}

fn get_node_position(nodes: &Vec<Node>, name: &str) -> Option<usize> {
    let mut index: Option<usize> = None;
    let node_index = nodes.iter().position(|n| n.name() == name);
    if let Some(i) = node_index {
        index = Some(i)
    }
    index
}
