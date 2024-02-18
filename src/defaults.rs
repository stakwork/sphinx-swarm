use crate::config::*;
use crate::images::boltwall::{BoltwallImage, ExternalLnd};
use crate::images::broker::BrokerImage;
use crate::images::cln::{ClnImage, ClnPlugin};
use crate::images::elastic::ElasticImage;
use crate::images::jarvis::JarvisImage;
use crate::images::mixer::MixerImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::tribes::TribesImage;
use crate::images::{
    btc::BtcImage, cache::CacheImage, lnd::LndImage, lss::LssImage, proxy::ProxyImage,
    relay::RelayImage, Image,
};
use crate::secrets;

fn sphinxv1_only(network: &str, host: Option<String>) -> Stack {
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

fn only_second_brain(network: &str, host: Option<String>, lightning_provider: &str) -> Stack {
    Stack {
        network: network.to_string(),
        nodes: second_brain_imgs(host.clone(), lightning_provider)
            .iter()
            .map(|n| Node::Internal(n.to_owned()))
            .collect(),
        host,
        users: vec![Default::default()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
        auto_update: Some(vec![
            "jarvis".to_string(),
            "boltwall".to_string(),
            "navfiber".to_string(),
        ]),
        custom_2b_domain: env_no_empty("NAV_BOLTWALL_SHARED_HOST"),
    }
}

fn env_no_empty(varname: &str) -> Option<String> {
    match std::env::var(varname).ok() {
        Some(v) => match v.as_str() {
            "" => None,
            s => Some(s.to_string()),
        },
        None => None,
    }
}

fn external_lnd() -> Option<ExternalLnd> {
    if let Some(a) = env_no_empty("EXTERNAL_LND_ADDRESS") {
        if let Some(m) = env_no_empty("EXTERNAL_LND_MACAROON") {
            if let Some(c) = env_no_empty("EXTERNAL_LND_CERT") {
                return Some(ExternalLnd::new(&a, &m, &c));
            }
        }
    }
    None
}

fn second_brain_imgs(host: Option<String>, lightning_provider: &str) -> Vec<Image> {
    // neo4j
    let v = "4.4.9";
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

    vec![
        Image::NavFiber(nav),
        Image::Neo4j(neo4j),
        Image::Elastic(elastic),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
    ]
}

// NETWORK = "bitcoin", "regtest"
// HOST = hostname for this server (swarmx.sphinx.chat)
// BTC_PASS = already created BTC password
// ONLY_NODE = start up just one node
impl Default for Stack {
    fn default() -> Self {
        // network
        let mut network = "regtest".to_string();
        if let Ok(env_net) = std::env::var("NETWORK") {
            if env_net == "bitcoin" || env_net == "regtest" {
                network = env_net;
            }
        }

        // host
        let mut host = std::env::var("HOST").ok();
        // must include a "."
        if let Some(h) = host.clone() {
            log::info!("HOST {:?}", h);
        }
        if !host.clone().unwrap_or(".".to_string()).contains(".") {
            host = None
        }

        // choose only second brain
        let sphinxv1 = match std::env::var("SPHINXV1").ok() {
            Some(sbo) => sbo == "true",
            None => false,
        };
        if sphinxv1 {
            return sphinxv1_only(&network, host);
        }

        let use_lnd = match std::env::var("USE_LND").ok() {
            Some(sbo) => sbo == "true",
            None => false,
        };
        // choose cln or lnd
        let is_cln = !use_lnd;
        let lightning_provider = if is_cln { "cln" } else { "lnd" };

        // choose only second brain
        let second_brain_only = match std::env::var("SECOND_BRAIN_ONLY").ok() {
            Some(sbo) => sbo == "true",
            None => false,
        };
        if second_brain_only {
            return only_second_brain(&network, host.clone(), lightning_provider);
        }

        let mut internal_nodes = vec![];
        let mut external_nodes = vec![];

        let mut external_btc = false;
        // CLN and external BTC
        if let Ok(ebtc) = std::env::var("CLN_MAINNET_BTC") {
            // check the BTC url is ok
            if let Ok(_) = url::Url::parse(&ebtc) {
                let btc = ExternalNode::new("bitcoind", ExternalNodeType::Btc, &ebtc);
                external_nodes.push(Node::External(btc));
                external_btc = true;
            }
        } else {
            if network == "bitcoin" {
                panic!("CLN_MAINNET_BTC required for mainnet");
            }
        }

        if !external_btc {
            let v = "v23.0";
            let mut bitcoind = BtcImage::new("bitcoind", v, &network);
            // connect to already running BTC node
            if let Ok(btc_pass) = std::env::var("BTC_PASS") {
                // only if its really there (not empty string)
                if btc_pass.len() > 0 {
                    bitcoind.set_user_password("sphinx", &btc_pass);
                }
            }
            // generate random pass if none exists
            if let None = bitcoind.pass {
                bitcoind.set_user_password("sphinx", &secrets::random_word(12));
            }
            internal_nodes.push(Image::Btc(bitcoind));
        }

        if is_cln {
            let skip_remote_signer = match std::env::var("NO_REMOTE_SIGNER").ok() {
                Some(nsb) => nsb == "true",
                None => false,
            };
            if !skip_remote_signer {
                // lightning storage server
                let lss = LssImage::new("lss", "latest", "55551");
                internal_nodes.push(Image::Lss(lss));
            }
            // cln with plugins
            let mut cln = ClnImage::new("cln", "latest", &network, "9735", "10009");
            cln.links(vec!["bitcoind", "lss"]);

            let plugins = if skip_remote_signer {
                vec![ClnPlugin::HtlcInterceptor]
            } else {
                cln.broker_frontend(); // broker parses bitcoind blocks
                vec![ClnPlugin::HsmdBroker, ClnPlugin::HtlcInterceptor]
            };
            cln.plugins(plugins);
            cln.host(host.clone());
            internal_nodes.push(Image::Cln(cln));
        } else {
            // lnd
            let v = "v0.16.2-beta";
            let mut lnd = LndImage::new("lnd", v, &network, "10009", "9735");
            lnd.http_port = Some("8881".to_string());
            lnd.links(vec!["bitcoind"]);
            lnd.host(host.clone());

            internal_nodes.push(Image::Lnd(lnd));
        }

        // proxy
        let mut v = "latest";
        let mut proxy = ProxyImage::new("proxy", v, &network, "11111", "5050");
        proxy.new_nodes(Some("0".to_string()));
        // proxy.channel_cap(Some("100000000".to_string()));
        proxy.links(vec![lightning_provider]);

        // relay
        v = "latest";
        let node_env = match host {
            Some(_) => "production",
            None => "development",
        };
        let mut relay = RelayImage::new("relay", v, node_env, "3000");
        relay.dont_ping_hub();
        relay.set_creds_dir("/relay/data");
        relay.links(vec![
            "proxy",
            lightning_provider,
            "tribes",
            "memes",
            "boltwall",
            "cache",
        ]);
        relay.host(host.clone());

        // cache
        v = "latest";
        let mut cache = CacheImage::new("cache", v, "9000", true);
        cache.links(vec!["tribes"]);

        // other_internal_nodes
        let other_internal_nodes = vec![
            Image::Proxy(proxy),
            Image::Relay(relay),
            Image::Cache(cache),
        ];
        internal_nodes.extend(other_internal_nodes);

        // NO_SECOND_BRAIN=true will skip these nodes
        let skip_second_brain = match std::env::var("NO_SECOND_BRAIN").ok() {
            Some(nsb) => nsb == "true",
            None => false,
        };
        if !skip_second_brain {
            let second_brain_nodes = second_brain_imgs(host.clone(), lightning_provider);
            internal_nodes.extend(second_brain_nodes);
        }

        let mut nodes: Vec<Node> = internal_nodes
            .iter()
            .map(|n| Node::Internal(n.to_owned()))
            .collect();

        // external nodes
        external_nodes.push(Node::External(ExternalNode::new(
            "tribes",
            ExternalNodeType::Tribes,
            "tribes.sphinx.chat",
        )));
        external_nodes.push(Node::External(ExternalNode::new(
            "memes",
            ExternalNodeType::Meme,
            "meme.sphinx.chat",
        )));

        // final nodes array
        nodes.extend(external_nodes);

        Stack {
            network,
            nodes,
            host,
            users: vec![Default::default()],
            jwt_key: secrets::random_word(16),
            ready: false,
            ip: env_no_empty("IP"),
            auto_update: None,
            custom_2b_domain: env_no_empty("NAV_BOLTWALL_SHARED_HOST"),
        }
    }
}
