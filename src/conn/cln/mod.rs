pub mod hsmd;
pub mod util;

use crate::images::cln::ClnImage;
use crate::secrets::hex_secret;
use crate::utils::docker_domain_tonic;
use anyhow::{anyhow, Result};
use cln_grpc::pb;
use std::collections::HashMap;
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
            match Self::new(cln, creds).await {
                Ok(c) => {
                    log::info!("connected to CLN!");
                    return Ok(c);
                }
                Err(e) => {
                    log::error!("Error connecting to CLN: {:?}", e);
                }
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
        let url = format!("https://{}:{}", grpc_url, &cln.grpc_port);
        let channel = Channel::from_shared(url)?
            .tls_config(tls)?
            .connect()
            .await?;
        let client = pb::node_client::NodeClient::new(channel);

        let mut s = Self { client };
        s.get_info().await?;

        Ok(s)
    }

    pub async fn get_info(&mut self) -> Result<pb::GetinfoResponse> {
        let response = match self.client.getinfo(pb::GetinfoRequest {}).await {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error getting node info: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn list_funds(&mut self) -> Result<pb::ListfundsResponse> {
        let response = match self
            .client
            .list_funds(pb::ListfundsRequest {
                ..Default::default()
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error listing funds: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn new_addr(&mut self) -> Result<pb::NewaddrResponse> {
        let response = match self
            .client
            .new_addr(pb::NewaddrRequest {
                ..Default::default()
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error getting new address: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn connect_peer(
        &mut self,
        id: &str,
        host: &str,
        port: &str,
    ) -> Result<pb::ConnectResponse> {
        let p = str::parse::<u32>(port)?;
        let response = match self
            .client
            .connect_peer(pb::ConnectRequest {
                id: id.to_string(),
                host: Some(host.to_string()),
                port: Some(p),
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error connecting to peer: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn list_peers(&mut self) -> Result<pb::ListpeersResponse> {
        let response = match self
            .client
            .list_peers(pb::ListpeersRequest {
                ..Default::default()
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error listing peer: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn list_peer_channels(
        &mut self,
        peer_id: Option<Vec<u8>>,
    ) -> Result<pb::ListpeerchannelsResponse> {
        let mut req = pb::ListpeerchannelsRequest::default();
        if let Some(peer_id) = peer_id {
            req.id = Some(peer_id);
        }
        let response = match self.client.list_peer_channels(req).await {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error listing peer channels: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
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
        let response = match self.client.fund_channel(req).await {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error funcding channel: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn keysend(
        &mut self,
        id: &str,
        amt: u64,
        route_hint: Option<String>,
        maxfeepercent: Option<f64>,
        exemptfee: Option<u64>,
        extratlvs: Option<HashMap<u64, Vec<u8>>>,
    ) -> Result<pb::KeysendResponse> {
        let id = hex::decode(id)?;
        let mut req = pb::KeysendRequest {
            destination: id,
            amount_msat: Some(amount(amt)),
            ..Default::default()
        };
        if let Some(mfp) = maxfeepercent {
            req.maxfeepercent = Some(mfp);
        }
        if let Some(ef) = exemptfee {
            req.exemptfee = Some(amount(ef));
        }
        if let Some(tlvs) = extratlvs {
            let mut entries: Vec<pb::TlvEntry> = Vec::new();
            for (k, value) in tlvs {
                entries.push(pb::TlvEntry { r#type: k, value })
            }
            req.extratlvs = Some(pb::TlvStream { entries });
        }
        if let Some(rh) = route_hint {
            if let Some(pos) = rh.chars().position(|c| c == ':') {
                let (pk, scid_str) = rh.split_at(pos);
                let mut scid_string = scid_str.to_string();
                scid_string.remove(0); // drop the ":"
                let mut routehints = pb::RoutehintList { hints: vec![] };
                let mut hint1 = pb::Routehint { hops: vec![] };
                let scid = scid_string.parse::<u64>()?;
                let hop1 = pb::RouteHop {
                    id: hex::decode(pk)?,
                    scid: ShortChannelId(scid).to_string(),
                    feebase: Some(amount(0)),
                    ..Default::default()
                };
                hint1.hops.push(hop1);
                routehints.hints.push(hint1);
                req.routehints = Some(routehints);
            }
        }
        log::info!("=======> CLN KEYSEND REQ: {:?}", req);
        let response = match self.client.key_send(req).await {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error executing keysend: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        let res = response.into_inner();
        log::info!("=====> KEYSEND RES: {:?}", res);
        Ok(res)
    }

    pub async fn create_invoice(&mut self, amt: u64) -> Result<pb::InvoiceResponse> {
        let response = match self
            .client
            .invoice(pb::InvoiceRequest {
                amount_msat: amount_or_any(amt),
                label: hex_secret(),
                ..Default::default()
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error creating an invoice error: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn pay(&mut self, bolt11: &str) -> Result<pb::PayResponse> {
        let response = match self
            .client
            .pay(pb::PayRequest {
                bolt11: bolt11.to_string(),
                ..Default::default()
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("Pay invoice error: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn close(&mut self, id: &str, out_addy: &str) -> Result<pb::CloseResponse> {
        let response = match self
            .client
            .close(pb::CloseRequest {
                id: id.to_string(),
                destination: Some(out_addy.to_string()),
                unilateraltimeout: Some(30),
                ..Default::default()
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error closing channel: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
        Ok(response.into_inner())
    }

    pub async fn list_invoices(
        &mut self,
        payment_hash: Option<String>,
    ) -> Result<pb::ListinvoicesResponse> {
        match payment_hash {
            Some(hash) => {
                let response = match self
                    .client
                    .list_invoices(pb::ListinvoicesRequest {
                        payment_hash: Some(hex::decode(hash)?),
                        ..Default::default()
                    })
                    .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        log::error!("Error getting invoice: {:?}", err.message());
                        return Err(anyhow!(extract_cln_error_msg(err.message())));
                    }
                };
                Ok(response.into_inner())
            }
            None => {
                let response = match self
                    .client
                    .list_invoices(pb::ListinvoicesRequest {
                        ..Default::default()
                    })
                    .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        log::error!("Error getting all invoices: {:?}", err.message());
                        return Err(anyhow!(extract_cln_error_msg(err.message())));
                    }
                };
                Ok(response.into_inner())
            }
        }
    }

    pub async fn list_pays(
        &mut self,
        payment_hash: Option<String>,
    ) -> Result<pb::ListsendpaysResponse> {
        match payment_hash {
            Some(hash) => {
                let response = match self
                    .client
                    .list_send_pays(pb::ListsendpaysRequest {
                        payment_hash: Some(hex::decode(hash)?),
                        ..Default::default()
                    })
                    .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        log::error!("Error listing a paid invoice: {:?}", err.message());
                        return Err(anyhow!(extract_cln_error_msg(err.message())));
                    }
                };
                Ok(response.into_inner())
            }
            None => {
                let response = match self
                    .client
                    .list_send_pays(pb::ListsendpaysRequest {
                        ..Default::default()
                    })
                    .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        log::error!("Error listing all paid invoices: {:?}", err.message());
                        return Err(anyhow!(extract_cln_error_msg(err.message())));
                    }
                };
                Ok(response.into_inner())
            }
        }
    }

    pub async fn get_route(&mut self, dest: &str, amt_msat: u64) -> Result<pb::GetrouteResponse> {
        let req = pb::GetrouteRequest {
            id: hex::decode(dest)?,
            amount_msat: Some(pb::Amount { msat: amt_msat }),
            riskfactor: 10,
            ..Default::default()
        };
        let response = match self.client.get_route(req).await {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error getting route: {:?}", err.message());
                return Err(anyhow!(extract_cln_error_msg(err.message())));
            }
        };
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

fn extract_cln_error_msg(input: &str) -> String {
    if let Some(start_idx) = input.find("message: ") {
        let start_idx = start_idx + "message: \"".len();
        let new_msg = &input[start_idx..];

        if let Some(end_idx) = new_msg.find("\"") {
            // Extract the message by slicing
            let message = &input[start_idx..start_idx + end_idx];
            return message.to_string();
        } else {
            return input.to_string();
        }
    } else {
        return input.to_string();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShortChannelId(pub u64);

impl ToString for ShortChannelId {
    fn to_string(&self) -> String {
        format!("{}x{}x{}", self.block(), self.txindex(), self.outnum())
    }
}

impl ShortChannelId {
    pub fn block(&self) -> u32 {
        (self.0 >> 40) as u32 & 0xFFFFFF
    }
    pub fn txindex(&self) -> u32 {
        (self.0 >> 16) as u32 & 0xFFFFFF
    }
    pub fn outnum(&self) -> u16 {
        self.0 as u16 & 0xFFFF
    }
}
