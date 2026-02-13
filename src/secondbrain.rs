use crate::config::*;
use crate::defaults::*;
use crate::images::boltwall::{BoltwallImage, ExternalLnd};
use crate::images::jarvis::JarvisImage;
use crate::images::llama::LlamaImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::quickwit::QuickwitImage;
use crate::images::redis::RedisImage;
use crate::images::repo2graph::Repo2GraphImage;
use crate::images::stakgraph::StakgraphImage;
use crate::images::vector::VectorImage;
use crate::images::Image;
use crate::secrets;

pub fn only_second_brain(network: &str, host: Option<String>, lightning_provider: &str) -> Stack {
    Stack {
        network: network.to_string(),
        nodes: second_brain_imgs(host.clone(), lightning_provider)
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
            "repo2graph".to_string(),
            "stakgraph".to_string(),
        ]),
        auto_restart: None,
        custom_2b_domain: env_no_empty("NAV_BOLTWALL_SHARED_HOST"),
        global_mem_limit: None,
        backup_services: Some(vec!["boltwall".to_string(), "neo4j".to_string()]),
        lightning_peers: None,
        ssl_cert_last_modified: None,
        instance_id: None,
    }
}

pub fn second_brain_imgs(host: Option<String>, lightning_provider: &str) -> Vec<Image> {
    // neo4j
    let mut v = "5.19.0";
    let mut neo4j = Neo4jImage::new("neo4j", v);
    neo4j.host(host.clone());

    // elastic
    // let mut v = "8.11.1";
    // let mut elastic = ElasticImage::new("elastic", v);
    // elastic.host(host.clone());

    // redis
    v = "latest";
    let redis = RedisImage::new("redis", v);

    // jarvis
    v = "latest";
    let mut jarvis = JarvisImage::new("jarvis", v, "6000", false);
    jarvis.links(vec!["neo4j", "elastic", "boltwall", "redis"]);

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

    // boltwall
    v = "latest";
    let mut bolt = BoltwallImage::new("boltwall", v, "8444");
    if let Some(ext) = external_lnd() {
        bolt.external_lnd(ext);
        bolt.links(vec!["jarvis"]);
    } else {
        bolt.links(vec!["jarvis", lightning_provider]);
    }
    bolt.host(host.clone());

    // navfiber
    v = "latest";
    let mut nav = NavFiberImage::new("navfiber", v, "8000");
    nav.links(vec!["jarvis"]);
    nav.host(host.clone());

    let mut imgs = vec![
        Image::NavFiber(nav),
        Image::Neo4j(neo4j),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
        Image::Redis(redis),
        Image::Repo2Graph(repo2graph),
        Image::Stakgraph(stakgraph),
    ];

    if env_is_true("LOCAL_LLAMA") {
        let mut llama = LlamaImage::new("llama", "8787");
        llama.links(vec!["jarvis"]);
        llama.host(host.clone());
        llama.set_pwd("/home/admin/sphinx-swarm");
        imgs.push(Image::Llama(llama));
    }

    imgs
}

pub fn external_lnd() -> Option<ExternalLnd> {
    if let Some(a) = env_no_empty("EXTERNAL_LND_ADDRESS") {
        if let Some(m) = env_no_empty("EXTERNAL_LND_MACAROON") {
            if let Some(c) = env_no_empty("EXTERNAL_LND_CERT") {
                return Some(ExternalLnd::new(&a, &m, &c));
            }
        }
    }
    None
}

/*
localtest:
export CHATUI_ONLY=true
llama-server --hf-repo microsoft/Phi-3-mini-4k-instruct-gguf --hf-file Phi-3-mini-4k-instruct-q4.gguf -c 4096
*/
pub fn only_local_chat_ui() -> Stack {
    println!("only_local_chat_ui");
    let mut llamacpp = crate::images::llama::LlamaImage::new("llama", "8080");
    llamacpp.model("Phi-3-mini-4k-instruct-q4.gguf");
    let mongo = crate::images::mongo::MongoImage::new("mongo", "latest");
    let mut jamie = crate::images::jamie::JamieImage::new("jamie", "latest");
    jamie.links(vec!["mongo", "llama"]);
    let nodes = vec![
        Node::Internal(Image::Llama(llamacpp)),
        Node::Internal(Image::Mongo(mongo)),
        Node::Internal(Image::Jamie(jamie)),
    ];
    return default_local_stack(None, "regtest", nodes);
}

pub fn default_local_stack(host: Option<String>, network: &str, nodes: Vec<Node>) -> Stack {
    Stack {
        network: network.to_string(),
        nodes,
        host,
        users: vec![Default::default()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: None,
        auto_update: None,
        auto_restart: None,
        custom_2b_domain: None,
        global_mem_limit: None,
        backup_services: None,
        lightning_peers: None,
        ssl_cert_last_modified: None,
        instance_id: None,
    }
}

/// Spin up only Quickwit and Vector for log ingestion testing
/// Usage: set ONLY_LOGS=true and run the stack
pub fn only_logs(host: Option<String>) -> Stack {
    // quickwit - log storage and search (internal only, no external access)
    let quickwit = QuickwitImage::new("quickwit", "latest");
    // NOTE: no quickwit.host() - keeps it internal, not exposed via Traefik

    // vector - log ingestion (exposed for external log drains)
    let mut vector = VectorImage::new("vector", "latest-distroless-libc");
    vector.host(host.clone());
    vector.links(vec!["quickwit"]);

    let nodes = vec![
        Node::Internal(Image::Quickwit(quickwit)),
        Node::Internal(Image::Vector(vector)),
    ];

    Stack {
        network: "regtest".to_string(),
        nodes,
        host,
        users: vec![Default::default()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
        auto_update: None,
        auto_restart: None,
        custom_2b_domain: None,
        global_mem_limit: None,
        backup_services: None,
        lightning_peers: None,
        ssl_cert_last_modified: None,
        instance_id: None,
    }
}
