extern crate bitcoincore_rpc;

use std::{thread, time::Duration};

use crate::images::BtcImage;

use bitcoincore_rpc::{Auth, Client, RpcApi};

pub async fn get_info(btc: &BtcImage, url: String, port: String) {
    let btc_url: String = format!("{}:{}", url, port);


    if let Ok(rpc) = Client::new(
        &btc_url,
        Auth::UserPass(btc.user.to_string(), btc.pass.to_string()),
    ) {
        println!("BTC AUTH {:?}", rpc);

        if let Ok(info) = rpc.get_blockchain_info() {
            println!("Btc Info: {:?}", info);
        } else {
            // Try again till it connects
            // println!("Btc Error ===");
            // thread::sleep(Duration::from_millis(1000));

            // let result = rpc.get_blockchain_info();
            // println!("Result === {:?}", result);
        }
    } else {
        panic!("Could not initiate BTC RPC connection")
    }
}
