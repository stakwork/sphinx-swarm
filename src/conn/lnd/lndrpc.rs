use std::collections::HashMap;

use crate::cmd::{AddChannel, AddInvoice, AddPeer, PayInvoice, PayKeysend};
use crate::images::lnd::LndImage;
use crate::secrets::hex_secret_32;
use crate::utils::docker_domain;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use tonic_lnd::lnrpc::*;
use tonic_lnd::Client;

pub struct LndRPC(Client);

impl LndRPC {
    pub async fn new(lnd: &LndImage, cert_pem: &str, macaroon: &str) -> Result<Self> {
        let host = docker_domain(&lnd.name);
        let address = format!("https://{}:{}", &host, lnd.rpc_port);
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

    pub async fn pay_invoice(&mut self, invoice: PayInvoice) -> Result<SendResponse> {
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
        const LND_KEYSEND_KEY: u64 = 5482373484;

        let rando_str = hex_secret_32();
        let preimage = hex::decode(rando_str)?;

        let mut hasher = Sha256::new();
        hasher.update(preimage.clone());
        let payment_hash = hasher.finalize().to_vec();

        let mut dest_custom_records = HashMap::new();
        dest_custom_records.insert(LND_KEYSEND_KEY, preimage.clone());

        if let Some(tlvs) = keysend.tlvs {
            for (k, v) in tlvs {
                dest_custom_records.insert(k, v);
            }
        }

        let sr = SendRequest {
            dest: hex::decode(keysend.dest)?,
            amt: keysend.amt,
            payment_hash,
            dest_custom_records,
            ..Default::default()
        };
        log::info!("SendRequest {:?}", sr);

        let response = lnd.send_payment_sync(sr).await?;
        Ok(response.into_inner())
    }

    pub async fn list_payments(&mut self) -> Result<ListPaymentsResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .list_payments(ListPaymentsRequest {
                ..Default::default()
            })
            .await?;

        Ok(response.into_inner())
    }

    pub async fn list_invoices(&mut self) -> Result<ListInvoiceResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .list_invoices(ListInvoiceRequest {
                ..Default::default()
            })
            .await?;

        Ok(response.into_inner())
    }

    pub async fn list_pending_channels(&mut self) -> Result<PendingChannelsResponse> {
        let lnd = self.0.lightning();
        let response = lnd
            .pending_channels(PendingChannelsRequest {
                ..Default::default()
            })
            .await?;
        Ok(response.into_inner())
    }
}

pub async fn sleep_ms(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_millis(n)).await;
}
