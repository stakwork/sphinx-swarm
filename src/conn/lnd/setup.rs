use crate::config::Clients;
use crate::conn::lnd::lndrpc::LndRPC;
use crate::conn::lnd::utils::{dl_cert, dl_macaroon};
use crate::images;
use anyhow::{anyhow, Result};
use bollard::Docker;
use images::lnd::{to_lnd_network, LndImage};

// returns LndRPC client and address if test mine needed
pub async fn lnd_clients(docker: &Docker, lnd_node: &LndImage) -> Result<(LndRPC, Option<String>)> {
    let cert_path = "/home/.lnd/tls.cert";
    let cert = dl_cert(docker, &lnd_node.name, cert_path).await?;
    let netwk = to_lnd_network(lnd_node.network.as_str());
    let macpath = format!("/home/.lnd/data/chain/bitcoin/{}/admin.macaroon", netwk);
    let mac = dl_macaroon(docker, &lnd_node.name, &macpath).await?;
    let mut client = LndRPC::new(lnd_node, &cert, &mac)
        .await
        .map_err(|e| anyhow!("LndRPC::new failed: {}", e))?;
    if &lnd_node.network != "regtest" {
        return Ok((client, None));
    }
    let bal = client.try_get_balance().await?;
    if bal.confirmed_balance > 0 {
        return Ok((client, None));
    }
    let addy = client.new_address().await?;
    Ok((client, Some(addy.address)))
}

pub fn test_mine_if_needed(test_mine_addy: Option<String>, btc_name: &str, clients: &mut Clients) {
    if let Some(addy) = test_mine_addy {
        log::info!("mining 101 blocks to LND address {}", addy);
        if let Some(btcrpc) = clients.bitcoind.get(btc_name) {
            if let Err(e) = btcrpc.test_mine(101, Some(addy)) {
                log::error!("failed to test mine {}", e);
            } else {
                log::info!("blocks mined!");
            }
        }
    }
}
