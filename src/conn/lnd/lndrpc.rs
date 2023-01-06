use crate::images::LndImage;
use crate::modes::stack::cmd::{AddChannel, AddPeer};
use crate::utils::wait_for_file;
use anyhow::Result;
use tonic_lnd::lnrpc::{
    ChannelPoint, ConnectPeerRequest, ConnectPeerResponse, GetInfoRequest, GetInfoResponse,
    LightningAddress, ListChannelsRequest, ListChannelsResponse, OpenChannelRequest,
};
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
        // wait 10 seconds for file to exist, or fail
        wait_for_file(&macaroon_file, 10).await?;
        let client = tonic_lnd::connect(address, cert_file, macaroon_file).await?;
        Ok(Self(client))
    }

    pub async fn get_info(&mut self) -> Result<GetInfoResponse> {
        let lnd = self.0.lightning();
        let response = lnd.get_info(GetInfoRequest {}).await?;
        Ok(response.into_inner())
    }

    pub async fn list_channels(&mut self) -> Result<ListChannelsResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .list_channels(ListChannelsRequest {
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn add_peer(&mut self, peer: AddPeer) -> Result<ConnectPeerResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .connect_peer(ConnectPeerRequest {
                addr: Some(LightningAddress {
                    pubkey: peer.pubkey,
                    host: peer.host,
                }),
                perm: true,
                timeout: 100000,
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn create_channel(&mut self, channel: AddChannel) -> Result<ChannelPoint> {
        let lnd = self.0.lightning();
        let node_pubkey = hex::decode(channel.pubkey)?;
        let response = lnd
            .open_channel_sync(OpenChannelRequest {
                sat_per_vbyte: channel.satsperbyte,
                node_pubkey,
                local_funding_amount: channel.amount,
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }
}
