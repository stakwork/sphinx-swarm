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
use crate::images::traefik::TraefikImage;
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
    // In development mode, use MEET_DOMAIN as base domain if no host is provided
    let base_domain = if host.is_none() && std::env::var("RUST_ENV").unwrap_or_else(|_| "production".to_string()) == "development" {
        std::env::var("MEET_DOMAIN").unwrap_or_else(|_| "localhost".to_string())
    } else {
        host.unwrap_or_else(|| "localhost".to_string())
    };
    
    // neo4j
    let mut v = "5.19.0";
    let mut neo4j = Neo4jImage::new("neo4j", v);
    if base_domain != "localhost" {
        neo4j.host(Some(format!("neo4j.{}", base_domain)));
    }

    // redis
    v = "latest";
    let redis = RedisImage::new("redis", v);

    // livekit
    v = "v1.5.0";
    let mut livekit = LivekitImage::new("livekit", v);
    livekit.host(Some(base_domain.clone())); // Pass base domain directly, livekit.host() will handle subdomain
    livekit.links(vec!["redis"]);

    // egress
    v = "v1.8.0";
    let mut egress = EgressImage::new("egress", v, &livekit.api_key, &livekit.api_secret);
    if base_domain != "localhost" {
        egress.host(Some(format!("egress.{}", base_domain)));
    }
    egress.links(vec!["livekit", "redis"]);

    // meet (sphinx-livekit)
    v = "latest";
    let mut meet = MeetImage::new("meet", v, &livekit.api_key, &livekit.api_secret);
    meet.host(Some(base_domain.clone())); // Pass base domain directly, meet.host() will use it as-is
    meet.links(vec!["livekit"]);

    // jarvis
    v = "latest";
    let mut jarvis = JarvisImage::new("jarvis", v, "6000", false);
    jarvis.links(vec!["neo4j", "boltwall", "redis", "livekit", "egress", "meet"]);

    // boltwall  
    v = "latest";
    let mut bolt = BoltwallImage::new("boltwall", v, "8444");
    bolt.links(vec!["jarvis"]);
    if base_domain != "localhost" {
        bolt.host(Some(format!("boltwall.{}", base_domain)));
    }

    // navfiber
    v = "latest";
    let mut nav = NavFiberImage::new("navfiber", v, "8000");
    nav.links(vec!["jarvis"]);
    if base_domain != "localhost" {
        nav.host(Some(format!("navfiber.{}", base_domain)));
    }

    let mut imgs = vec![
        Image::NavFiber(nav),
        Image::Neo4j(neo4j),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
        Image::Redis(redis),
        Image::Livekit(livekit),
        Image::Egress(egress),
        Image::Meet(meet),
    ];

    // Add Traefik only in development mode
    // if std::env::var("RUST_ENV").unwrap_or_else(|_| "production".to_string()) == "development" {
    //     let traefik = TraefikImage::new("traefik");
    //     imgs.push(Image::Traefik(traefik));
    // }

    imgs
}
