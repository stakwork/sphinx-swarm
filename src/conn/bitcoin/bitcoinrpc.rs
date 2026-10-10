extern crate bitcoincore_rpc;

use crate::images::btc::BtcImage;
use anyhow::Result;
use bitcoincore_rpc::bitcoin::{Address, BlockHash, Txid};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoincore_rpc_json::{AddressType, GetBlockchainInfoResult};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub struct BitcoinRPC(Client);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionStatus {
    pub confirmed: bool,
    pub block_height: Option<u64>,
}

impl BitcoinRPC {
    pub fn new(btc: &BtcImage, url: &str, port: &str) -> Result<Self> {
        let btc_url: String = format!("{}:{}", url, port);
        Ok(Self(Client::new(
            &btc_url,
            Auth::UserPass(
                btc.user.clone().unwrap_or("".to_string()),
                btc.pass.clone().unwrap_or("".to_string()),
            ),
        )?))
    }

    pub async fn new_and_create_wallet(btc: &BtcImage, url: &str, port: &str) -> Result<Self> {
        let c = BitcoinRPC::new(btc, url, port)?;
        sleep(1).await;
        c.create_or_load_wallet()?;
        Ok(c)
    }

    pub fn get_info(&self) -> Result<GetBlockchainInfoResult> {
        Ok(self.0.get_blockchain_info()?)
    }

    pub fn create_or_load_wallet(&self) -> Result<()> {
        let wallet = "wallet";
        // try to create, otherwise load
        match self.0.create_wallet(wallet, None, None, None, None) {
            Ok(_) => Ok(()),
            Err(_) => {
                let _ = self.0.load_wallet(wallet);
                Ok(())
            }
        }
    }

    pub fn load_wallet(&self) -> Result<()> {
        let wallet = "wallet";
        let _ = self.0.load_wallet(wallet);
        Ok(())
    }

    pub fn get_wallet_balance(&self) -> Result<f64> {
        Ok(self.0.get_balance(Some(6), None)?.as_btc())
    }

    pub fn get_transaction_status(&self, txid: String) -> Result<TransactionStatus> {
        let txid = Txid::from_str(&txid)?;
        let tx = self.0.get_raw_transaction_info(&txid, None)?;

        let Some(block_hash) = tx.blockhash else {
            return Ok(TransactionStatus {
                confirmed: false,
                block_height: None,
            });
        };

        let header = self.0.get_block_header_info(&block_hash)?;
        Ok(TransactionStatus {
            confirmed: tx.confirmations.unwrap_or(0) > 0,
            block_height: Some(header.height as u64),
        })
    }

    pub fn test_mine(&self, n: u64, addr: Option<String>) -> Result<Vec<BlockHash>> {
        let address = if let Some(addy) = addr {
            Address::from_str(&addy)?
        } else {
            self.0.get_new_address(None, Some(AddressType::Bech32))?
        };
        let _ = self.load_wallet();
        Ok(self.0.generate_to_address(n, &address)?)
    }
}

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
