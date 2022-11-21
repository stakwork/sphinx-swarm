extern crate bitcoincore_rpc;

use bitcoincore_rpc::{Auth, Client, RpcApi};
use crate::images::{BtcImage};

pub fn get_info(btc: &BtcImage, url: String, port: String) {
    let btc_url: String = format!("{}:{}", url, port);

    if let Ok(rpc) = Client::new(
        &btc_url,
        Auth::UserPass(
            btc.user.to_string(),
            btc.pass.to_string(),
        ),
    )  {
        if let Ok(info) = rpc.get_blockchain_info()  {
            println!("Btc Info: {:?}", info);
        }
    } else {
        panic!("Could not initiate BTC RPC connection")
    }
    
}
