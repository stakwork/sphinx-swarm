use crate::api::*;
use crate::images;
use anyhow::Result;
use bollard::Docker;

pub async fn run(docker: &Docker) -> Result<()> {
    let btc1 = images::btc("bitcoind");
    let _id = create_and_start(docker, btc1).await?;
    // remove_container(&docker, &id).await?;
    for i in 1..3 {
        let name = format!("cln{}", i);
        let cln1 = images::cln_vls(name.as_str(), i, vec!["bitcoind"]);
        create_and_start(docker, cln1).await?;
    }
    Ok(())
}
