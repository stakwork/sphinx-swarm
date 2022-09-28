mod api;
mod images;
mod utils;

use api::*;
use bollard::Docker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let docker = Docker::connect_with_socket_defaults().unwrap();

    let btc1 = images::btc("bitcoind");
    create_image(&docker, &btc1).await?;
    let id = create_container(&docker, btc1).await?;
    println!("ID {}", id);
    start_container(&docker, &id).await?;
    // remove_container(&docker, &id).await?;

    let cln1 = images::cln_vls("cln1", 0, vec!["bitcoind"]);
    create_image(&docker, &cln1).await?;
    let id2 = create_container(&docker, cln1).await?;
    println!("ID {}", id2);
    start_container(&docker, &id2).await?;
    // remove_container(&docker, &id2).await?;

    Ok(())
}
