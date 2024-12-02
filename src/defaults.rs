use crate::config::*;
use crate::images::cln::{ClnImage, ClnPlugin};
use crate::images::{
    btc::BtcImage, cache::CacheImage, lnd::LndImage, lss::LssImage, proxy::ProxyImage,
    relay::RelayImage, Image,
};
use crate::secondbrain::*;
use crate::secrets;
use crate::sphinxv2::*;
use crate::utils::{getenv, make_reqwest_client};

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

        if env_is_true("CHATUI_ONLY") {
            return only_local_chat_ui();
        }

        if env_is_true("SPHINXV1") {
            return sphinxv1_only(&network, host);
        }

        if env_is_true("SPHINXV2") {
            return sphinxv2_only(&network, host);
        }

        if env_is_true("LLAMA_ONLY") {
            return llama_only(&network, host);
        }

        if env_is_true("IS_CONFIG") {
            return config_only(host);
        }

        let use_lnd = env_is_true("USE_LND");
        // choose cln or lnd
        let is_cln = !use_lnd;
        let lightning_provider = if is_cln { "cln" } else { "lnd" };

        if env_is_true("SECOND_BRAIN_ONLY") {
            return only_second_brain(&network, host.clone(), lightning_provider);
        }

        let mut internal_nodes = vec![];
        let mut external_nodes = vec![];

        add_btc(&network, &mut internal_nodes, &mut external_nodes);

        if is_cln {
            let skip_remote_signer = env_is_true("NO_REMOTE_SIGNER");
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
        let skip_second_brain = env_is_true("NO_SECOND_BRAIN");
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

        let mut users = vec![User::default()];
        let superuser = create_super_user();
        users.push(superuser);

        Stack {
            network,
            nodes,
            host,
            users,
            jwt_key: secrets::random_word(16),
            ready: false,
            ip: env_no_empty("IP"),
            auto_update: None,
            auto_restart: None,
            custom_2b_domain: env_no_empty("NAV_BOLTWALL_SHARED_HOST"),
            global_mem_limit: None,
            backup_services: None,
            lightning_peers: None,
        }
    }
}

pub fn create_super_user() -> User {
    let password = crate::secrets::hex_secret_32();
    let password_ = password.clone();
    tokio::spawn(async move {
        let error_msg = "is not set in the environment variable for setting up superadmin";
        //get x-super-token
        let super_token = getenv("SUPER_TOKEN").unwrap_or("".to_string());

        if super_token.is_empty() {
            log::error!("SUPER_TOKEN {}", &error_msg);
            return;
        }

        //get super url
        let super_url = getenv("SUPER_URL").unwrap_or("".to_string());

        if super_url.is_empty() {
            log::error!("SUPER_URL {}", &error_msg);
            return;
        }

        let default_host = getenv("HOST").unwrap_or("".to_string());

        if default_host.is_empty() {
            log::error!("HOST {}", &error_msg);
            return;
        }

        //get swarm host
        let mut my_domain = getenv("NAV_BOLTWALL_SHARED_HOST").unwrap_or("".to_string());

        if my_domain.is_empty() {
            my_domain = default_host.clone();
        }

        let client = make_reqwest_client();

        let route = format!("{}/super/add_new_swarm", super_url);

        let body = SendSwarmDetailsBody {
            username: "super".to_string(),
            password: password_,
            host: my_domain,
            default_host: default_host,
        };

        match client
            .post(route.as_str())
            .header("x-super-token", super_token)
            .json(&body)
            .send()
            .await
        {
            Ok(res) => {
                if res.status().clone() != 201 {
                    log::error!("Response code: {:?}", res.status().clone());
                    match res.text().await {
                        Ok(data) => {
                            log::error!("{:?}", data)
                        }
                        Err(err) => {
                            log::error!("Error parsing JSON response: {:?}", err);
                        }
                    }
                    return;
                }
                log::info!("Swarm details sent to super admin successfully")
            }
            Err(err) => {
                log::error!("Error sending Swarm details to admin: {:?}", err)
            }
        }
    });
    User {
        id: 2,
        username: "super".to_string(),
        pass_hash: bcrypt::hash(&password, bcrypt::DEFAULT_COST).expect("failed to bcrypt"),
        pubkey: None,
        role: Role::Super,
    }
}

pub fn add_btc(network: &str, internal_nodes: &mut Vec<Image>, external_nodes: &mut Vec<Node>) {
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
}

pub fn env_no_empty(varname: &str) -> Option<String> {
    match std::env::var(varname).ok() {
        Some(v) => match v.as_str() {
            "" => None,
            s => Some(s.to_string()),
        },
        None => None,
    }
}

pub fn env_is_true(v: &str) -> bool {
    match std::env::var(v).ok() {
        Some(sbo) => sbo == "true",
        None => false,
    }
}

// for testing
pub fn llama_only(network: &str, host: Option<String>) -> Stack {
    let mut llama = crate::images::llama::LlamaImage::new("llama", "8080");
    llama.host(host.clone());
    Stack {
        network: network.to_string(),
        nodes: vec![Image::Llama(llama)]
            .iter()
            .map(|n| Node::Internal(n.to_owned()))
            .collect(),
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
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn create_password_and_password_hash() {
        let password = crate::secrets::hex_secret_32();
        println!("Logging Password: {}", password);
        let hashed_password =
            bcrypt::hash(&password, bcrypt::DEFAULT_COST).expect("failed to bcrypt");
        println!("Hashed Password: {}", hashed_password);
    }
}
