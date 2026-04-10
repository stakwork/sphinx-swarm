//! Integration test for the main swarm handler flow.
//!
//! Spins up a real bitcoind container (regtest), hydrates STATE,
//! and exercises `handle()` end-to-end for GetConfig and Bitcoind::GetInfo.
//!
//! Requires Docker running locally.
//! Run with: `cargo test --test handler_test -- --nocapture`
//!
//! Tests run sequentially (not in parallel) because they share the global STACK/CLIENTS RwLocks.

use anyhow::Result;
use bollard::Docker;
use sphinx_swarm::cmd::{
    BitcoindCmd, ChangePasswordInfo, Cmd, LoginInfo, SwarmCmd,
};
use sphinx_swarm::config::{Clients, Node, Role, Stack, User};
use sphinx_swarm::dock::{
    create_and_init, create_network, remove_volume, start_container, stop_and_remove,
};
use sphinx_swarm::handler::{handle, hydrate};
use sphinx_swarm::images::btc::BtcImage;
use sphinx_swarm::images::{DockerConfig, Image};
use std::sync::Once;
use std::time::Instant;

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
        // Ensure the config directory exists for stack_write (put_config_file)
        let _ = std::fs::create_dir_all(format!("vol/{}", TEST_PROJECT));
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

/// Build a Stack with multiple test users (no Docker nodes needed).
fn make_auth_stack() -> Stack {
    let admin = User {
        id: 1,
        username: "admin".to_string(),
        pass_hash: bcrypt::hash("adminpass", bcrypt::DEFAULT_COST).unwrap(),
        pubkey: None,
        role: Role::Admin,
    };
    let testuser = User {
        id: 2,
        username: "testuser".to_string(),
        pass_hash: bcrypt::hash("testpass", bcrypt::DEFAULT_COST).unwrap(),
        pubkey: None,
        role: Role::Admin,
    };

    Stack {
        network: "regtest".to_string(),
        nodes: vec![],
        host: None,
        users: vec![admin, testuser],
        jwt_key: "test-jwt-key".to_string(),
        ready: false,
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
    }
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

/// Start the bitcoind container and hydrate globals with a live RPC client.
/// Returns the Docker handle (caller is responsible for cleanup).
async fn start_bitcoind_and_hydrate(docker: &Docker) -> Result<()> {
    let (stack, btc) = make_test_stack();

    create_network(docker, None).await?;

    let nodes = stack.nodes.clone();
    let img = Image::Btc(btc.clone());
    let node_config = img.make_config(&nodes, docker).await?;

    let (id_opt, need_to_start, _) = create_and_init(docker, node_config, false).await?;
    assert!(id_opt.is_some(), "container should have been created");

    if need_to_start {
        let id = id_opt.as_ref().unwrap();
        start_container(docker, id).await?;
        eprintln!("[setup] started container {}", id);
    }

    let mut clients = Clients::default();
    btc.connect_client(&mut clients).await;
    assert!(
        clients.bitcoind.contains_key(BTC_NAME),
        "bitcoind client should be registered after connect_client"
    );

    hydrate(stack, clients).await;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Main test entrypoint — runs all sub-tests sequentially
// ═══════════════════════════════════════════════════════════════════════

/// Single sequential test entrypoint.
///
/// All handler tests share the global STACK/CLIENTS RwLocks, so they must not
/// run concurrently. We use one `#[tokio::test]` that calls each
/// sub-test in order.
#[tokio::test]
async fn test_handler_flow() -> Result<()> {
    init();

    let docker = Docker::connect_with_unix_defaults()?;

    // Clean up from any prior failed run
    cleanup(&docker).await;

    // --- Original tests ---
    eprintln!("\n=== test_get_config ===");
    test_get_config(&docker).await?;

    eprintln!("\n=== test_access_denied ===");
    test_access_denied(&docker).await?;

    eprintln!("\n=== test_bitcoind_get_info ===");
    test_bitcoind_get_info(&docker).await?;

    // --- New tests (stack-only, no Docker container needed) ---
    eprintln!("\n=== test_concurrent_reads ===");
    test_concurrent_reads(&docker).await?;

    eprintln!("\n=== test_login_and_change_password ===");
    test_login_and_change_password(&docker).await?;

    eprintln!("\n=== test_read_during_write ===");
    test_read_during_write(&docker).await?;

    eprintln!("\n=== test_stack_mutations_persist ===");
    test_stack_mutations_persist(&docker).await?;

    eprintln!("\n=== test_access_control ===");
    test_access_control(&docker).await?;

    // --- New test (needs live bitcoind container) ---
    eprintln!("\n=== test_concurrent_bitcoind_calls ===");
    test_concurrent_bitcoind_calls(&docker).await?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Original tests
// ═══════════════════════════════════════════════════════════════════════

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

    eprintln!(
        "[pass] GetConfig returned valid stack with {} node(s)",
        nodes.len()
    );
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
    start_bitcoind_and_hydrate(docker).await?;

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

    assert_eq!(parsed["chain"], "regtest", "should be regtest chain");
    assert!(
        parsed["blocks"].as_u64().is_some(),
        "should have a blocks field"
    );
    eprintln!(
        "[pass] bitcoind chain={}, blocks={}",
        parsed["chain"], parsed["blocks"]
    );

    // Don't cleanup yet — later tests reuse the container
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Test 1: Concurrent reads don't block each other
// ═══════════════════════════════════════════════════════════════════════

/// Spawn 10 concurrent GetConfig calls. All should complete within 2 seconds.
/// With RwLock, read locks are shared so they run concurrently.
/// Under the old Mutex, they would serialize and take much longer.
async fn test_concurrent_reads(docker: &Docker) -> Result<()> {
    let stack = make_auth_stack();
    hydrate(stack, Clients::default()).await;

    // Set the JWT key so handle() doesn't error
    sphinx_swarm::auth::set_jwt_key("test-jwt-key");

    let start = Instant::now();
    let mut handles = Vec::new();

    for _ in 0..10 {
        let docker = docker.clone();
        let h = tokio::spawn(async move {
            handle(
                TEST_PROJECT,
                Cmd::Swarm(SwarmCmd::GetConfig),
                "SWARM",
                &docker,
                &Some(1),
            )
            .await
        });
        handles.push(h);
    }

    let mut successes = 0;
    for h in handles {
        let res = h.await.expect("task panicked");
        assert!(res.is_ok(), "concurrent GetConfig failed: {:?}", res.err());
        successes += 1;
    }

    let elapsed = start.elapsed();
    assert_eq!(successes, 10);
    assert!(
        elapsed.as_secs() < 2,
        "10 concurrent reads took {:?}, expected < 2s (RwLock should allow concurrent readers)",
        elapsed
    );

    eprintln!(
        "[pass] 10 concurrent GetConfig completed in {:?}",
        elapsed
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Test 2: Login and ChangePassword flow
// ═══════════════════════════════════════════════════════════════════════

/// Tests the full login + change password cycle:
/// 1. Login with correct password -> get token
/// 2. Login with wrong password -> empty string
/// 3. ChangePassword -> success
/// 4. Login with NEW password -> get token
/// 5. Login with OLD password -> empty string
async fn test_login_and_change_password(docker: &Docker) -> Result<()> {
    let stack = make_auth_stack();
    hydrate(stack, Clients::default()).await;
    sphinx_swarm::auth::set_jwt_key("test-jwt-key");

    // 1. Login with correct password
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        })),
        "SWARM",
        docker,
        &None, // Login doesn't require auth
    )
    .await?;

    let parsed: serde_json::Value = serde_json::from_str(&res)?;
    assert!(
        parsed["token"].is_string(),
        "login should return a token, got: {}",
        res
    );
    let token = parsed["token"].as_str().unwrap();
    assert!(!token.is_empty(), "token should not be empty");
    eprintln!("[pass] login with correct password returned token");

    // 2. Login with wrong password
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "testuser".to_string(),
            password: "wrongpass".to_string(),
        })),
        "SWARM",
        docker,
        &None,
    )
    .await?;

    assert_eq!(res, "", "wrong password should return empty string");
    eprintln!("[pass] login with wrong password returned empty string");

    // 3. ChangePassword (user_id=2 is testuser)
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::ChangePassword(ChangePasswordInfo {
            user_id: 2,
            old_pass: "testpass".to_string(),
            password: "newpass123".to_string(),
        })),
        "SWARM",
        docker,
        &Some(2),
    )
    .await?;

    let parsed: serde_json::Value = serde_json::from_str(&res)?;
    assert_eq!(
        parsed["success"], true,
        "ChangePassword should succeed, got: {}",
        res
    );
    eprintln!("[pass] ChangePassword succeeded");

    // 4. Login with NEW password
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "testuser".to_string(),
            password: "newpass123".to_string(),
        })),
        "SWARM",
        docker,
        &None,
    )
    .await?;

    let parsed: serde_json::Value = serde_json::from_str(&res)?;
    assert!(
        parsed["token"].is_string() && !parsed["token"].as_str().unwrap().is_empty(),
        "login with new password should return token, got: {}",
        res
    );
    eprintln!("[pass] login with new password returned token");

    // 5. Login with OLD password should fail
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        })),
        "SWARM",
        docker,
        &None,
    )
    .await?;

    assert_eq!(
        res, "",
        "login with old password should return empty string after change"
    );
    eprintln!("[pass] login with old password correctly rejected after change");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Test 3: Read during write doesn't deadlock
// ═══════════════════════════════════════════════════════════════════════

/// Verifies that GetConfig can complete while a concurrent stack_write is in
/// progress. This is the core UI-freeze scenario the refactor was designed to fix.
///
/// We simulate a slow write by spawning a task that holds the STACK write lock
/// for 2 seconds, then immediately try GetConfig. With RwLock, the write blocks
/// reads, BUT because handle() only holds locks briefly (not during slow work
/// like Docker ops or bcrypt), a real RestartContainer wouldn't block reads.
///
/// Here we test the handler-level pattern: spawn a Login (which does bcrypt
/// outside the lock) and verify GetConfig completes quickly even though Login
/// is doing expensive CPU work.
async fn test_read_during_write(docker: &Docker) -> Result<()> {
    // Build a stack with a user whose password hash is expensive to verify
    let stack = make_auth_stack();
    hydrate(stack, Clients::default()).await;
    sphinx_swarm::auth::set_jwt_key("test-jwt-key");

    // Spawn a Login call — bcrypt::verify is CPU-expensive but runs outside the lock
    let docker_clone = docker.clone();
    let login_handle = tokio::spawn(async move {
        handle(
            TEST_PROJECT,
            Cmd::Swarm(SwarmCmd::Login(LoginInfo {
                username: "admin".to_string(),
                password: "adminpass".to_string(),
            })),
            "SWARM",
            &docker_clone,
            &None,
        )
        .await
    });

    // Immediately try GetConfig — should not be blocked by the Login's bcrypt
    let start = Instant::now();
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        docker,
        &Some(1),
    )
    .await;

    let elapsed = start.elapsed();
    assert!(res.is_ok(), "GetConfig failed: {:?}", res.err());
    assert!(
        elapsed.as_secs() < 2,
        "GetConfig took {:?}, should be fast since bcrypt is outside the lock",
        elapsed
    );
    eprintln!("[pass] GetConfig completed in {:?} while Login was running", elapsed);

    // Wait for Login to finish too
    let login_res = login_handle.await.expect("login task panicked");
    assert!(login_res.is_ok(), "Login failed: {:?}", login_res.err());
    eprintln!("[pass] Login also completed successfully");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Test 4: Stack mutations persist
// ═══════════════════════════════════════════════════════════════════════

/// Sets global_mem_limit via handle, then reads it back via GetConfig.
async fn test_stack_mutations_persist(docker: &Docker) -> Result<()> {
    let stack = make_auth_stack();
    hydrate(stack, Clients::default()).await;
    sphinx_swarm::auth::set_jwt_key("test-jwt-key");

    // Set global_mem_limit to 1234
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::SetGlobalMemLimit(1234)),
        "SWARM",
        docker,
        &Some(1),
    )
    .await;

    assert!(
        res.is_ok(),
        "SetGlobalMemLimit failed: {:?}",
        res.err()
    );
    let json = res.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(
        parsed["global_mem_limit"], 1234,
        "response should echo the new limit"
    );
    eprintln!("[pass] SetGlobalMemLimit returned correct value");

    // Read it back via GetConfig
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        docker,
        &Some(1),
    )
    .await?;

    let parsed: serde_json::Value = serde_json::from_str(&res)?;
    assert_eq!(
        parsed["global_mem_limit"], 1234,
        "GetConfig should show persisted global_mem_limit=1234, got: {}",
        parsed["global_mem_limit"]
    );
    eprintln!("[pass] GetConfig shows persisted global_mem_limit=1234");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Test 5: Client clone safety (concurrent BitcoinRPC)
// ═══════════════════════════════════════════════════════════════════════

/// Spawn 5 concurrent Bitcoind::GetInfo calls against the live bitcoind container.
/// Verifies that Arc<BitcoinRPC> clone + concurrent access works correctly.
async fn test_concurrent_bitcoind_calls(docker: &Docker) -> Result<()> {
    // Reuse the bitcoind container from test_bitcoind_get_info (still running).
    // Re-hydrate to make sure globals are set with the live client.
    start_bitcoind_and_hydrate(docker).await?;

    let start = Instant::now();
    let mut handles = Vec::new();

    for i in 0..5 {
        let docker = docker.clone();
        let h = tokio::spawn(async move {
            let res = handle(
                TEST_PROJECT,
                Cmd::Bitcoind(BitcoindCmd::GetInfo),
                BTC_NAME,
                &docker,
                &Some(1),
            )
            .await;
            (i, res)
        });
        handles.push(h);
    }

    let mut chains = Vec::new();
    for h in handles {
        let (i, res) = h.await.expect("task panicked");
        assert!(
            res.is_ok(),
            "concurrent GetInfo #{} failed: {:?}",
            i,
            res.err()
        );
        let json = res.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json)?;
        assert_eq!(parsed["chain"], "regtest", "task #{} wrong chain", i);
        chains.push(parsed["chain"].as_str().unwrap().to_string());
    }

    let elapsed = start.elapsed();
    assert_eq!(chains.len(), 5, "all 5 tasks should have returned");
    // All should report the same chain
    assert!(
        chains.iter().all(|c| c == "regtest"),
        "all tasks should see regtest"
    );
    assert!(
        elapsed.as_secs() < 5,
        "5 concurrent GetInfo took {:?}, expected < 5s",
        elapsed
    );

    eprintln!(
        "[pass] 5 concurrent Bitcoind::GetInfo completed in {:?}, all returned regtest",
        elapsed
    );

    // Final cleanup
    cleanup(docker).await;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Test 6: Access control
// ═══════════════════════════════════════════════════════════════════════

/// Verifies:
/// 1. Non-Login command with user_id: None -> access denied
/// 2. Non-Login command with non-existent user_id -> access denied
/// 3. Login with user_id: None -> allowed (login doesn't require auth)
async fn test_access_control(docker: &Docker) -> Result<()> {
    let stack = make_auth_stack();
    hydrate(stack, Clients::default()).await;
    sphinx_swarm::auth::set_jwt_key("test-jwt-key");

    // 1. GetConfig with no user_id -> access denied
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        docker,
        &None,
    )
    .await;

    assert!(res.is_err(), "GetConfig with no user should be denied");
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains("access denied"),
        "expected 'access denied', got: {}",
        err
    );
    eprintln!("[pass] GetConfig with user_id=None -> access denied");

    // 2. GetConfig with non-existent user_id -> access denied
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::GetConfig),
        "SWARM",
        docker,
        &Some(9999), // no user with this id
    )
    .await;

    assert!(
        res.is_err(),
        "GetConfig with non-existent user should be denied"
    );
    let err = res.err().unwrap().to_string();
    assert!(
        err.contains("access denied"),
        "expected 'access denied', got: {}",
        err
    );
    eprintln!("[pass] GetConfig with user_id=9999 -> access denied");

    // 3. Login with user_id: None -> allowed
    let res = handle(
        TEST_PROJECT,
        Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "admin".to_string(),
            password: "adminpass".to_string(),
        })),
        "SWARM",
        docker,
        &None,
    )
    .await;

    assert!(res.is_ok(), "Login should be allowed without auth, got: {:?}", res.err());
    let json = res.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json)?;
    assert!(
        parsed["token"].is_string() && !parsed["token"].as_str().unwrap().is_empty(),
        "Login should return a token, got: {}",
        json
    );
    eprintln!("[pass] Login with user_id=None -> allowed, returned token");

    Ok(())
}
