use crate::images::LndImage;
use anyhow::Result;
use serde::Serialize;
use tonic_lnd::lnrpc::{
    Channel, GetInfoRequest, GetInfoResponse, Htlc, ListChannelsRequest, ListChannelsResponse,
};
use tonic_lnd::tonic::Status;
use tonic_lnd::Client;

pub struct LndRPC(Client);

#[derive(Serialize)]
pub struct LndChannel {
    pub(crate) active: bool,
    pub(crate) remote_pubkey: String,
    // channel_point: String,
    // chan_id: u64,
    // capacity: i64,
    // local_balance: i64,
    // remote_balance: i64,
    // commit_fee: i64,
    // commit_weight: i64,
    // fee_per_kw: i64,
    // unsettled_balance: i64,
    // total_satoshis_sent: i64,
    // total_satoshis_received: i64,
    // num_updates: u64,
    // // pending_htlcs: Vec<Htlc>,
    // csv_delay: u32,
    // private: bool,
    // initiator: bool,
    // chan_status_flags: String,
    // local_chan_reserve_sat: i64,
    // remote_chan_reserve_sat: i64,
    // static_remote_key: bool,
    // commitment_type: i32,
    // lifetime: i64,
    // uptime: i64,
    // close_address: String,
    // push_amount_sat: u64,
    // thaw_height: u32,
}

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

    pub async fn list_channels(&mut self) -> Result<ListChannelsResponse, Status> {
        let lnd = self.0.lightning();
        let response = lnd
            .list_channels(ListChannelsRequest {
                active_only: false,
                inactive_only: false,
                public_only: false,
                private_only: false,
                peer: vec![],
            })
            .await?;
        Ok(response.into_inner())
    }
}
