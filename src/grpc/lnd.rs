use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
pub struct LndUnlocker(String, reqwest::Client);

#[derive(Serialize, Deserialize)]
pub struct InitWalletRequest {
    cipher_seed_mnemonic: Vec<String>,
    password: String,
}
#[derive(Serialize, Deserialize)]
pub struct InitWalletResponse {
    admin_macaroon: String,
}

impl LndUnlocker {
    pub async fn new(port: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build reqwest client");
        Ok(Self(format!("localhost:{}", port), client))
    }
    pub async fn init_wallet(&self) -> Result<()> {
        let cipher_seed_mnemonic = vec![
            "above", "hair", "trigger", "live", "innocent", "monster", "surprise", "discover",
            "art", "broccoli", "cable", "balcony", "exclude", "maple", "luggage", "dragon",
            "erosion", "basic", "census", "earn", "ripple", "gossip", "record", "monster",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let pass = "asdfasdf";
        let password = base64::encode(pass.as_bytes());
        let body = InitWalletRequest {
            cipher_seed_mnemonic,
            password,
        };
        let route = format!("https://{}/v1/initwallet", self.0);
        match self
            .1
            .post(route.as_str())
            .json(&body)
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(res) => {
                println!("RES {:?}", res);
            }
            Err(e) => {
                println!("UNLOCK ERR {:?}", e);
            }
        }
        Ok(())
    }
    // pub async fn unlock_wallet(&mut self) -> Result<UnlockWalletResponse> {
    //     let request = tonic::Request::new(UnlockWalletRequest {
    //         wallet_password: "password".as_bytes().to_vec(),
    //         ..Default::default()
    //     });
    //     let response = self.0.unlock_wallet(request).await?;
    //     Ok(response.into_inner())
    // }
}
