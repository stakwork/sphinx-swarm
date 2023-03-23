pub mod util;

use anyhow::Result;
use cln_grpc::pb;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
pub use util::*;

pub struct ClnRPC {
    pub client: pb::node_client::NodeClient<Channel>,
}

impl ClnRPC {
    pub async fn new(grpc_port: &str, creds: Creds) -> Result<Self> {
        // println!("CA PEM {:?}", &creds.ca_pem);
        // println!("CLEINT PEM {:?}", &creds.client_pem);
        // println!("CLIENT KEY {:?}", &creds.client_key);

        let ca = Certificate::from_pem(&creds.ca_pem);
        let ident = Identity::from_pem(&creds.client_pem, &creds.client_key);

        let tls = ClientTlsConfig::new()
            .domain_name("cln")
            .identity(ident)
            .ca_certificate(ca);

        let url = format!("http://[::1]:{}", grpc_port);
        let channel = Channel::from_shared(url)?
            .tls_config(tls)?
            .connect()
            .await?;
        let client = pb::node_client::NodeClient::new(channel);

        Ok(Self { client })
    }

    pub async fn get_info(&mut self) -> Result<pb::GetinfoResponse> {
        let response = self.client.getinfo(pb::GetinfoRequest {}).await?;
        Ok(response.into_inner())
    }
}
