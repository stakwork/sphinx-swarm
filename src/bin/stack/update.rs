use anyhow::Context;
use bollard::container::Config;
use bollard::Docker;
use reqwest::Url;
use rocket::tokio::sync::MutexGuard;
use sphinx_swarm::cmd::UpdateNode;
use sphinx_swarm::config::{ExternalNodeType, Node, State};
use sphinx_swarm::conn::lnd::utils::{dl_cert, dl_macaroon, strip_pem_prefix_suffix};
use sphinx_swarm::dock::stop_and_remove;
use sphinx_swarm::images::boltwall::BoltwallImage;
use sphinx_swarm::images::btc::BtcImage;
use sphinx_swarm::images::cache::CacheImage;
use sphinx_swarm::images::jarvis::JarvisImage;
use sphinx_swarm::images::lnd::{to_lnd_network, LndImage};
use sphinx_swarm::images::navfiber::NavFiberImage;
use sphinx_swarm::images::neo4j::Neo4jImage;
use sphinx_swarm::images::proxy::ProxyImage;
use sphinx_swarm::images::relay::RelayImage;
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
            let old_btc = action_node.as_btc()?;
            let mut btc = BtcImage::new(
                &old_btc.name,
                &node.version,
                &old_btc.network,
                &old_btc.user,
            );
            btc.set_password(&old_btc.pass);

            node_update = Some(Node::Internal(Btc(btc.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::btc::btc(&btc));
        }
        "Lnd" => {
            let old_lnd = action_node.as_lnd()?;
            let mut lnd = LndImage::new(
                &old_lnd.name,
                &node.version,
                &old_lnd.network,
                &old_lnd.rpc_port,
                &old_lnd.peer_port,
            );
            if let Some(http_port) = old_lnd.http_port {
                lnd.http_port = Some(http_port);
            }
            let links: Vec<&str> = to_vec_str(&old_lnd.links);
            lnd.links(links);
            lnd.host(old_lnd.host);

            let li = LinkedImages::from_nodes(lnd.links.clone(), &nodes);
            let btc = li.find_btc().context("BTC required for LND")?;

            node_update = Some(Node::Internal(Lnd(lnd.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::lnd::lnd(&lnd, &btc));
        }
        "Relay" => {
            let old_relay = action_node.as_relay()?;

            let mut relay = RelayImage::new(
                &old_relay.name,
                &node.version,
                &old_relay.node_env,
                &old_relay.port,
            );
            let links: Vec<&str> = to_vec_str(&old_relay.links);
            relay.links(links);
            relay.host(old_relay.host.clone());

            let li = LinkedImages::from_nodes(relay.links.clone(), &nodes);
            let lnd = li.find_lnd().context("LND required for Relay")?;
            let proxy = li.find_proxy();

            node_update = Some(Node::Internal(Relay(relay.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::relay::relay(&relay, &lnd, proxy));
        }
        "Proxy" => {
            let old_proxy = action_node.as_proxy()?;
            let mut proxy = ProxyImage::new(
                &old_proxy.name,
                &node.version,
                &old_proxy.network,
                &old_proxy.port,
                &old_proxy.admin_port,
            );

            let links: Vec<&str> = to_vec_str(&old_proxy.links);
            proxy.new_nodes(Some("0".to_string()));
            proxy.links(links);

            let li = LinkedImages::from_nodes(proxy.links.clone(), &nodes);
            let lnd = li.find_lnd().context("LND required for Proxy")?;

            node_update = Some(Node::Internal(Proxy(proxy.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::proxy::proxy(&proxy, &lnd));
        }
        "Cache" => {
            let old_cache = action_node.as_cache()?;
            let mut cache = CacheImage::new(&old_cache.name, &node.version, &old_cache.port, true);
            let links: Vec<&str> = to_vec_str(&old_cache.links);
            cache.links(links);

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
            let old_neo4j = action_node.as_neo4j()?;
            let neo4j = Neo4jImage::new(&old_neo4j.name, &node.version);

            node_update = Some(Node::Internal(Neo4j(neo4j.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::neo4j::neo4j(&neo4j));
        }
        "NavFiber" => {
            let old_nav = action_node.as_navfiber()?;
            let mut nav = NavFiberImage::new(&old_nav.name, &node.version, &old_nav.port);
            let links: Vec<&str> = to_vec_str(&old_nav.links);
            nav.links(links);
            nav.host(old_nav.host.clone());

            node_update = Some(Node::Internal(NavFiber(nav.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::navfiber::navfiber(&nav));
        }
        "JarvisBackend" => {
            let old_jarvis = action_node.as_jarvis()?;
            let mut jarvis = JarvisImage::new(&old_jarvis.name, &node.version, &old_jarvis.port);
            let links: Vec<&str> = to_vec_str(&old_jarvis.links);
            jarvis.links(links);

            let neo4j = get_iternal_node(nodes, "neo4j")?.as_neo4j()?;

            node_update = Some(Node::Internal(Jarvis(jarvis.clone())));
            node_index = get_node_position(&nodes, &node.id);
            new_node = Some(images::jarvis::jarvis(&jarvis, &neo4j));
        }
        "Boltwall" => {
            let old_bolt = action_node.as_boltwall()?;
            let mut bolt = BoltwallImage::new(&old_bolt.name, &node.version, &old_bolt.port);
            let links: Vec<&str> = to_vec_str(&old_bolt.links);
            bolt.links(links);
            bolt.host(old_bolt.host.clone());

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

fn to_vec_str(links: &Vec<String>) -> Vec<&str> {
    links.iter().map(|s| s as &str).collect()
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
