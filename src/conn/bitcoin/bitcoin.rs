extern crate bitcoincore_rpc;

use std::collections::HashMap;

use crate::images::BtcImage;

use bitcoincore_rpc::{Auth, Client, RpcApi};

use reqwest::Url;

// use bitcoincore_rpc::{Auth, Client, RpcApi};
pub async fn get_info(btc: &BtcImage, url: String, port: String) {
    // let btc_url: String = format!("http://{}:{}@{}:{}", btc.user, btc.pass, url, port);
    let btc_url: String = format!("http://{}:{}", url, port);
    println!("BTC URL {}",  btc_url);


    println!("Useranem {} Pass {}", btc.user, btc.pass);

    // let url = Url::parse(&btc_url).unwrap();

    // println!("BTC URL {} {}",  btc_url, url.has_authority().to_string());

    // let mut map = HashMap::new();

    // map.insert("jsonrpc", "1.0");
    // map.insert("id", "curltext");
    // map.insert("method", "getblockchaininfo");
    // map.insert("params", "[]");


    // println!("URL {}", url);

    // let client = reqwest::Client::new();
    // let res = client
    //     .post(btc_url)
    //     .json(&map)
    //     .send();

    // println!("Result === {:?}", res.await);

    if let Ok(rpc) = Client::new(
        &btc_url,
        Auth::UserPass(btc.user.to_string(), btc.pass.to_string()),
    ) {
        println!("BTC AUTH {:?}", rpc);

        if let Ok(info) = rpc.get_blockchain_info() {
            println!("Btc Info: {:?}", info);
        }
    } else {
        panic!("Could not initiate BTC RPC connection")
    }
}
