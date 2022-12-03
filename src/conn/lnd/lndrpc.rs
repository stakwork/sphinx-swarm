use std::error::Error;
use tonic_lnd::Client;
use tonic_lnd::lnrpc::{GetInfoRequest, GetInfoResponse};
use tonic_lnd::tonic::Status;

pub struct LndRPC(Client);

impl LndRPC {
    pub async fn new(address: String, cert_file: String, macaroon_file: String) -> Self {
        let client = tonic_lnd::connect(address, cert_file, macaroon_file).await.unwrap();

        Self(client)
    }

    pub async fn get_info(&mut self, request: GetInfoRequest) -> Result<GetInfoResponse, Status> {
        let mut lnd = self.0.lightning();
        let response = lnd.get_info(request).await?;

        Ok(response.into_inner())
    }
}