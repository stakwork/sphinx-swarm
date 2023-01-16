use anyhow::Result;
use bollard::container::Config;
use bollard_stubs::models::HostConfig;
use rocket::tokio::signal;
use sphinx_swarm::dock::*;
use sphinx_swarm::utils::host_port;

// docker run -it --privileged --pid=host debian nsenter -t 1 -m -u -n -i sh

// cd /var/lib/docker/volumes/

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let proj = "btc_test";
    let btc1 = btc(proj, "bitcoind");

    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("btc launched!");
    signal::ctrl_c().await?;
    stop_and_remove(&docker, &btc_id).await?;
    Ok(())
}

pub fn btc(_proj: &str, name: &str) -> Config<String> {
    let ports = vec![
        "18443".to_string(),
        "8333".to_string(),
        "28332".to_string(),
        "28333".to_string(),
    ];
    // let pwd = std::env::current_dir().unwrap_or_default();
    let domainname = format!("{}.test", name);
    let vol = format!("{}:/data/.bitcoin:rw", domainname);
    Config {
        image: Some(format!("lncm/bitcoind:v23.0")),
        hostname: Some(domainname),
        cmd: Some(vec![
            format!("-regtest=1"),
            format!("-rpcallowip=0.0.0.0/0"),
            format!("-rpcbind=0.0.0.0"),
            format!("-rpcpassword=thepass"),
            format!("-rpcport=18443"),
            format!("-rpcuser=evan"),
            format!("-server"),
        ]),
        host_config: Some(HostConfig {
            binds: Some(vec![vol]),
            port_bindings: host_port(ports),
            extra_hosts: extra_hosts(),
            ..Default::default()
        }),
        // host_config: host_config(proj, name, ports, &vol, None, None),
        ..Default::default()
    }
}

/*

docker exec -it bitcoind.test sh

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -getinfo

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass createwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass loadwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -generate 6


docker-compose -f ./src/modes/btc_test.yml --project-directory . up -d

rm -rf ./bitcoind/regtest

docker inspect bitcoind.test

*/

fn extra_hosts() -> Option<Vec<String>> {
    Some(vec!["host.docker.internal:host-gateway".to_string()])
}

/*

docker run btc WORKS when innitially loaded by the compose???
but docker AFTER cargo fails. So the cargo initialization is the problem.

/Users/evanfeenstra/code/sphinx/sphinx-swarm/bitcoind
/Users/evanfeenstra/code/sphinx/sphinx-swarm/bitcoind

created by cargo
drwxr-xr-x  12 evanfeenstra  staff   384 Jan 13 12:49 regtest
drwx------   4 evanfeenstra  staff   128 Jan 13 12:49 blocks
drwxr-xr-x   2 evanfeenstra  staff    64 Jan 13 12:49 wallets
*/
