use crate::images::LndImage;
use anyhow::Result;
use tonic_lnd::lnrpc::{GetInfoRequest, GetInfoResponse};
use tonic_lnd::tonic::Status;
use tonic_lnd::Client;

pub struct LndRPC(Client);

impl LndRPC {
    pub async fn new(proj: &str, lnd: &LndImage) -> Result<Self> {
        let address = format!("https://localhost:{}", lnd.port);
        let cert_file = format!("vol/{}/{}/tls.cert", proj, lnd.name);
        let macaroon_file = format!(
            "vol/{}/{}/data/chain/bitcoin/{}/admin.macaroon",
            proj, lnd.name, lnd.network
        );
        let client = tonic_lnd::connect(address, cert_file, macaroon_file).await?;
        Ok(Self(client))
    }

    pub async fn get_info(&mut self) -> Result<GetInfoResponse, Status> {
        let lnd = self.0.lightning();
        let response = lnd.get_info(GetInfoRequest {}).await?;
        Ok(response.into_inner())
    }
}
