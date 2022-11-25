extern crate bitcoincore_rpc;
extern crate serde;
extern crate serde_json;

use bitcoincore_rpc::{Client, Error, Result, RpcApi};


pub struct RetryClient {
    pub(crate) client: Client,
}

const INTERVAL: u64 = 1000;
const RETRY_ATTEMPTS: u8 = 10;

impl RpcApi for RetryClient {
    fn call<T: for<'a> serde::de::Deserialize<'a>>(
        &self,
        cmd: &str,
        args: &[serde_json::Value],
    ) -> Result<T> {
        for _ in 0..RETRY_ATTEMPTS {
            match self.client.call(cmd, args) {
                Ok(ret) => {
                    println!("Result   ======= ");
                    return Ok(ret)
                }
                Err(Error::JsonRpc(bitcoincore_rpc::jsonrpc::error::Error::Rpc(ref rpcerr)))
                    if rpcerr.code > 0 =>
                {
                    println!("Errror  ======= {:?}", rpcerr);
                    ::std::thread::sleep(::std::time::Duration::from_millis(INTERVAL));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        self.client.call(cmd, args)
    }
}

fn main() {}