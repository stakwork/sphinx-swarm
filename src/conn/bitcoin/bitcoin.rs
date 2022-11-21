extern crate bitcoincore_rpc;

use bitcoincore_rpc::{Auth, Client, RpcApi};
use crate::images::{BtcImage};

pub fn get_info(btc: &BtcImage, url: String, port: String) {
    println!("User name {} Password {}", btc.user, btc.pass);
    let btc_url: String = format!("{}:{}", url, port);
    
    let rpc = Client::new(
        &btc_url,
        Auth::UserPass(
            btc.user.to_string(),
            btc.pass.to_string(),
        ),
    )
    .unwrap();

    let info = rpc.get_blockchain_info().unwrap();
    println!("Btc Info: {:?}", info);
}
