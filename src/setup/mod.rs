use crate::cmd::PayKeysend;
use crate::config::{Clients, Node};
use crate::dock::*;
use crate::utils::domain;
use anyhow::Result;

const LND_TLV_LEN: usize = 1944;

pub async fn get_pubkey_cln(clients: &mut Clients, node_id: &str) -> Result<String> {
    let client = clients.cln.get_mut(node_id).unwrap();
    let info = client.get_info().await?;
    let pubkey = hex::encode(info.id);
    Ok(pubkey)
}

pub async fn get_pubkey_lnd(clients: &mut Clients, node_id: &str) -> Result<String> {
    let lnd1 = clients.lnd.get_mut(node_id).unwrap();
    let lnd1_info = lnd1.get_info().await?;
    Ok(lnd1_info.identity_pubkey)
}

pub async fn setup_cln_chans(
    clients: &mut Clients,
    nodes: &Vec<Node>,
    sender: &str,
    recip: &str,
    btc_name: &str,
) -> Result<()> {
    let cln2_pubkey = get_pubkey_cln(clients, recip).await?;
    if let Some(node) = nodes.iter().find(|n| n.name() == recip) {
        log::info!("CLN2 pubkey {}", &cln2_pubkey);
        let n = node.as_internal()?.as_cln()?;
        new_chan_from_cln1(clients, sender, recip, &cln2_pubkey, &n.peer_port, btc_name).await?;
        // keysend send
        cln_keysend_to(clients, sender, &cln2_pubkey, 1_000_000, false).await?;
        sleep(1000).await;
        cln_keysend_to(clients, sender, &cln2_pubkey, 1_000_000, false).await?;
        sleep(1000).await;
        // keysend receive
        let cln1_pubkey = get_pubkey_cln(clients, sender).await?;
        cln_keysend_to(clients, recip, &cln1_pubkey, 500_000, true).await?;
    } else {
        log::error!("{} not found!", recip);
    }
    Ok(())
}

pub async fn setup_lnd_chans(
    clients: &mut Clients,
    nodes: &Vec<Node>,
    sender: &str,
    recip: &str,
    btc_name: &str,
) -> Result<()> {
    // if !do_test_proxy() {
    //     return Ok(());
    // }
    let lnd1_pubkey = get_pubkey_lnd(clients, recip).await?;
    if let Some(node) = nodes.iter().find(|n| n.name() == recip) {
        log::info!("LND1 pubkey {}", &lnd1_pubkey);
        let n = node.as_internal()?.as_lnd()?;
        new_chan_from_cln1(clients, sender, recip, &lnd1_pubkey, &n.peer_port, btc_name).await?;
        // keysend send
        cln_keysend_to(clients, sender, &lnd1_pubkey, 1_000_000, false).await?;
        sleep(1000).await;
        // keysend send
        cln_keysend_to(clients, sender, &lnd1_pubkey, 1_000_000, false).await?;
        sleep(59000).await;
        // keysend receive
        let cln1_pubkey = get_pubkey_cln(clients, sender).await?;
        log::info!("lnd send 1");
        lnd_keysend_to(clients, recip, &cln1_pubkey, 500_000, false).await?;
        log::info!("lnd send 2");
        lnd_keysend_to(clients, recip, &cln1_pubkey, 500_000, true).await?;
    }
    Ok(())
}

pub async fn new_chan_from_cln1(
    clients: &mut Clients,
    sender_name: &str,
    peer_name: &str,
    peer_pubkey: &str,
    peer_port: &str,
    btc_name: &str,
) -> Result<()> {
    let cln1 = clients.cln.get_mut(sender_name).unwrap();

    // skip if already have a chan
    let peers = cln1.list_peers().await?;
    for p in peers
        .peers
        .iter()
        .filter(|peer| hex::encode(peer.id.clone()) == peer_pubkey)
    {
        if p.num_channels.unwrap_or(0) > 0 {
            log::info!("skipping new channel setup");
            return Ok(());
        }
    }

    let connected = cln1
        .connect_peer(peer_pubkey, &domain(peer_name), peer_port)
        .await?;
    let channel = hex::encode(connected.id);
    log::info!("CLN1 connected to {}: {}", peer_name, channel);
    let funded = cln1.try_fund_channel(&channel, 100_000_000, None).await?;
    log::info!("funded {:?}", hex::encode(funded.tx));
    let addr = cln1.new_addr().await?;

    let btcrpc = clients.bitcoind.get(btc_name).unwrap();
    let address = addr.bech32.unwrap();
    btcrpc.test_mine(6, Some(address.clone()))?;
    log::info!("mined 6 blocks to {:?}", address);

    let mut ok = false;
    log::info!("wait for channel to confirm...");
    while !ok {
        let pc = cln1.list_peer_channels(hex::decode(peer_pubkey)?).await?;
        for c in pc.channels {
            // println!("{:?}", c.status);
            if let Some(status) = c.status.get(0) {
                if status.starts_with("CHANNELD_NORMAL") {
                    log::info!("channel confirmed!!!");
                    ok = true;
                }
            }
        }
        sleep(1000).await;
    }

    Ok(())
}

pub async fn cln_keysend_to(
    clients: &mut Clients,
    sender_id: &str,
    recip_pubkey: &str,
    amt: u64,
    do_tlv: bool,
) -> Result<()> {
    let tlv_opt = if do_tlv {
        let mut tlvs = std::collections::HashMap::new();
        tlvs.insert(133773310, [9u8; 1124].to_vec()); // (1207 ok, 1208 not) 603 bytes max
        Some(tlvs)
    } else {
        None
    };

    let cln1 = clients.cln.get_mut(sender_id).unwrap();
    match cln1
        .keysend(recip_pubkey, amt, None, None, None, tlv_opt)
        .await
    {
        Ok(sent_keysend) => println!(
            "[CLN] => sent_keysend to {} {:?}",
            recip_pubkey, sent_keysend.status
        ),
        Err(e) => {
            println!("[CLN] keysend err {:?}", e)
        }
    };
    Ok(())
}

pub async fn lnd_keysend_to(
    clients: &mut Clients,
    sender_id: &str,
    recip_pubkey: &str,
    amt: u64,
    do_tlv: bool,
) -> Result<()> {
    let tlv_opt = if do_tlv {
        let mut tlvs = std::collections::HashMap::new();
        tlvs.insert(133773310, [9u8; LND_TLV_LEN].to_vec()); // (1124 ok, 1224 not)
        Some(tlvs)
    } else {
        None
    };

    let pk = PayKeysend {
        dest: recip_pubkey.to_string(),
        amt: (amt / 1000) as i64,
        tlvs: tlv_opt,
        ..Default::default()
    };
    log::info!("pk {:?}", &pk);
    let lnd1 = clients.lnd.get_mut(sender_id).unwrap();
    match lnd1.pay_keysend(pk).await {
        Ok(sent_keysend) => println!(
            "[LND] => sent_keysend to {} {:?}",
            recip_pubkey, sent_keysend
        ),
        Err(e) => {
            println!("[LND] keysend err {:?}", e)
        }
    };
    Ok(())
}
