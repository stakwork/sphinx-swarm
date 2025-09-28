use crate::config::*;
use crate::defaults::*;
use crate::images::boltwall::{BoltwallImage};
use crate::images::egress::EgressImage;
use crate::images::jarvis::JarvisImage;
use crate::images::livekit::LivekitImage;
use crate::images::meet::MeetImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::redis::RedisImage;
use crate::images::Image;
use crate::secrets;

pub fn only_graph_mindset(network: &str, host: Option<String>) -> Stack {
    Stack {
        network: network.to_string(),
        nodes: graph_mindset_imgs(host.clone())
            .iter()
            .map(|n| Node::Internal(n.to_owned()))
            .collect(),
        host,
        users: vec![Default::default(), create_super_user()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
        auto_update: Some(vec![
            "jarvis".to_string(),
            "boltwall".to_string(),
            "navfiber".to_string(),
            "livekit".to_string(),
            "egress".to_string(),
            "meet".to_string(),
        ]),
        auto_restart: None,
        custom_2b_domain: env_no_empty("NAV_BOLTWALL_SHARED_HOST"),
        global_mem_limit: None,
        backup_services: Some(vec!["boltwall".to_string(), "neo4j".to_string()]),
        lightning_peers: None,
    }
}

pub fn graph_mindset_imgs(host: Option<String>) -> Vec<Image> {
    // neo4j
    let mut v = "5.19.0";
    let mut neo4j = Neo4jImage::new("neo4j", v);
    neo4j.host(host.clone());

    // redis
    v = "latest";
    let redis = RedisImage::new("redis", v);

    // livekit
    v = "v1.5.0";
    let mut livekit = LivekitImage::new("livekit", v);
    livekit.host(host.clone());
    livekit.links(vec!["redis"]);

    // egress
    v = "v1.8.0";
    let mut egress = EgressImage::new("egress", v, &livekit.api_key, &livekit.api_secret);
    egress.host(host.clone());
    egress.links(vec!["livekit", "redis"]);

    // meet
    v = "latest";
    let mut meet = MeetImage::new("meet", v, &livekit.api_key, &livekit.api_secret);
    meet.host(host.clone());
    meet.links(vec!["livekit"]);

    // jarvis
    v = "latest";
    let mut jarvis = JarvisImage::new("jarvis", v, "6000", false);
    jarvis.links(vec!["neo4j", "boltwall", "redis", "livekit", "egress", "meet"]);

    // boltwall
    v = "latest";
    let mut bolt = BoltwallImage::new("boltwall", v, "8444");
    bolt.links(vec!["jarvis"]);
    bolt.host(host.clone());

    // navfiber
    v = "latest";
    let mut nav = NavFiberImage::new("navfiber", v, "8000");
    nav.links(vec!["jarvis"]);
    nav.host(host.clone());

    let imgs = vec![
        Image::NavFiber(nav),
        Image::Neo4j(neo4j),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
        Image::Redis(redis),
        Image::Livekit(livekit),
        Image::Egress(egress),
        Image::Meet(meet),
    ];

    imgs
}
