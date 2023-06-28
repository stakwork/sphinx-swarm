use super::ClnRPC;
use crate::dock;
use crate::images::cln::ClnImage;
use anyhow::Result;
use bollard::Docker;

pub async fn setup<Canceller>(
    node: &ClnImage,
    docker: &Docker,
    canceller: Canceller,
) -> Result<(ClnRPC, Option<String>)>
where
    Canceller: Fn() -> bool,
{
    let creds = collect_creds(docker, &node.name, &node.network).await?;

    let seconds_in_a_day = 86400;
    let mut client = ClnRPC::try_new(&node, &creds, seconds_in_a_day, canceller).await?;

    if &node.network != "regtest" {
        return Ok((client, None));
    }
    let funds = client.list_funds().await?;
    if funds.outputs.len() > 0 {
        return Ok((client, None));
    }
    let addy = client.new_addr().await?;
    Ok((client, Some(addy.bech32.expect("no bech32 address"))))
}

#[derive(Debug)]
pub struct Creds {
    pub ca_pem: Vec<u8>,
    pub client_pem: Vec<u8>,
    pub client_key: Vec<u8>,
}
pub async fn collect_creds(docker: &Docker, cln_name: &str, network: &str) -> Result<Creds> {
    let root = format!("/root/.lightning/{}/", network);
    let ca_pem = dl_cert(docker, cln_name, &format!("{}ca.pem", &root)).await?;
    let client_pem = dl_cert(docker, cln_name, &format!("{}client.pem", &root)).await?;
    let client_key = dl_cert(docker, cln_name, &format!("{}client-key.pem", &root)).await?;

    Ok(Creds {
        ca_pem,
        client_pem,
        client_key,
    })
}

// PEM encoded (with -----BEGIN CERTIFICATE----- and -----END CERTIFICATE-----)
pub async fn dl_cert(docker: &Docker, cln_name: &str, path: &str) -> Result<Vec<u8>> {
    Ok(dock::try_dl(docker, cln_name, path).await?)
}

pub async fn sleep_ms(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_millis(n)).await;
}
