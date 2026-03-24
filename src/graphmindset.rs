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
    let mut internal_nodes = vec![];
    let mut external_nodes = vec![];

    add_btc(network, &mut internal_nodes, &mut external_nodes);

    let imgs = graph_mindset_imgs(network, host.clone());
    internal_nodes.extend(imgs);

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

pub fn graph_mindset_imgs(network: &str, host: Option<String>) -> Vec<Image> {
    // cln
    let seed_str = std::env::var("SEED").expect("SEED env var required for graph_mindset");
    if seed_str.len() != 64 {
        panic!("SEED must be 64 hex chars");
    }
    let seed_vec = hex::decode(&seed_str).expect("SEED decode failed");
    let seed = hex::encode(seed_vec);

    let mut cln = ClnImage::new("cln", "latest", network, "9735", "10009");
    cln.set_seed(seed);
    cln.plugins(vec![ClnPlugin::HtlcInterceptor]);
    cln.host(host.clone());
    cln.links(vec!["bitcoind"]);

    // neo4j
    let mut v = "5.19.0";
    let mut neo4j = Neo4jImage::new("neo4j", v);
    neo4j.host(host.clone());

    // redis
    v = "latest";
    let redis = RedisImage::new("redis", v);

    // jarvis
    v = "latest";
    let mut jarvis = JarvisImage::new("jarvis", v, "6000", false);
    jarvis.links(vec!["neo4j", "boltwall", "redis"]);

    // boltwall - linked to cln for internal lightning
    v = "latest";
    let mut bolt = BoltwallImage::new("boltwall", v, "8444");
    bolt.links(vec!["jarvis", "cln"]);
    bolt.host(host.clone());

    // navfiber
    v = "latest";
    let mut nav = NavFiberImage::new("navfiber", v, "8000");
    nav.links(vec!["jarvis"]);
    nav.host(host.clone());

    // repo2graph
    v = "latest";
    let mut repo2graph = Repo2GraphImage::new("repo2graph", v, "3355");
    repo2graph.host(host.clone());
    repo2graph.links(vec!["neo4j", "boltwall"]);

    // stakgraph
    v = "latest";
    let mut stakgraph = StakgraphImage::new("stakgraph", v, "7799");
    stakgraph.host(host.clone());
    stakgraph.links(vec!["neo4j", "boltwall"]);

    vec![
        Image::Cln(cln),
        Image::NavFiber(nav),
        Image::Neo4j(neo4j),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
        Image::Redis(redis),
        Image::Repo2Graph(repo2graph),
        Image::Stakgraph(stakgraph),
    ]
}
