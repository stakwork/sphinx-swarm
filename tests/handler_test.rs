//! Integration test for the main swarm handler flow.
//!
//! Spins up a real bitcoind container (regtest), hydrates STATE,
//! and exercises `handle()` end-to-end for GetConfig and Bitcoind::GetInfo.
//!
//! Requires Docker running locally.
//! Run with: `cargo test --test handler_test -- --nocapture`
//!
//! Tests run sequentially (not in parallel) because they share the global STATE mutex.

use anyhow::Result;
use bollard::Docker;
use sphinx_swarm::cmd::{BitcoindCmd, Cmd, SwarmCmd};
use sphinx_swarm::config::{Clients, Node, Role, Stack, User};
use sphinx_swarm::dock::{
    create_and_init, create_network, remove_volume, start_container, stop_and_remove,
};
use sphinx_swarm::handler::{handle, hydrate};
use sphinx_swarm::images::btc::BtcImage;
use sphinx_swarm::images::{DockerConfig, Image};
use std::sync::Once;

const TEST_PROJECT: &str = "test-handler";
const BTC_NAME: &str = "test-btc";
const BTC_VERSION: &str = "v23.0";
const BTC_USER: &str = "testuser";
const BTC_PASS: &str = "testpass";

static INIT: Once = Once::new();

/// One-time setup: logger + env vars needed for local Docker.
fn init() {
    INIT.call_once(|| {
        let _ = simple_logger::init_with_level(log::Level::Info);
        // Use local log driver (not awslogs) so containers start without AWS credentials
        std::env::set_var("RUST_ENV", "local");
    });
}

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
async fn cleanup(docker: &Docker) {
    let hostname = sphinx_swarm::utils::domain(BTC_NAME);
    if let Err(e) = stop_and_remove(docker, &hostname).await {
        eprintln!("[cleanup] stop_and_remove {}: {:?}", hostname, e);
    }
    if let Err(e) = remove_volume(docker, &hostname).await {
        eprintln!("[cleanup] remove_volume {}: {:?}", hostname, e);
    }
}

/// Single sequential test entrypoint.
///
/// All handler tests share the global `STATE` mutex, so they must not
/// run concurrently. We use one `#[tokio::test]` that calls each
/// sub-test in order.
#[tokio::test]
async fn test_handler_flow() -> Result<()> {
    init();

    let docker = Docker::connect_with_unix_defaults()?;

    // Clean up from any prior failed run
    cleanup(&docker).await;

    // --- Test 1: GetConfig ---
    eprintln!("\n=== test_get_config ===");
    test_get_config(&docker).await?;

    // --- Test 2: Access denied without user ---
    eprintln!("\n=== test_access_denied ===");
    test_access_denied(&docker).await?;

    // --- Test 3: Bitcoind GetInfo (needs real container) ---
    eprintln!("\n=== test_bitcoind_get_info ===");
    test_bitcoind_get_info(&docker).await?;

    Ok(())
}

/// GetConfig returns valid stack data for an admin user.
async fn test_get_config(docker: &Docker) -> Result<()> {
    let (stack, _btc) = make_test_stack();

    // Hydrate STATE with our test stack (no clients needed for GetConfig)
    hydrate(stack, Clients::default()).await;

    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        docker,
        &Some(1),
    )
    .await;

    assert!(res.is_ok(), "GetConfig failed: {:?}", res.err());

    let json = res.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(parsed["network"], "regtest");
    assert_eq!(parsed["ready"], true); // hydrate sets ready=true

    // Verify our btc node is in the config
    let nodes = parsed["nodes"].as_array().expect("nodes should be array");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["type"], "Btc");
    assert_eq!(nodes[0]["name"], BTC_NAME);

    eprintln!("[pass] GetConfig returned valid stack with {} node(s)", nodes.len());
    Ok(())
}

/// GetConfig with no user_id is rejected.
async fn test_access_denied(docker: &Docker) -> Result<()> {
    let (stack, _btc) = make_test_stack();
    hydrate(stack, Clients::default()).await;

    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        docker,
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

    eprintln!("[pass] access denied for unauthenticated request");
    Ok(())
}

/// Full end-to-end: start a real bitcoind container, connect RPC, call GetInfo through handler.
async fn test_bitcoind_get_info(docker: &Docker) -> Result<()> {
    let (stack, btc) = make_test_stack();

    // 1. Create the Docker network (uses default "sphinx-swarm" because host_config hardcodes it)
    create_network(docker, None).await?;

    // 2. Build the bitcoind container config and create/start it
    let nodes = stack.nodes.clone();
    let img = Image::Btc(btc.clone());
    let node_config = img.make_config(&nodes, docker).await?;

    let (id_opt, need_to_start, _created_new_volume) =
        create_and_init(docker, node_config, false).await?;
    assert!(id_opt.is_some(), "container should have been created");

    if need_to_start {
        let id = id_opt.as_ref().unwrap();
        start_container(docker, id).await?;
        eprintln!("[test] started container {}", id);
    }

    // 3. Connect the bitcoind RPC client
    let mut clients = Clients::default();
    btc.connect_client(&mut clients).await;
    assert!(
        clients.bitcoind.contains_key(BTC_NAME),
        "bitcoind client should be registered after connect_client"
    );

    // 4. Hydrate STATE with stack + live clients
    hydrate(stack, clients).await;

    // 5. Call handle() with Bitcoind::GetInfo
    let res = handle(
        TEST_PROJECT,
        Cmd::Bitcoind(BitcoindCmd::GetInfo),
        BTC_NAME,
        docker,
        &Some(1),
    )
    .await;

    assert!(res.is_ok(), "Bitcoind GetInfo failed: {:?}", res.err());

    let json = res.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json)?;

    // Verify we got blockchain info back
    assert_eq!(parsed["chain"], "regtest", "should be regtest chain");
    assert!(
        parsed["blocks"].as_u64().is_some(),
        "should have a blocks field"
    );
    eprintln!(
        "[pass] bitcoind chain={}, blocks={}",
        parsed["chain"], parsed["blocks"]
    );

    // 6. Cleanup
    cleanup(docker).await;

    Ok(())
}
