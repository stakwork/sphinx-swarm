use crate::config::*;
use crate::defaults::*;
use crate::images::broker::BrokerImage;
use crate::images::cln::{ClnImage, ClnPlugin};
use crate::images::config_server::ConfigImage;
use crate::images::mixer::MixerImage;
use crate::images::tribes::TribesImage;
use crate::images::Image;
use crate::secrets;
use crate::utils::getenv;

pub fn sphinxv2_only(network: &str, host: Option<String>) -> Stack {
    let seed_str = std::env::var("SEED").expect("no seed");
    if seed_str.len() != 64 {
        panic!("seed must be 64 hex chars");
    }
    let seed_vec = hex::decode(&seed_str).expect("seed decode");
    let seed = hex::encode(seed_vec);

    let is_router = match std::env::var("IS_ROUTER").ok() {
        Some(ir) => ir == "true",
        None => false,
    };

    let mut internal_nodes = vec![];
    let mut external_nodes = vec![];

    add_btc(&network, &mut internal_nodes, &mut external_nodes);

    let mut cln = ClnImage::new("cln", "latest", network, "9735", "10009");
    cln.set_seed(seed.clone());
    if !is_router {
        let cln_plugins = vec![ClnPlugin::HtlcInterceptor];
        cln.plugins(cln_plugins);
    }
    cln.host(host.clone());
    cln.links(vec!["bitcoind"]);
    internal_nodes.push(Image::Cln(cln));

    let mut broker = BrokerImage::new(
        "broker",
        "latest",
        network,
        "1883",                   // mqtt
        Some("5005".to_string()), // ws
    );
    broker.set_seed(&seed_str);
    broker.host(host.clone());
    internal_nodes.push(Image::Broker(broker));

    let mut mixer = MixerImage::new("mixer", "latest", network, "8800");
    mixer.links(vec!["cln", "broker"]);
    mixer.host(host.clone());
    internal_nodes.push(Image::Mixer(mixer));

    if !is_router {
        let mut tribes = TribesImage::new("tribes", "latest", network, "8801");
        tribes.links(vec!["broker"]);
        tribes.host(host.clone());
        internal_nodes.push(Image::Tribes(tribes));
    }

    let mut nodes: Vec<Node> = internal_nodes
        .iter()
        .map(|n| Node::Internal(n.to_owned()))
        .collect();
    nodes.extend(external_nodes);

    Stack {
        network: network.to_string(),
        nodes: nodes,
        host,
        users: vec![Default::default()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
        auto_update: None,
        custom_2b_domain: None,
    }
}

// for testing
pub fn sphinxv1_only(network: &str, host: Option<String>) -> Stack {
    let mut broker = BrokerImage::new("broker", "latest", network, "1883", None);
    broker.host(host.clone());

    let mut mixer = MixerImage::new("mixer", "latest", network, "8800");
    mixer.set_no_lightning();
    mixer.links(vec!["broker"]);
    mixer.host(host.clone());

    let mut tribes = TribesImage::new("tribes", "latest", network, "8801");
    tribes.links(vec!["broker"]);
    tribes.host(host.clone());

    Stack {
        network: network.to_string(),
        nodes: vec![
            Image::Broker(broker),
            Image::Mixer(mixer),
            Image::Tribes(tribes),
        ]
        .iter()
        .map(|n| Node::Internal(n.to_owned()))
        .collect(),
        host,
        users: vec![Default::default()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
        auto_update: None,
        custom_2b_domain: None,
    }
}

fn cfg_img() -> anyhow::Result<ConfigImage> {
    let regtest_tribe = getenv("REGTEST_TRIBE")?;
    let regtest_tribe_host = getenv("REGTEST_TRIBE_HOST")?;
    let regtest_router = getenv("REGTEST_ROUTER")?;
    let regtest_default_lsp = getenv("REGTEST_DEFAULT_LSP")?;
    let mainnet_tribe = getenv("MAINNET_TRIBE")?;
    let mainnet_tribe_host = getenv("MAINNET_TRIBE_HOST")?;
    let mainnet_router = getenv("MAINNET_ROUTER")?;
    let mainnet_default_lsp = getenv("MAINNET_DEFAULT_LSP")?;
    Ok(ConfigImage::new(
        "config",
        "latest",
        "8001",
        regtest_tribe,
        regtest_tribe_host,
        regtest_router,
        regtest_default_lsp,
        mainnet_tribe,
        mainnet_tribe_host,
        mainnet_router,
        mainnet_default_lsp,
    ))
}

pub fn config_only(host: Option<String>) -> Stack {
    let mut cfg = cfg_img().expect("bad cfg img");

    cfg.host(host.clone());

    let nodes = vec![Node::Internal(Image::Config(cfg))];

    Stack {
        network: "bitcoin".to_string(),
        nodes,
        host,
        users: vec![Default::default()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
        auto_update: None,
        custom_2b_domain: None,
    }
}
