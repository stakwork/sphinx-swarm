use crate::config::*;
use crate::defaults::*;
use crate::images::boltwall::BoltwallImage;
use crate::images::bot::BotImage;
use crate::images::jarvis::JarvisImage;
use crate::images::graphmindset::GraphMindsetImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::redis::RedisImage;
use crate::images::repo2graph::Repo2GraphImage;
use crate::images::stakgraph::StakgraphImage;
use crate::images::Image;
use crate::secrets;

pub fn only_graph_mindset(network: &str, host: Option<String>) -> Stack {
    let nodes: Vec<Node> = graph_mindset_imgs(network, host.clone())
        .into_iter()
        .map(|n| Node::Internal(n))
        .collect();

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
            "graphmindset".to_string(),
            "repo2graph".to_string(),
            "stakgraph".to_string(),
            "bot".to_string(),
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

pub fn graph_mindset_imgs(_network: &str, host: Option<String>) -> Vec<Image> {
    // bot (v2 user — replaces CLN + bitcoind)
    let mut bot = BotImage::new("bot", "latest", "3000");
    bot.set_external_broker("broker.v2.sphinx.chat");
    bot.links(vec!["boltwall"]);

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

    // boltwall - linked to bot for v2 lightning
    v = "latest";
    let mut bolt = BoltwallImage::new("boltwall", v, "8444");
    bolt.links(vec!["jarvis", "bot"]);
    bolt.host(host.clone());

    // navfiber (existing frontend)
    v = "latest";
    let mut nav = NavFiberImage::new("navfiber", v, "8000");
    nav.links(vec!["jarvis"]);
    nav.host(host.clone());

    // graphmindset (v2 frontend)
    v = "latest";
    let mut gm = GraphMindsetImage::new("graphmindset", v, "8000");
    gm.links(vec!["jarvis"]);
    gm.host(host.clone());

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
        Image::Bot(bot),
        Image::NavFiber(nav),
        Image::GraphMindset(gm),
        Image::Neo4j(neo4j),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
        Image::Redis(redis),
        Image::Repo2Graph(repo2graph),
        Image::Stakgraph(stakgraph),
    ]
}
