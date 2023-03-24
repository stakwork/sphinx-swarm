use anyhow::Result;
use rocket::tokio::signal;
use sphinx_swarm::builder;
use sphinx_swarm::config::{Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::cln::ClnPlugin;
use sphinx_swarm::images::{btc::BtcImage, cln::ClnImage, Image};

// docker run -it --privileged --pid=host debian nsenter -t 1 -m -u -n -i sh

// cd /var/lib/docker/volumes/

const BTC: &str = "btc_1";
const CLN1: &str = "cln_1";
const CLN2: &str = "cln_2";

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let stack = make_stack();
    let mut clients = builder::build_stack("cln", &docker, &stack).await?;

    let cln2 = clients.cln.get_mut(CLN2).unwrap();
    let cln2_info = cln2.get_info().await?;
    let cln2_pubkey = hex::encode(cln2_info.id);
    log::info!("CLN2 pubkey {}", &cln2_pubkey);

    let cln1 = clients.cln.get_mut(CLN1).unwrap();
    let connected = cln1
        .connect_peer(&cln2_pubkey, &format!("{}.sphinx", CLN2), "9736")
        .await?;
    let channel = hex::encode(connected.id);
    log::info!("CLN1 connected to CLN2: {}", channel);
    let funded = cln1.try_fund_channel(&channel, 100_000_000).await?;
    log::info!("funded {:?}", hex::encode(funded.tx));
    let addr = cln1.new_addr().await?;

    let btcrpc = clients.bitcoind.get(BTC).unwrap();
    let address = addr.bech32.unwrap();
    btcrpc.test_mine(6, Some(address.clone()))?;
    log::info!("mined 6 blocks to {:?}", address);

    let mut ok = false;
    log::info!("wait for channel to confirm...");
    while !ok {
        let peers = cln1.list_peers().await?;
        for p in peers.peers {
            for c in p.channels {
                // println!("{:?}", c.status);
                if c.status.get(0).unwrap().starts_with("CHANNELD_NORMAL") {
                    log::info!("channel confirmed!!!");
                    ok = true;
                }
            }
        }
        sleep(1000).await;
    }

    let sent_keysend = cln1.keysend(&cln2_pubkey, 1_000_000).await?;
    println!("=> sent_keysend {:?}", sent_keysend.status);

    log::info!("stack created!");
    signal::ctrl_c().await?;

    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    // bitcoind
    let v = "v23.0";
    let mut bitcoind = BtcImage::new(BTC, v, &network, "sphinx");
    bitcoind.set_password("password");

    let v = "v23.02";
    let mut cln = ClnImage::new(CLN1, v, &network, "9735", "10009");
    let plugins = vec![ClnPlugin::HsmdBroker];
    cln.plugins(plugins);
    cln.links(vec!["btc_1"]);

    let mut cln2 = ClnImage::new(CLN2, v, &network, "9736", "10010");
    cln2.links(vec!["btc_1"]);

    let internal_nodes = vec![Image::Btc(bitcoind), Image::Cln(cln), Image::Cln(cln2)];
    let nodes: Vec<Node> = internal_nodes
        .iter()
        .map(|n| Node::Internal(n.to_owned()))
        .collect();
    Stack {
        network,
        nodes,
        host: None,
        users: vec![Default::default()],
        jwt_key: sphinx_swarm::secrets::random_word(16),
    }
}

/*

docker exec -it bitcoind.test sh

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -getinfo

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass createwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass loadwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -generate 6

*/
