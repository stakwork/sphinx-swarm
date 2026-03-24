use crate::config::*;
use crate::defaults::*;
use crate::images::boltwall::BoltwallImage;
use crate::images::cln::{ClnImage, ClnPlugin};
use crate::images::jarvis::JarvisImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::redis::RedisImage;
use crate::images::repo2graph::Repo2GraphImage;
use crate::images::stakgraph::StakgraphImage;
use crate::images::Image;
use crate::secrets;

pub fn only_graph_mindset(network: &str, host: Option<String>) -> Stack {
    let seed_str = std::env::var("SEED").expect("no seed");
    if seed_str.len() != 64 {
        panic!("seed must be 64 hex chars");
    }
    let seed_vec = hex::decode(&seed_str).expect("seed decode");
    let seed = hex::encode(seed_vec);

    let mut internal_nodes: Vec<Image> = vec![];
    let mut external_nodes: Vec<Node> = vec![];

    add_btc(&network, &mut internal_nodes, &mut external_nodes);

    // cln
    let mut cln = ClnImage::new("cln", "latest", network, "9735", "10009");
    cln.set_seed(seed);
    cln.plugins(vec![ClnPlugin::HtlcInterceptor]);
    cln.links(vec!["bitcoind"]);
    internal_nodes.push(Image::Cln(cln));

    // neo4j
    let mut neo4j = Neo4jImage::new("neo4j", "5.19.0");
    neo4j.host(host.clone());
    internal_nodes.push(Image::Neo4j(neo4j));

    // redis
    let redis = RedisImage::new("redis", "latest");
    internal_nodes.push(Image::Redis(redis));

    // jarvis
    let mut jarvis = JarvisImage::new("jarvis", "latest", "6000", false);
    jarvis.links(vec!["neo4j", "boltwall", "redis"]);
    internal_nodes.push(Image::Jarvis(jarvis));

    // repo2graph
    let mut repo2graph = Repo2GraphImage::new("repo2graph", "latest", "3355");
    repo2graph.host(host.clone());
    repo2graph.links(vec!["neo4j", "boltwall"]);
    internal_nodes.push(Image::Repo2Graph(repo2graph));

    // stakgraph
    let mut stakgraph = StakgraphImage::new("stakgraph", "latest", "7799");
    stakgraph.host(host.clone());
    stakgraph.links(vec!["neo4j", "boltwall"]);
    internal_nodes.push(Image::Stakgraph(stakgraph));

    // boltwall - linked to cln (no external LND)
    let mut bolt = BoltwallImage::new("boltwall", "latest", "8444");
    bolt.links(vec!["jarvis", "cln"]);
    bolt.host(host.clone());
    internal_nodes.push(Image::BoltWall(bolt));

    // navfiber
    let mut nav = NavFiberImage::new("navfiber", "latest", "8000");
    nav.links(vec!["jarvis"]);
    nav.host(host.clone());
    internal_nodes.push(Image::NavFiber(nav));

    let mut nodes: Vec<Node> = internal_nodes
        .iter()
        .map(|n| Node::Internal(n.to_owned()))
        .collect();
    nodes.extend(external_nodes);

    Stack {
        network: network.to_string(),
        nodes,
        host,
        users: vec![Default::default(), create_super_user()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
        auto_update: Some(vec![
            "jarvis".to_string(),
            "boltwall".to_string(),
            "navfiber".to_string(),
            "cln".to_string(),
            "repo2graph".to_string(),
            "stakgraph".to_string(),
        ]),
        auto_restart: None,
        custom_2b_domain: env_no_empty("NAV_BOLTWALL_SHARED_HOST"),
        global_mem_limit: None,
        backup_services: Some(vec!["boltwall".to_string(), "neo4j".to_string()]),
        backup_files: None,
        lightning_peers: None,
        ssl_cert_last_modified: None,
        instance_id: None,
    }
}
