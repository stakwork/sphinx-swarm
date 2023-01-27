use crate::cmd::{AddChannel, AddPeer, AddInvoice, PayInvoice, PayKeysend};
use crate::images::lnd::LndImage;
use anyhow::{anyhow, Result};
use jwt::ToBase64;
use tonic_lnd::lnrpc::*;
use tonic_lnd::Client;

pub struct LndRPC(Client);

impl LndRPC {
    pub async fn new(lnd: &LndImage, cert_pem: &str, macaroon: &str) -> Result<Self> {
        let address = format!("https://localhost:{}", lnd.rpc_port);
        let client = tonic_lnd::connect_from_memory(address, cert_pem, macaroon).await?;
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
                    pubkey: peer.pubkey.clone(),
                    host: peer.host.clone(),
                }),
                perm: true,
                timeout: 15,
            })
            .await?;

        Ok(response.into_inner())
    }

    pub async fn list_peers(&mut self) -> Result<ListPeersResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .list_peers(ListPeersRequest {
                ..Default::default()
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

    pub async fn get_balance(&mut self) -> Result<WalletBalanceResponse> {
        let lnd = self.0.lightning();
        let response = lnd.wallet_balance(WalletBalanceRequest {}).await?;
        Ok(response.into_inner())
    }

    pub async fn try_get_balance(&mut self) -> Result<WalletBalanceResponse> {
        for _ in 0..60 {
            if let Ok(b) = self.get_balance().await {
                return Ok(b);
            }
            sleep_ms(500).await;
        }
        Err(anyhow!("failed to get_balance"))
    }

    pub async fn new_address(&mut self) -> Result<NewAddressResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .new_address(NewAddressRequest {
                r#type: 1,
                account: "".to_string(),
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn add_invoice(&mut self, invoice: AddInvoice) -> Result<AddInvoiceResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .add_invoice(Invoice {
                value: invoice.amt_paid_sat,
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn pay_invoice(&mut self, invoice: PayInvoice)  -> Result<SendResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .send_payment_sync(SendRequest {
                payment_request: invoice.payment_request,
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }

    pub async fn pay_keysend(&mut self, keysend: PayKeysend) -> Result<SendResponse> {
        let lnd = self.0.lightning();

        let response = lnd
            .send_payment_sync(SendRequest {
                dest: base64::encode(keysend.dest).into_bytes(),
                amt: keysend.amt,
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }
}

pub async fn sleep_ms(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_millis(n)).await;
}
