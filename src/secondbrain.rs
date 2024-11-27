use crate::config::*;
use crate::defaults::*;
use crate::images::boltwall::{BoltwallImage, ExternalLnd};
use crate::images::elastic::ElasticImage;
use crate::images::jarvis::JarvisImage;
use crate::images::llama::LlamaImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
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
        ]),
        custom_2b_domain: env_no_empty("NAV_BOLTWALL_SHARED_HOST"),
        global_mem_limit: None,
        backup_services: Some(vec!["boltwall".to_string(), "neo4j".to_string()]),
        lightning_peers: None,
    }
}

pub fn second_brain_imgs(host: Option<String>, lightning_provider: &str) -> Vec<Image> {
    // neo4j
    let v = "5.19.0";
    let mut neo4j = Neo4jImage::new("neo4j", v);
    neo4j.host(host.clone());

    // elastic
    let mut v = "8.11.1";
    let mut elastic = ElasticImage::new("elastic", v);
    elastic.host(host.clone());

    // jarvis
    v = "latest";
    let mut jarvis = JarvisImage::new("jarvis", v, "6000", false);
    jarvis.links(vec!["neo4j", "elastic", "boltwall"]);

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
    let mut nav = NavFiberImage::new("navfiber", v, "8001");
    nav.links(vec!["jarvis"]);
    nav.host(host.clone());

    let mut imgs = vec![
        Image::NavFiber(nav),
        Image::Neo4j(neo4j),
        Image::Elastic(elastic),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
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
        custom_2b_domain: None,
        global_mem_limit: None,
        backup_services: None,
        lightning_peers: None,
    }
}
