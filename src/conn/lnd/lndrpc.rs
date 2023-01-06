use crate::cmd::{AddPeer, AddChannel};
use crate::images::LndImage;
use anyhow::Result;
use serde::Serialize;
use tonic_lnd::lnrpc::{
    Channel, 
    GetInfoRequest, 
    GetInfoResponse, 
    ListChannelsRequest, 
    ListChannelsResponse, 
    ConnectPeerResponse, 
    ConnectPeerRequest, 
    LightningAddress,
    OpenChannelRequest, OpenStatusUpdate,
};
use tonic_lnd::tonic::{Status, Streaming};
use tonic_lnd::Client;

pub struct LndRPC(Client);

#[derive(Serialize)]
pub struct LndChannel {
    active: bool,
    remote_pubkey: String,
    channel_point: String,
    chan_id: u64,
    capacity: i64,
    local_balance: i64,
    remote_balance: i64,
    commit_fee: i64,
    commit_weight: i64,
    fee_per_kw: i64,
    unsettled_balance: i64,
    total_satoshis_sent: i64,
    total_satoshis_received: i64,
    num_updates: u64,
    // pending_htlcs: Vec<Htlc>,
    csv_delay: u32,
    private: bool,
    initiator: bool,
    chan_status_flags: String,
    local_chan_reserve_sat: i64,
    remote_chan_reserve_sat: i64,
    static_remote_key: bool,
    commitment_type: i32,
    lifetime: i64,
    uptime: i64,
    close_address: String,
    push_amount_sat: u64,
    thaw_height: u32,
}

impl LndChannel {
    pub fn convert_to_json(chan: Channel) -> Self {
        Self {
            active: chan.active,
            remote_pubkey: chan.remote_pubkey,
            channel_point: chan.channel_point,
            chan_id: chan.chan_id,
            capacity: chan.capacity,
            local_balance: chan.local_balance,
            remote_balance: chan.remote_balance,
            commit_fee: chan.commit_fee,
            commit_weight: chan.commit_weight,
            fee_per_kw: chan.fee_per_kw,
            unsettled_balance: chan.unsettled_balance,
            total_satoshis_sent: chan.total_satoshis_sent,
            total_satoshis_received: chan.total_satoshis_received,
            num_updates: chan.num_updates,
            // pending_htlcs: Vec<Htlc>,
            csv_delay: chan.csv_delay,
            private: chan.private,
            initiator:chan.initiator,
            chan_status_flags: chan.chan_status_flags,
            local_chan_reserve_sat: chan.local_chan_reserve_sat,
            remote_chan_reserve_sat: chan.remote_chan_reserve_sat,
            static_remote_key: chan.static_remote_key,
            commitment_type: chan.commitment_type,
            lifetime: chan.lifetime,
            uptime: chan.uptime,
            close_address: chan.close_address,
            push_amount_sat: chan.push_amount_sat,
            thaw_height: chan.thaw_height
        }
    }
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

    pub async fn add_peer(&mut self, peer: AddPeer) -> Result<ConnectPeerResponse, Status> {
        let lnd = self.0.lightning();
        let response = lnd.connect_peer(ConnectPeerRequest {
            addr: Some(LightningAddress {
                pubkey: peer.pubkey,
                host: peer.host
            }),
            perm: true,
            timeout: 100000
        }).await?;
        Ok(response.into_inner())
    }

    pub async fn create_channel(&mut self, channel: AddChannel) -> Result<Streaming<OpenStatusUpdate>, Status> {
        let lnd = self.0.lightning();
        let response = lnd.open_channel(OpenChannelRequest {
            sat_per_vbyte: channel.satsperbyte,
            node_pubkey: channel.pubkey.into_bytes(),
            local_funding_amount: channel.amount,
            push_sat: 10,
            target_conf: 6,
            private: false,
            min_htlc_msat: 5,
            remote_csv_delay: 0,
            min_confs: 6,
            spend_unconfirmed: false,
            funding_shim: None,
            remote_max_value_in_flight_msat: 1000,
            remote_max_htlcs: 5,
            max_local_csv: 6,
            commitment_type: 1,
            zero_conf: false,
            scid_alias: true,
            close_address: "".to_string(),
            node_pubkey_string: "".to_string(),
            sat_per_byte: 0,
        }).await?;
        Ok(response.into_inner())
    }
}
