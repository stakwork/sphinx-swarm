//! Integration test for the main swarm handler flow.
//!
//! Spins up a real bitcoind container (regtest), hydrates STATE,
//! and exercises `handle()` end-to-end for GetConfig and Bitcoind::GetInfo.
//!
//! Requires Docker running locally.
//! Run with: `cargo test --test handler_test -- --nocapture`

use anyhow::Result;
use bollard::Docker;
use sphinx_swarm::cmd::{BitcoindCmd, Cmd, SwarmCmd};
use sphinx_swarm::config::{Clients, Node, Role, Stack, User};
use sphinx_swarm::dock::{
    create_and_init, create_network, remove_network, remove_volume, start_container,
    stop_and_remove,
};
use sphinx_swarm::handler::{handle, hydrate};
use sphinx_swarm::images::btc::BtcImage;
use sphinx_swarm::images::{DockerConfig, Image};

const TEST_PROJECT: &str = "test-handler";
const TEST_NETWORK: &str = "test-handler-net";
const BTC_NAME: &str = "test-btc";
const BTC_VERSION: &str = "v23.0";
const BTC_USER: &str = "testuser";
const BTC_PASS: &str = "testpass";

/// Build a minimal Stack with just one bitcoind node and an admin user.
fn make_test_stack() -> (Stack, BtcImage) {
    let mut btc = BtcImage::new(BTC_NAME, BTC_VERSION, "regtest");
    btc.set_user_password(BTC_USER, BTC_PASS);

    let node = Node::Internal(Image::Btc(btc.clone()));

    let user = User {
        id: 1,
        username: "admin".to_string(),
        pass_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        pubkey: None,
        role: Role::Admin,
    };

    let stack = Stack {
        network: "regtest".to_string(),
        nodes: vec![node],
        host: None,
        users: vec![user],
        jwt_key: "test-jwt-key".to_string(),
        ready: false, // hydrate() sets this to true
        ip: None,
        auto_update: None,
        auto_restart: None,
        custom_2b_domain: None,
        global_mem_limit: None,
        backup_services: None,
        backup_files: None,
        lightning_peers: None,
        ssl_cert_last_modified: None,
        instance_id: None,
    };

    (stack, btc)
}

/// Clean up Docker resources created during the test.
/// Best-effort — logs warnings but doesn't fail.
async fn cleanup(docker: &Docker, btc: &BtcImage) {
    let hostname = sphinx_swarm::utils::domain(&btc.name);
    if let Err(e) = stop_and_remove(docker, &hostname).await {
        eprintln!("[cleanup] stop_and_remove {}: {:?}", hostname, e);
    }
    if let Err(e) = remove_volume(docker, &hostname).await {
        eprintln!("[cleanup] remove_volume {}: {:?}", hostname, e);
    }
    if let Err(e) = remove_network(docker, Some(TEST_NETWORK)).await {
        eprintln!("[cleanup] remove_network: {:?}", e);
    }
}

#[tokio::test]
async fn test_handler_get_config() -> Result<()> {
    let _ = simple_logger::init_with_level(log::Level::Info);

    let docker = Docker::connect_with_unix_defaults()?;
    let (stack, btc) = make_test_stack();

    // Hydrate STATE with our test stack (no clients needed for GetConfig)
    let clients = Clients::default();
    hydrate(stack, clients).await;

    // GetConfig should work — it's allowed before ready and for admin users
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        &docker,
        &Some(1),
    )
    .await;

    assert!(res.is_ok(), "GetConfig failed: {:?}", res.err());

    let json = res.unwrap();
    // The response should be a valid JSON representation of the Stack
    let parsed: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(parsed["network"], "regtest");
    assert_eq!(parsed["ready"], true); // hydrate sets ready=true

    // Verify our btc node is in the config
    let nodes = parsed["nodes"].as_array().expect("nodes should be array");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["type"], "Btc");
    assert_eq!(nodes[0]["name"], BTC_NAME);

    // No Docker cleanup needed — we didn't start any containers
    // But reset STATE so the next test starts clean
    cleanup(&docker, &btc).await;

    Ok(())
}

#[tokio::test]
async fn test_handler_bitcoind_get_info() -> Result<()> {
    let _ = simple_logger::init_with_level(log::Level::Info);

    // Use local log driver (not awslogs) so containers can start without AWS credentials
    std::env::set_var("RUST_ENV", "local");

    let docker = Docker::connect_with_unix_defaults()?;
    let (stack, btc) = make_test_stack();

    // Ensure cleanup from any prior failed run
    cleanup(&docker, &btc).await;

    // 1. Create the Docker network (uses default "sphinx-swarm" network
    //    because the container's host_config hardcodes it)
    create_network(&docker, None).await?;

    // 2. Build the bitcoind container config and create/start it
    let nodes = stack.nodes.clone();
    let img = Image::Btc(btc.clone());
    let node_config = img.make_config(&nodes, &docker).await?;

    let (id_opt, need_to_start, _created_new_volume) =
        create_and_init(&docker, node_config, false).await?;
    assert!(id_opt.is_some(), "container should have been created");

    if need_to_start {
        let id = id_opt.as_ref().unwrap();
        start_container(&docker, id).await?;
        eprintln!("[test] started container {}", id);
    }

    // 3. Connect the bitcoind RPC client
    let mut clients = Clients::default();
    btc.connect_client(&mut clients).await;
    assert!(
        clients.bitcoind.contains_key(BTC_NAME),
        "bitcoind client should be registered"
    );

    // 4. Hydrate STATE
    hydrate(stack, clients).await;

    // 5. Call handle() with Bitcoind::GetInfo
    let res = handle(
        TEST_PROJECT,
        Cmd::Bitcoind(BitcoindCmd::GetInfo),
        BTC_NAME,
        &docker,
        &Some(1),
    )
    .await;

    assert!(res.is_ok(), "Bitcoind GetInfo failed: {:?}", res.err());

    let json = res.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json)?;

    // Verify we got blockchain info back
    assert_eq!(parsed["chain"], "regtest", "should be regtest chain");
    // Block count should be >= 0 (freshly started node)
    assert!(
        parsed["blocks"].as_u64().is_some(),
        "should have a blocks field"
    );
    eprintln!(
        "[test] bitcoind chain={}, blocks={}",
        parsed["chain"], parsed["blocks"]
    );

    // 6. Cleanup
    cleanup(&docker, &btc).await;

    Ok(())
}

#[tokio::test]
async fn test_handler_access_denied_without_user() -> Result<()> {
    let _ = simple_logger::init_with_level(log::Level::Info);

    let docker = Docker::connect_with_unix_defaults()?;
    let (stack, _btc) = make_test_stack();

    hydrate(stack, Clients::default()).await;

    // Calling GetConfig with no user_id should fail (access denied)
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        &docker,
        &None,
    )
    .await;

    assert!(res.is_err(), "should be access denied");
    let err_msg = res.err().unwrap().to_string();
    assert!(
        err_msg.contains("access denied"),
        "error should mention access denied, got: {}",
        err_msg
    );

    Ok(())
}
