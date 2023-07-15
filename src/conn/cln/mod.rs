pub mod util;

use crate::images::cln::ClnImage;
use crate::secrets::hex_secret;
use crate::utils::docker_domain_tonic;
use anyhow::{anyhow, Result};
use cln_grpc::pb;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
pub use util::*;

pub struct ClnRPC {
    pub client: pb::node_client::NodeClient<Channel>,
}

impl ClnRPC {
    // try new a few times
    pub async fn try_new<Canceller>(
        cln: &ClnImage,
        creds: &Creds,
        i: usize,
        canceller: Canceller,
    ) -> Result<Self>
    where
        Canceller: Fn() -> bool,
    {
        for iteration in 0..i {
            if let Ok(c) = Self::new(cln, creds).await {
                return Ok(c);
            }
            sleep_ms(1000).await;
            if canceller() {
                break;
            }
            log::info!("retry CLN connect {}", iteration);
        }
        Err(anyhow!("could not connect to CLN"))
    }
    pub async fn new(cln: &ClnImage, creds: &Creds) -> Result<Self> {
        // println!("CA PEM {:?}", &creds.ca_pem);
        // println!("CLEINT PEM {:?}", &creds.client_pem);
        // println!("CLIENT KEY {:?}", &creds.client_key);

        let ca = Certificate::from_pem(&creds.ca_pem);
        let ident = Identity::from_pem(&creds.client_pem, &creds.client_key);

        let tls = ClientTlsConfig::new()
            .domain_name("cln")
            .identity(ident)
            .ca_certificate(ca);

        let grpc_url = docker_domain_tonic(&cln.name);
        let url = format!("http://{}:{}", grpc_url, &cln.grpc_port);
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

    pub async fn list_funds(&mut self) -> Result<pb::ListfundsResponse> {
        let response = self
            .client
            .list_funds(pb::ListfundsRequest {
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn new_addr(&mut self) -> Result<pb::NewaddrResponse> {
        let response = self
            .client
            .new_addr(pb::NewaddrRequest {
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn connect_peer(
        &mut self,
        id: &str,
        host: &str,
        port: &str,
    ) -> Result<pb::ConnectResponse> {
        let p = str::parse::<u32>(port)?;
        let response = self
            .client
            .connect_peer(pb::ConnectRequest {
                id: id.to_string(),
                host: Some(host.to_string()),
                port: Some(p),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn list_peers(&mut self) -> Result<pb::ListpeersResponse> {
        let response = self
            .client
            .list_peers(pb::ListpeersRequest {
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn try_fund_channel(
        &mut self,
        id: &str,
        amt: u64,
        satsperbyte: Option<u32>,
    ) -> Result<pb::FundchannelResponse> {
        for iteration in 0..100 {
            if let Ok(c) = self.fund_channel(id, amt, satsperbyte).await {
                return Ok(c);
            }
            sleep_ms(5000).await;
            log::info!("retry fund channel {}", iteration);
        }
        Err(anyhow!("could not fund channel - probably not synced"))
    }

    pub async fn fund_channel(
        &mut self,
        id: &str,
        amt: u64,
        satsperbyte: Option<u32>,
    ) -> Result<pb::FundchannelResponse> {
        let id = hex::decode(id)?;
        let mut req = pb::FundchannelRequest {
            id: id,
            amount: amount_or_all(amt),
            ..Default::default()
        };
        if let Some(spvb) = satsperbyte {
            if spvb > 0 {
                let perkw = spvb * 1000;
                req.feerate = Some(pb::Feerate {
                    style: Some(pb::feerate::Style::Perkb(perkw)),
                });
            }
        }
        let response = self.client.fund_channel(req).await?;
        Ok(response.into_inner())
    }

    pub async fn keysend(&mut self, id: &str, amt: u64) -> Result<pb::KeysendResponse> {
        let id = hex::decode(id)?;
        // let mut routehints = pb::RoutehintList { hints: vec![] };
        // let mut hint1 = pb::Routehint { hops: vec![] };
        // let hop1 = pb::RouteHop {
        //     ..Default::default()
        // };
        // hint1.hops.push(hop1);
        // routehints.hints.push(hint1);
        let response = self
            .client
            .key_send(pb::KeysendRequest {
                destination: id,
                amount_msat: Some(amount(amt)),
                // routehints: Some(routehints),
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn create_invoice(&mut self, amt: u64) -> Result<pb::InvoiceResponse> {
        let response = self
            .client
            .invoice(pb::InvoiceRequest {
                amount_msat: amount_or_any(amt),
                label: hex_secret(),
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn pay(&mut self, bolt11: &str) -> Result<pb::PayResponse> {
        let response = self
            .client
            .pay(pb::PayRequest {
                bolt11: bolt11.to_string(),
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn close(&mut self, id: &str, out_addy: &str) -> Result<pb::CloseResponse> {
        let response = self
            .client
            .close(pb::CloseRequest {
                id: id.to_string(),
                destination: Some(out_addy.to_string()),
                unilateraltimeout: Some(30),
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn list_invoices(&mut self) -> Result<pb::ListinvoicesResponse> {
        let response = self
            .client
            .list_invoices(pb::ListinvoicesRequest {
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn list_pays(&mut self) -> Result<pb::ListsendpaysResponse> {
        let response = self
            .client
            .list_send_pays(pb::ListsendpaysRequest {
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }
}

fn amount_or_any(msat: u64) -> Option<pb::AmountOrAny> {
    Some(pb::AmountOrAny {
        value: Some(pb::amount_or_any::Value::Amount(amount(msat))),
    })
}
fn amount_or_all(msat: u64) -> Option<pb::AmountOrAll> {
    Some(pb::AmountOrAll {
        value: Some(pb::amount_or_all::Value::Amount(amount(msat))),
    })
}
fn amount(msat: u64) -> pb::Amount {
    pb::Amount { msat }
}
