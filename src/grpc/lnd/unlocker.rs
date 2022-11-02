use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct LndUnlocker {
    pub client: reqwest::Client,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitWalletRequest {
    cipher_seed_mnemonic: Vec<String>,
    wallet_password: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct InitWalletResponse {
    admin_macaroon: Option<String>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnlockWalletRequest {
    wallet_password: String,
}

impl LndUnlocker {
    pub async fn new(port: &str, cert_path: &str) -> Result<Self> {
        let cont = std::fs::read(cert_path)?;
        let cert = reqwest::Certificate::from_pem(&cont)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .add_root_certificate(cert)
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build reqwest client");
        Ok(Self {
            url: format!("localhost:{}", port),
            client,
        })
    }
    pub async fn init_wallet(
        &self,
        password: &str,
        mnemonic: Vec<&str>,
    ) -> Result<InitWalletResponse> {
        let cipher_seed_mnemonic = mnemonic.iter().map(|s| s.to_string()).collect();
        let wallet_password = base64::encode(password.as_bytes());
        let body = InitWalletRequest {
            cipher_seed_mnemonic,
            wallet_password,
        };
        let route = format!("https://{}/v1/initwallet", self.url);
        match self
            .client
            .post(route.as_str())
            .json(&body)
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(res) => Ok(res.json().await?),
            Err(e) => Err(anyhow::anyhow!("INIT ERR {:?}", e)),
        }
    }
    pub async fn unlock_wallet(&self, password: &str) -> Result<()> {
        let wallet_password = base64::encode(password.as_bytes());
        let body = UnlockWalletRequest { wallet_password };
        let route = format!("https://{}/v1/unlockwallet", self.url);
        match self
            .client
            .post(route.as_str())
            .json(&body)
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("UNLOCK ERR {:?}", e)),
        }
    }
}
