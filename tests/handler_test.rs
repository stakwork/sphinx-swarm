/// Integration tests for command handling.
///
/// These tests exercise the `handle()` function directly (bypassing Rocket
/// routes and the mpsc channel) so that we can verify state-management
/// commands work correctly both before and after the concurrency refactor.
///
/// Commands that require Docker or external services are NOT tested here.
/// We focus on state reads, state writes, login/auth, and — critically —
/// concurrent command execution.
///
/// IMPORTANT: Because `handle()` uses a process-global `STATE` singleton,
/// ALL tests must run sequentially.  Run with: cargo test --test handler_test
/// The tests are structured as a single #[tokio::test] to guarantee ordering.
use sphinx_swarm::auth;
use sphinx_swarm::cmd::*;
use sphinx_swarm::config::{Clients, LightningPeer, Stack, User, Role, STACK};
use sphinx_swarm::handler;
use std::collections::HashMap;

use bollard::Docker;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal Stack with one admin user (admin/password) and mark it ready.
fn make_test_stack() -> Stack {
    let pass_hash = bcrypt::hash("password", 4).expect("bcrypt hash"); // cost=4 for speed
    Stack {
        network: "regtest".to_string(),
        nodes: vec![],
        host: None,
        users: vec![User {
            id: 1,
            username: "admin".to_string(),
            pass_hash,
            pubkey: None,
            role: Role::Admin,
        }],
        jwt_key: "test-jwt-secret".to_string(),
        ready: true,
        ip: None,
        auto_update: None,
        auto_restart: None,
        custom_2b_domain: None,
        global_mem_limit: None,
        backup_services: None,
        backup_files: None,
        lightning_peers: Some(vec![]),
        ssl_cert_last_modified: None,
        instance_id: None,
    }
}

/// Hydrate global state with the given stack and empty clients, then set the
/// JWT key so that `auth::make_jwt` works.  Also ensures the vol/test/
/// directory exists for config persistence.
async fn setup_state(stack: Stack) {
    let _ = tokio::fs::create_dir_all("vol/test").await;
    auth::set_jwt_key(&stack.jwt_key);
    handler::hydrate(stack, Clients::default()).await;
}

fn docker() -> Docker {
    Docker::connect_with_unix_defaults().unwrap()
}

/// Send a Cmd through `handle()` with admin user_id=1.
async fn run_cmd(cmd: Cmd) -> anyhow::Result<String> {
    let docker = docker();
    handler::handle("test", cmd, "SWARM", &docker, &Some(1)).await
}

/// Send a Cmd through `handle()` with no user_id (unauthenticated).
async fn run_cmd_no_auth(cmd: Cmd) -> anyhow::Result<String> {
    let docker = docker();
    handler::handle("test", cmd, "SWARM", &docker, &None).await
}

// ---------------------------------------------------------------------------
// All tests in one function to guarantee sequential execution on global STATE.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handler_commands() {
    // =====================================================================
    // 1. GetConfig
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let result = run_cmd(Cmd::Swarm(SwarmCmd::GetConfig))
            .await
            .expect("GetConfig should succeed");
        let parsed: Stack = serde_json::from_str(&result).expect("should parse as Stack");
        assert_eq!(parsed.network, "regtest");
        assert!(parsed.users.is_empty(), "users should be stripped");
        assert!(parsed.jwt_key.is_empty(), "jwt_key should be stripped");
        assert!(parsed.ready);
        println!("  [PASS] GetConfig");
    }

    // =====================================================================
    // 2. GetConfig without auth should fail
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let result = run_cmd_no_auth(Cmd::Swarm(SwarmCmd::GetConfig)).await;
        assert!(result.is_err(), "GetConfig without auth should fail");
        println!("  [PASS] GetConfig no auth");
    }

    // =====================================================================
    // 3. Login success
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "admin".to_string(),
            password: "password".to_string(),
        }));
        let result = run_cmd_no_auth(cmd).await.expect("Login should succeed");
        let parsed: HashMap<String, String> =
            serde_json::from_str(&result).expect("should parse");
        assert!(parsed.contains_key("token"), "should contain token");
        assert!(!parsed["token"].is_empty(), "token should not be empty");
        println!("  [PASS] Login success");
    }

    // =====================================================================
    // 4. Login wrong password
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "admin".to_string(),
            password: "wrong".to_string(),
        }));
        let result = run_cmd_no_auth(cmd).await.expect("Login should return Ok");
        assert_eq!(result, "", "wrong password should return empty string");
        println!("  [PASS] Login wrong password");
    }

    // =====================================================================
    // 5. Login unknown user
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "nobody".to_string(),
            password: "password".to_string(),
        }));
        let result = run_cmd_no_auth(cmd).await.expect("Login should return Ok");
        assert_eq!(result, "", "unknown user should return empty string");
        println!("  [PASS] Login unknown user");
    }

    // =====================================================================
    // 6. SetGlobalMemLimit (state mutation + save)
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let cmd = Cmd::Swarm(SwarmCmd::SetGlobalMemLimit(4096));
        let result = run_cmd(cmd).await.expect("SetGlobalMemLimit should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("should parse");
        assert_eq!(parsed["global_mem_limit"], 4096);

        let stack = STACK.read().await;
        assert_eq!(stack.global_mem_limit, Some(4096));
        drop(stack);
        println!("  [PASS] SetGlobalMemLimit");
    }

    // =====================================================================
    // 7. AddLightningPeer (state mutation + save)
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let peer = LightningPeer {
            alias: "testpeer".to_string(),
            pubkey: "02abc123".to_string(),
        };
        let cmd = Cmd::Swarm(SwarmCmd::AddLightningPeer(peer));
        let result = run_cmd(cmd).await.expect("AddLightningPeer should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("should parse");
        assert_eq!(parsed["success"], true);

        let stack = STACK.read().await;
        let peers = stack.lightning_peers.as_ref().unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].alias, "testpeer");
        assert_eq!(peers[0].pubkey, "02abc123");
        drop(stack);
        println!("  [PASS] AddLightningPeer");
    }

    // =====================================================================
    // 8. UpdateLightningPeer
    // =====================================================================
    {
        let mut stack = make_test_stack();
        stack.lightning_peers = Some(vec![LightningPeer {
            alias: "old".to_string(),
            pubkey: "02abc123".to_string(),
        }]);
        setup_state(stack).await;

        let updated = LightningPeer {
            alias: "new".to_string(),
            pubkey: "02abc123".to_string(),
        };
        let cmd = Cmd::Swarm(SwarmCmd::UpdateLightningPeer(updated));
        let result = run_cmd(cmd)
            .await
            .expect("UpdateLightningPeer should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("should parse");
        assert_eq!(parsed["success"], true);

        let stack = STACK.read().await;
        assert_eq!(stack.lightning_peers.as_ref().unwrap()[0].alias, "new");
        drop(stack);
        println!("  [PASS] UpdateLightningPeer");
    }

    // =====================================================================
    // 9. GetSignedInUserDetails
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let cmd = Cmd::Swarm(SwarmCmd::GetSignedInUserDetails);
        let result = run_cmd(cmd)
            .await
            .expect("GetSignedInUserDetails should succeed");
        let parsed: User = serde_json::from_str(&result).expect("should parse as User");
        assert_eq!(parsed.username, "admin");
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.pass_hash, "", "pass_hash should be stripped");
        println!("  [PASS] GetSignedInUserDetails");
    }

    // =====================================================================
    // 10. GetLightningPeers
    // =====================================================================
    {
        let mut stack = make_test_stack();
        stack.lightning_peers = Some(vec![
            LightningPeer {
                alias: "alice".to_string(),
                pubkey: "02aaa".to_string(),
            },
            LightningPeer {
                alias: "bob".to_string(),
                pubkey: "02bbb".to_string(),
            },
        ]);
        setup_state(stack).await;

        let cmd = Cmd::Swarm(SwarmCmd::GetLightningPeers);
        let result = run_cmd(cmd)
            .await
            .expect("GetLightningPeers should succeed");
        let parsed: Vec<LightningPeer> = serde_json::from_str(&result).expect("should parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].alias, "alice");
        assert_eq!(parsed[1].alias, "bob");
        println!("  [PASS] GetLightningPeers");
    }

    // =====================================================================
    // 11. Access denied without user_id
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let cmd = Cmd::Swarm(SwarmCmd::SetGlobalMemLimit(1024));
        let result = run_cmd_no_auth(cmd).await;
        assert!(result.is_err(), "should be denied without auth");
        assert!(result.unwrap_err().to_string().contains("access denied"));
        println!("  [PASS] Access denied without user_id");
    }

    // =====================================================================
    // 12. Not-ready state blocks most commands
    //     (use hydrate_stack instead of hydrate — hydrate forces ready=true)
    // =====================================================================
    {
        let mut stack = make_test_stack();
        stack.ready = false;
        let _ = tokio::fs::create_dir_all("vol/test").await;
        auth::set_jwt_key(&stack.jwt_key);
        handler::hydrate_stack(stack).await;

        // GetConfig should still work before ready
        let result = run_cmd(Cmd::Swarm(SwarmCmd::GetConfig)).await;
        assert!(result.is_ok(), "GetConfig should work before ready");

        // Login should still work before ready
        let cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "admin".to_string(),
            password: "password".to_string(),
        }));
        let result = run_cmd_no_auth(cmd).await;
        assert!(result.is_ok(), "Login should work before ready");

        // Other commands should fail before ready
        let cmd = Cmd::Swarm(SwarmCmd::SetGlobalMemLimit(1024));
        let result = run_cmd(cmd).await;
        assert!(result.is_err(), "SetGlobalMemLimit should fail before ready");
        println!("  [PASS] Not-ready state");
    }

    // =====================================================================
    // 13. CONCURRENT: Multiple GetConfig reads
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let mut handles = vec![];
        for _ in 0..10 {
            handles.push(tokio::spawn(async {
                let cmd = Cmd::Swarm(SwarmCmd::GetConfig);
                run_cmd(cmd).await
            }));
        }
        for handle in handles {
            let result = handle.await.expect("task should not panic");
            let result = result.expect("GetConfig should succeed");
            let parsed: Stack = serde_json::from_str(&result).expect("should parse");
            assert_eq!(parsed.network, "regtest");
        }
        println!("  [PASS] Concurrent GetConfig (10 readers)");
    }

    // =====================================================================
    // 14. CONCURRENT: Multiple logins
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let mut handles = vec![];
        for _ in 0..10 {
            handles.push(tokio::spawn(async {
                let cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
                    username: "admin".to_string(),
                    password: "password".to_string(),
                }));
                run_cmd_no_auth(cmd).await
            }));
        }
        for handle in handles {
            let result = handle.await.expect("task should not panic");
            let result = result.expect("Login should succeed");
            let parsed: HashMap<String, String> =
                serde_json::from_str(&result).expect("should parse");
            assert!(parsed.contains_key("token"));
        }
        println!("  [PASS] Concurrent logins (10 logins)");
    }

    // =====================================================================
    // 15. CONCURRENT: Mixed reads + writes (the KEY test)
    //     5 GetConfig reads + 5 AddLightningPeer writes simultaneously.
    //     After all complete, all 5 peers must exist in state.
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let mut handles = vec![];

        for i in 0..5 {
            // reader
            handles.push(tokio::spawn(async move {
                let cmd = Cmd::Swarm(SwarmCmd::GetConfig);
                let result = run_cmd(cmd).await.expect("GetConfig should succeed");
                let parsed: Stack = serde_json::from_str(&result).expect("should parse");
                assert_eq!(parsed.network, "regtest");
                "read_ok".to_string()
            }));

            // writer
            let alias = format!("peer{}", i);
            let pubkey = format!("02{:03}", i);
            handles.push(tokio::spawn(async move {
                let peer = LightningPeer { alias, pubkey };
                let cmd = Cmd::Swarm(SwarmCmd::AddLightningPeer(peer));
                let result = run_cmd(cmd)
                    .await
                    .expect("AddLightningPeer should succeed");
                let parsed: serde_json::Value =
                    serde_json::from_str(&result).expect("should parse");
                assert_eq!(parsed["success"], true);
                "write_ok".to_string()
            }));
        }

        for handle in handles {
            let result = handle.await.expect("task should not panic");
            assert!(result == "read_ok" || result == "write_ok");
        }

        let stack = STACK.read().await;
        let peers = stack.lightning_peers.as_ref().unwrap();
        assert_eq!(peers.len(), 5, "all 5 peers should have been added");
        drop(stack);
        println!("  [PASS] Concurrent reads + writes (5 readers, 5 writers)");
    }

    // =====================================================================
    // 16. CONCURRENT: Multiple state mutations
    //     10 concurrent SetGlobalMemLimit — last writer wins, but all must
    //     succeed and the final value must be valid.
    // =====================================================================
    {
        setup_state(make_test_stack()).await;
        let mut handles = vec![];
        for i in 0..10u64 {
            let val = (i + 1) * 1024;
            handles.push(tokio::spawn(async move {
                let cmd = Cmd::Swarm(SwarmCmd::SetGlobalMemLimit(val));
                run_cmd(cmd).await.expect("SetGlobalMemLimit should succeed");
            }));
        }
        for handle in handles {
            handle.await.expect("task should not panic");
        }
        let stack = STACK.read().await;
        assert!(
            stack.global_mem_limit.is_some(),
            "global_mem_limit should be set"
        );
        let val = stack.global_mem_limit.unwrap();
        assert!(
            val >= 1024 && val <= 10240,
            "value should be in expected range, got {}",
            val
        );
        drop(stack);
        println!("  [PASS] Concurrent mem limit updates (10 writers)");
    }

    println!("\n=== ALL HANDLER TESTS PASSED ===");
}
