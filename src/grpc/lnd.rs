use anyhow::Result;
use tonic::{transport::Server, Request, Response, Status};
use walletunlocker::wallet_unlocker_client::WalletUnlockerClient;
use walletunlocker::{
    InitWalletRequest, InitWalletResponse, UnlockWalletRequest, UnlockWalletResponse,
};

pub mod walletunlocker {
    tonic::include_proto!("lnrpc");
}

pub struct Lnd(WalletUnlockerClient<tonic::transport::Channel>);

impl Lnd {
    pub async fn new(port: &str) -> Result<Self> {
        let client = WalletUnlockerClient::connect(format!("https://[::1]:{}", port)).await?;
        Ok(Self(client))
    }
    pub async fn init_wallet(&mut self) -> Result<InitWalletResponse> {
        let mn = vec![
            "above", "hair", "trigger", "live", "innocent", "monster", "surprise", "discover",
            "art", "broccoli", "cable", "balcony", "exclude", "maple", "luggage", "dragon",
            "erosion", "basic", "census", "earn", "ripple", "gossip", "record", "monster",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let request = tonic::Request::new(InitWalletRequest {
            cipher_seed_mnemonic: mn,
            wallet_password: "password".as_bytes().to_vec(),
            ..Default::default()
        });
        let response = self.0.init_wallet(request).await?;
        Ok(response.into_inner())
    }
    pub async fn unlock_wallet(&mut self) -> Result<UnlockWalletResponse> {
        let request = tonic::Request::new(UnlockWalletRequest {
            wallet_password: "password".as_bytes().to_vec(),
            ..Default::default()
        });
        let response = self.0.unlock_wallet(request).await?;
        Ok(response.into_inner())
    }
}
