extern crate bitcoincore_rpc;

use std::{thread, time::Duration};

use crate::images::BtcImage;

use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoincore_rpc_json::GetBlockchainInfoResult;

pub struct BitcoinRPC {
    btc_url: String,
    user: String,
    pass: String,
}

const INTERVAL: u64 = 1000;
const RETRY_ATTEMPTS: u8 = 5;

impl BitcoinRPC {
    pub fn new(btc: &BtcImage, url: &str, port: &str) -> Self {
        let btc_url: String = format!("{}:{}", url, port);

        BitcoinRPC {
            btc_url,
            user: btc.user.to_string(),
            pass: btc.pass.to_string(),
        }
    }

    pub fn get_info(&self) -> Result<GetBlockchainInfoResult, String> {
        if let Ok(rpc) = Client::new(
            &self.btc_url,
            Auth::UserPass(self.user.to_string(), self.pass.to_string()),
        ) {
            if let Ok(info) = rpc.get_blockchain_info() {
                return Ok(info);
            } else {
                // Try for a definite amount of time till untill it connects, else return an error.
                for _ in 0..RETRY_ATTEMPTS {
                    thread::sleep(Duration::from_millis(INTERVAL));
                    
                    if let Ok(info) = rpc.get_blockchain_info() {
                        return Ok(info);
                    }
                   
                }
                return Err("could not connect".to_string());      
            }
        } else {
            panic!("Could not initiate BTC RPC connection")
        }
    }
}
