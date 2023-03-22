use crate::dock;
use anyhow::Result;
use bollard::Docker;

pub struct Creds {
    pub ca_pem: String,
    pub client_pem: String,
    pub client_key: String,
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
pub async fn dl_cert(docker: &Docker, cln_name: &str, path: &str) -> Result<String> {
    let cert_bytes = dock::try_dl(docker, cln_name, path).await?;
    Ok(String::from_utf8_lossy(&cert_bytes[..]).to_string())
}
