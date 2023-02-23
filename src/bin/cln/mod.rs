use anyhow::Result;
use rocket::tokio::signal;
use sphinx_swarm::builder;
use sphinx_swarm::config::{Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::{btc::BtcImage, cln::ClnImage, Image};

// docker run -it --privileged --pid=host debian nsenter -t 1 -m -u -n -i sh

// cd /var/lib/docker/volumes/

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let stack = make_stack();
    let _clients = builder::build_stack("cln", &docker, &stack).await?;

    log::info!("stack created!");
    signal::ctrl_c().await?;

    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    // bitcoind
    let v = "v23.0";
    let mut bitcoind = BtcImage::new("btc_1", v, &network, "sphinx");
    bitcoind.set_password("password");

    let v = "v22.11.1";
    let mut cln = ClnImage::new("cln_1", v, &network, "9735", "10009");
    cln.links(vec!["btc_1"]);

    let internal_nodes = vec![Image::Btc(bitcoind), Image::Cln(cln)];
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
