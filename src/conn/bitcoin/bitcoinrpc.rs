extern crate bitcoincore_rpc;

use crate::images::BtcImage;
use anyhow::Result;
use bitcoincore_rpc::bitcoin::{Address, Amount, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoincore_rpc_json::{
    AddressType, GetBlockchainInfoResult, LoadWalletResult, WalletProcessPsbtResult,
};
use std::str::FromStr;

pub struct BitcoinRPC(Client);

impl BitcoinRPC {
    pub fn new(btc: &BtcImage, url: &str, port: &str) -> Result<Self> {
        let btc_url: String = format!("{}:{}", url, port);
        Ok(Self(Client::new(
            &btc_url,
            Auth::UserPass(btc.user.to_string(), btc.pass.to_string()),
        )?))
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
                self.0.load_wallet(wallet)?;
                Ok(())
            }
        }
    }

    pub fn list_wallets(&self) -> Result<Vec<String>> {
        Ok(self.0.list_wallets()?)
    }

    pub fn get_wallet_balance(&self) -> Result<Amount> {
        Ok(self.0.get_balance(Some(6), None)?)
    }

    pub fn test_mine(&self, n: u64, addr: Option<String>) -> Result<Vec<BlockHash>> {
        let address = if let Some(addy) = addr {
            Address::from_str(&addy)?
        } else {
            self.0.get_new_address(None, Some(AddressType::Bech32))?
        };
        Ok(self.0.generate_to_address(n, &address)?)
    }
}
