# Main Swarm Concurrent Command Handling — Implementation Plan

## Problem

Same problem as the super admin had, but worse. All swarm commands flow through a single `mpsc` channel processed by `spawn_handler`, which runs one command at a time. The global `Mutex<State>` is held for the **entire duration** of every command — including Docker operations, Lightning RPC calls, HTTP requests to boltwall, bcrypt hashing, and more.

While any command runs (even a slow `UpdateNode` pulling a Docker image), every other command queues up: `GetConfig`, `ListContainers`, `Login`, all of it. The UI freezes.

Three cron jobs also compete for the same lock:
- `auto_restart_cron` — holds the lock during N sequential Docker restarts
- `renew_ssl_cert` — holds the lock during S3 + HTTP calls
- `update_node_from_state` (auto-updater) — holds the lock during Docker pull + gRPC connect (minutes)

## Why This Is Harder Than Super

The super admin's `State` was pure config data — no live connections. The main swarm's `State` holds both config (`Stack`) and **live RPC client connections** (`Clients`):

```rust
pub struct State {
    pub stack: Stack,      // config, users, nodes — serializable
    pub clients: Clients,  // live connections — BitcoinRPC, LndRPC, ClnRPC, etc.
}

pub struct Clients {
    pub bitcoind: HashMap<String, BitcoinRPC>,
    pub lnd: HashMap<String, LndRPC>,
    pub cln: HashMap<String, ClnRPC>,
    pub proxy: HashMap<String, ProxyAPI>,
    pub relay: HashMap<String, RelayAPI>,
    pub hsmd: HashMap<String, HsmdClient>,
}
```

Commands like `Cmd::Lnd(LndCmd::GetInfo)` need to grab a client from the hashmap and make an RPC call on it. The I/O *is* calling a method on the client that lives inside state. We can't just "read config, drop lock, do I/O" — the client itself needs to be accessible outside the lock.

Most client types are already cloneable or trivially can be. The two that aren't get `Arc` wrapping:

| Client | Methods | Clone? | Strategy |
|---|---|---|---|
| **ProxyAPI** | `&self` | Trivially | `#[derive(Clone)]`, clone out of map |
| **RelayAPI** | `&self` | Trivially | `#[derive(Clone)]`, clone out of map |
| **HsmdClient** | `&self` | Trivially | `#[derive(Clone)]`, clone out of map |
| **ClnRPC** | `&mut self` | Trivially (`NodeClient<Channel>` is tonic-generated, Clone) | `#[derive(Clone)]`, relax methods to `&self` |
| **BitcoinRPC** | `&self` | No (`bitcoincore_rpc::Client` not Clone) | `Arc<BitcoinRPC>` (no Mutex — methods are `&self`) |
| **LndRPC** | `&mut self` | No (would require fork changes) | `Arc<Mutex<LndRPC>>` — simplest, not a hot path |

`ClnRPC`'s tonic-generated `NodeClient<Channel>` is Clone and its methods don't actually need `&mut self` (tonic clients multiplex over a cloned Channel). So we derive Clone and relax to `&self`.

`LndRPC` has the same tonic internals but is behind a fork (`Evanfeenstra/tonic_lnd`) that exposes `&mut self` accessors. Rather than modifying the fork, just wrap in `Arc<Mutex>` — LND commands to the same node serialize, which is fine.

## Core Design: Separate `Stack` and `Clients` Into Independent Locks

Split `State` into two independently-lockable pieces:

```rust
use once_cell::sync::Lazy;
use rocket::tokio::sync::RwLock;
use std::sync::Arc;

// Config/user data — same RwLock pattern as super admin
pub static STACK: Lazy<RwLock<Stack>> = Lazy::new(|| RwLock::new(Default::default()));

// Live client connections — cloneable clients, grab and go
pub static CLIENTS: Lazy<RwLock<ClientMap>> = Lazy::new(|| RwLock::new(Default::default()));
```

### Why two separate locks?

Most commands need only one:
- `GetConfig`, `Login`, `ChangePassword` → only `STACK`
- `Cmd::Bitcoind(GetInfo)`, `Cmd::Lnd(GetInfo)` → only `CLIENTS`
- `StartContainer`, `UpdateNode` → both, but at different times (read stack, then use client)

With a single lock, a slow `Lnd::PayInvoice` blocks `GetConfig`. With separate locks, they don't interfere at all.

### ClientMap: mostly just Clone, one Arc

```rust
pub struct ClientMap {
    pub bitcoind: HashMap<String, Arc<BitcoinRPC>>,        // Arc — not Clone, but methods are &self
    pub lnd: HashMap<String, Arc<Mutex<LndRPC>>>,          // Arc<Mutex> — not Clone, methods are &mut self
    pub cln: HashMap<String, ClnRPC>,                       // Clone
    pub proxy: HashMap<String, ProxyAPI>,                    // Clone
    pub relay: HashMap<String, RelayAPI>,                    // Clone
    pub hsmd: HashMap<String, HsmdClient>,                   // Clone
}
```

Usage:
```rust
// Cloneable clients: clone out, use freely
let client = clients_read(|c| c.cln.get(tag).cloned()).await.context("no cln")?;
client.get_info().await?;  // no lock held

// Arc-only (BitcoinRPC): clone the Arc, deref to &self methods
let client = clients_read(|c| c.bitcoind.get(tag).cloned()).await.context("no btc")?;
client.get_info()?;  // Arc<BitcoinRPC> derefs, &self works

// Arc<Mutex> (LndRPC): clone the Arc, lock for the RPC call
let client = clients_read(|c| c.lnd.get(tag).cloned()).await.context("no lnd")?;
let info = client.lock().await.get_info().await?;
```

This means:
- `GetConfig` on STACK and `Lnd::GetInfo` on CLIENTS run concurrently ✓
- Two `Lnd::GetInfo` calls to different nodes run concurrently ✓
- Two `Lnd::GetInfo` calls to the same node serialize (fine — not a hot path) ✓
- Two `Cln::GetInfo` calls to the same node run concurrently (tonic channels multiplex) ✓
- `StartContainer` can read STACK, do Docker work, then grab a client — no long locks ✓

## Stack Access Helpers

Same pattern as super admin, adapted for `Stack` instead of `Super`:

```rust
/// Read something from stack config. Lock held only for the duration of `f`.
pub async fn stack_read<F, T>(f: F) -> T
where
    F: FnOnce(&Stack) -> T,
{
    let stack = STACK.read().await;
    f(&stack)
}

/// Mutate stack config and save to disk. Lock held only for the duration of `f`.
pub async fn stack_write<F, T>(proj: &str, f: F) -> T
where
    F: FnOnce(&mut Stack) -> T,
{
    let mut stack = STACK.write().await;
    let result = f(&mut stack);
    put_config_file(proj, &stack).await;
    result
}
```

## Client Access Helpers

```rust
/// Read/clone something from the client map. Brief read lock on CLIENTS.
pub async fn clients_read<F, T>(f: F) -> T
where
    F: FnOnce(&ClientMap) -> T,
{
    let clients = CLIENTS.read().await;
    f(&clients)
}

/// Insert or remove a client. Brief write lock on CLIENTS.
pub async fn clients_write<F, T>(f: F) -> T
where
    F: FnOnce(&mut ClientMap) -> T,
{
    let mut clients = CLIENTS.write().await;
    f(&mut clients)
}
```

## Phased Implementation

Each phase is self-contained and the code compiles and works after each one.

---

### Phase 0: Integration Test

**Goal:** A test that exercises the real handler flow end-to-end: spin up a bitcoind container, call `GetInfo` through the handler, verify we get a block height, tear it down. This gives us a safety net for all subsequent phases.

**What to build:**

Create `tests/handler_test.rs` — a `#[tokio::test]` that:
1. Creates the Docker client and network
2. Builds a minimal stack with just bitcoind (regtest)
3. Calls `hydrate()` to populate STATE
4. Calls `handle()` with `Cmd::Swarm(SwarmCmd::GetConfig)` and verifies the response
5. Calls `handle()` with `Cmd::Bitcoind(BitcoindCmd::GetInfo)` and verifies block height
6. Tears down the container and cleans up

```rust
#[tokio::test]
async fn test_bitcoind_handler_flow() {
    // 1. Docker setup
    let docker = dockr();
    create_network(&docker, Some("test-swarm")).await.unwrap();

    // 2. Create and start bitcoind
    let mut btc = BtcImage::new("test-btc", "23.0", "regtest");
    btc.set_user_password("testuser", "testpass");
    // ... create container, start it, connect client

    // 3. Hydrate state
    hydrate(stack, clients).await;

    // 4. Test GetConfig
    let res = handle("test", Cmd::Swarm(SwarmCmd::GetConfig), "SWARM", &docker, &Some(1)).await;
    assert!(res.is_ok());

    // 5. Test GetInfo
    let res = handle("test", Cmd::Bitcoind(BitcoindCmd::GetInfo), "test-btc", &docker, &Some(1)).await;
    assert!(res.is_ok());
    // verify response contains blockcount

    // 6. Cleanup
    stop_and_remove(&docker, "test-btc.sphinx").await.unwrap();
}
```

**Depends on:** `Clients` not implementing `Default` for test contexts — may need a test helper to build a minimal `Clients` with just a bitcoind entry. The existing `build_stack` or `add_node` functions can be reused.

**Run with:** `cargo test --test handler_test -- --nocapture` (needs Docker running locally)

**Note:** This test uses the CURRENT architecture (single Mutex, channel, etc.). We'll update it as we refactor.

---

### Phase 1: Make Clients Cloneable + Split `State` Into `STACK` + `CLIENTS`

**Goal:** Replace the single `Mutex<State>` with `RwLock<Stack>` + `RwLock<ClientMap>`. Everything still runs through the channel and `spawn_handler`, but the data structure is ready.

**Changes:**

#### Prerequisite: Make all client types cloneable

Most are trivial — just add `#[derive(Clone)]`:

| Client | Change needed |
|---|---|
| `ProxyAPI` | Add `#[derive(Clone)]` — all fields already Clone |
| `RelayAPI` | Add `#[derive(Clone)]` — all fields already Clone |
| `HsmdClient` | Add `#[derive(Clone)]` — all fields already Clone |
| `ClnRPC` | Add `#[derive(Clone)]` — `NodeClient<Channel>` is Clone. Relax methods from `&mut self` → `&self` |
| `LndRPC` | No changes — wrap in `Arc<Mutex<LndRPC>>` in the map |
| `BitcoinRPC` | No changes — wrap in `Arc<BitcoinRPC>` in the map |

For `ClnRPC`, the `&mut self` is unnecessary — tonic generated service clients multiplex over a cloned `Channel`. Each `.clone()` gives an independent handle to the same HTTP/2 connection.

#### `config.rs`
- Remove `pub static STATE: Lazy<Mutex<State>>` and the `State` struct
- Add `pub static STACK: Lazy<RwLock<Stack>>`
- Add `stack_read()` and `stack_write()` helpers
- Keep `put_config_file`, `load_config_file`, etc. unchanged

#### New `clients.rs` (or section in `config.rs`)
- Define `ClientMap` — plain cloneable types except `Arc<BitcoinRPC>`:
  ```rust
  pub struct ClientMap {
      pub bitcoind: HashMap<String, Arc<BitcoinRPC>>,
      pub lnd: HashMap<String, LndRPC>,
      pub cln: HashMap<String, ClnRPC>,
      pub proxy: HashMap<String, ProxyAPI>,
      pub relay: HashMap<String, RelayAPI>,
      pub hsmd: HashMap<String, HsmdClient>,
  }
  ```
- Add `pub static CLIENTS: Lazy<RwLock<ClientMap>>`
- Add `clients_read()` and `clients_write()` helpers

#### `handler.rs`
- `handle()` changes from one big `STATE.lock()` to targeted reads/writes:
  ```rust
  // Before:
  let mut state = config::STATE.lock().await;
  if !access(&cmd, &state, user_id) { ... }

  // After:
  let allowed = stack_read(|s| access(&cmd, s, user_id)).await;
  if !allowed { ... }
  ```
- Each match arm uses the appropriate helper (see Command Patterns below)
- `must_save_stack` flag goes away — `stack_write()` auto-saves
- `hydrate()`, `hydrate_stack()`, `hydrate_clients()` rewritten for new statics

#### `builder.rs`
- `build_stack()` returns `ClientMap` instead of `Clients`
- `add_node()`, `make_client()`, `connect_client()` take `&mut ClientMap`
- `Image::remove_client()` calls `.remove()` on the `ClientMap` hashmap

#### All Image `connect_client` implementations
- `btc.rs`: `clients.bitcoind.insert(name, Arc::new(client))`
- `lnd.rs`: `clients.lnd.insert(name, Arc::new(Mutex::new(client)))`
- `cln.rs`: `clients.cln.insert(name, client)` (plain Clone)
- `proxy.rs`, `relay.rs`, `hsmd.rs`: same — plain insert

#### External `STATE` consumers — migrate to `STACK`/`CLIENTS`

| File | Current | New |
|---|---|---|
| `builder.rs:116` (`update_node_from_state`) | `STATE.lock()` held during Docker + gRPC | `stack_read` for node info, `clients_write` for remove/insert, Docker work outside both locks |
| `dock.rs:1140` (`restore_backup_if_exist`) | `STATE.lock()`, clones, drops | `stack_read(|s| (nodes.clone(), backup_services.clone()))` |
| `backup.rs:53` | `STATE.lock()`, clones nodes, drops | `stack_read(|s| s.nodes.clone())` |
| `app_login.rs:79,127` | `STATE.lock()` for user lookup | `stack_read(|s| s.users.iter().find(...).cloned())` |
| `auto_restart_cron.rs:61` | `STATE.lock()` held during N restarts | Split: `stack_read` for node info, Docker work outside, `clients_write` for reconnect |
| `renew_ssl_cert.rs:51` | `STATE.lock()` held during S3 + HTTP | Split: `stack_read` for cert info, S3/HTTP outside, `stack_write` if changed |
| `service/public_ip.rs:11,29` | `STATE.lock()` — already well-structured | `stack_read` / `stack_write` |

**Does NOT change yet:** The `mpsc` channel and `spawn_handler` still exist. Commands still serialize. But each command holds locks for milliseconds instead of seconds.

---

### Phase 2: Remove Channel, Direct `handle()` Calls

**Goal:** HTTP routes call `handle()` directly via `tokio::spawn`, enabling concurrent command execution.

**Changes:**

#### `handler.rs`
- Remove `spawn_handler()` function
- `handle()` signature stays the same — it's already `async fn`
- The timeout wrapper moves into the route handler

#### `routes.rs`
- Remove `mpsc::Sender<CmdRequest>` from route handlers
- Routes call `handle()` directly:
  ```rust
  let res = tokio::time::timeout(
      Duration::from_secs(60),
      handle(&proj, cmd, &tag, &docker, &Some(claims.user))
  ).await;
  ```
- `Docker` client passed as Rocket managed state (it's already `Clone`)

#### `rocket_utils.rs`
- `CmdRequest` stays — other binaries (tome, cln, etc.) still use it
- Stack binary just stops importing/using it

#### Binary entry points (`src/bin/stack/mod.rs`, etc.)
- Remove channel creation: `let (tx, rx) = mpsc::channel::<CmdRequest>(1000)`
- Remove `spawn_handler(proj, rx, docker.clone())`
- Pass `Docker` and project name as Rocket managed state
- `hydrate_stack` / `hydrate_clients` use the new globals

**After this phase:** Multiple HTTP requests execute concurrently. `GetConfig` responds instantly while `UpdateNode` pulls a Docker image. The UI stops freezing.

---

### Phase 3: Fix Cron Jobs

**Goal:** Cron jobs no longer hold locks during slow I/O.

#### `builder.rs` — `update_node_from_state()`

Before: locks STATE for entire Docker pull + gRPC connect.

After:
```rust
pub async fn update_node_from_state(proj: &str, docker: &Docker, node_name: &str) -> Result<()> {
    // 1. Read node info (milliseconds)
    let (nodes, img) = stack_read(|s| {
        let nodes = s.nodes.clone();
        let img = find_img(node_name, &nodes)?;
        Ok((nodes, img))
    }).await?;

    // 2. Check if update needed (no lock, hits Docker Hub)
    let version_response = get_image_version(node_name, &docker, &img.repo().org).await;
    if version_response.is_latest {
        return Ok(());
    }

    // 3. Remove client (brief write lock on CLIENTS)
    clients_write(|c| img.remove_client(c)).await;

    // 4. Docker work (no locks — pull, stop, create, start)
    update_node(proj, docker, node_name, &nodes, &img).await?;

    // 5. Reconnect client (brief write lock on CLIENTS, may block on gRPC connect)
    let nodes = stack_read(|s| s.nodes.clone()).await;
    img.connect_client(proj, &mut *CLIENTS.write().await, docker, &nodes, is_shutdown).await?;
    img.post_client(&*CLIENTS.read().await).await?;

    Ok(())
}
```

#### `auto_restart_cron.rs` — same split pattern

#### `renew_ssl_cert.rs` — read cert info from stack, do S3/HTTP outside, write back if changed

---

## Command Patterns (handler.rs)

Every command in `handle()` follows one of these patterns after the refactor:

### Pattern 1: Stack read only
```rust
SwarmCmd::GetConfig => {
    let res = stack_read(|s| serde_json::to_string(&s.remove_tokens())).await?;
    Some(res)
}
```
Commands: `GetConfig`, `GetSignedInUserDetails`, `GetLightningPeers`, `GetNeo4jPassword`, `GetBotToken`, `GetBoltwallRequestPerSeconds`, `GetBoltwallMaxRequestLimit`, `GetBoltwallAccessibility`, `GetFeatureFlags`, `GetSecondBrainAboutDetails`

### Pattern 2: Stack mutate
```rust
SwarmCmd::UpdateAdminPubkey(details) => {
    let res = stack_write(proj, |s| {
        match s.users.iter().position(|u| u.id == details.user_id) {
            Some(ui) => {
                s.users[ui].pubkey = Some(details.pubkey.to_string());
                serde_json::to_string(&HashMap::from([("success", true)]))
            }
            None => Ok("invalid user".to_string()),
        }
    }).await?;
    Some(res)
}
```
Commands: `UpdateAdminPubkey`, `SetGlobalMemLimit`, `UpdateNeo4jConfig`, `AddLightningPeer`, `UpdateLightningPeer`

### Pattern 3: Stack read + external I/O (boltwall pattern)
```rust
SwarmCmd::GetBoltwallSuperAdmin => {
    let boltwall = stack_read(|s| find_boltwall(&s.nodes)).await?;
    let response = crate::conn::boltwall::get_super_admin(&boltwall).await?;
    Some(serde_json::to_string(&response)?)
}
```
Commands: Most boltwall commands, `ListVersions`, `GetDockerImageTags`, `UpdateSwarm`

### Pattern 4: Stack read + external I/O + stack write
```rust
SwarmCmd::AddBoltwallUser(user) => {
    let boltwall = stack_read(|s| find_boltwall(&s.nodes)).await?;
    let response = crate::conn::boltwall::add_user(&boltwall, ...).await?;
    stack_write(proj, |s| { /* apply changes */ }).await;
    Some(serde_json::to_string(&response)?)
}
```
Commands: `AddBoltwallUser`, `DeleteSubAdmin`, `UpdateUser`, `UpdateBoltwallRequestPerSeconds`, `UpdateBoltwallMaxRequestLimit`, `UpdateEvn`, `ChangeReservedSwarmToActive`, `UpdateSslCert`

### Pattern 5: Client only (clone/Arc out and call)
```rust
// Cloneable client (Cln, Proxy, Relay, Hsmd)
Cmd::Cln(ClnCmd::GetInfo) => {
    let client = clients_read(|c| c.cln.get(tag).cloned())
        .await.context("no cln client")?;
    let info = client.get_info().await?;
    Some(serde_json::to_string(&info)?)
}

// Arc-only (BitcoinRPC) — deref to &self
Cmd::Bitcoind(BitcoindCmd::GetInfo) => {
    let client = clients_read(|c| c.bitcoind.get(tag).cloned())
        .await.context("no bitcoind client")?;
    let info = client.get_info()?;
    Some(serde_json::to_string(&info)?)
}

// Arc<Mutex> (LndRPC) — lock for the call
Cmd::Lnd(LndCmd::GetInfo) => {
    let client = clients_read(|c| c.lnd.get(tag).cloned())
        .await.context("no lnd client")?;
    let info = client.lock().await.get_info().await?;
    Some(serde_json::to_string(&info)?)
}
```
Commands: All `Cmd::Bitcoind`, most `Cmd::Lnd`, most `Cmd::Cln`, `Cmd::Proxy`, `Cmd::Hsmd`

### Pattern 6: Client + stack write (AddPeer)
```rust
Cmd::Lnd(LndCmd::AddPeer(peer)) => {
    if let Some(alias) = peer.alias.clone() {
        stack_write(proj, |s| {
            add_new_lightning_peer(s, LightningPeer { pubkey: peer.pubkey.clone(), alias });
        }).await;
    }
    let client = clients_read(|c| c.lnd.get(tag).cloned())
        .await.context("no lnd client")?;
    let result = client.lock().await.add_peer(peer).await?;
    Some(serde_json::to_string(&result)?)
}
```

### Pattern 7: Login / bcrypt outside lock
```rust
SwarmCmd::Login(ld) => {
    let user_data = stack_read(|s| {
        s.users.iter().find(|u| u.username == ld.username).map(|u| (u.id, u.pass_hash.clone()))
    }).await;
    match user_data {
        Some((uid, hash)) => {
            if !bcrypt::verify(&ld.password, &hash)? {
                Some("".to_string())
            } else {
                let mut hm = HashMap::new();
                hm.insert("token", auth::make_jwt(uid)?);
                Some(serde_json::to_string(&hm)?)
            }
        }
        None => Some("".to_string()),
    }
}
```

### Pattern 8: Docker operations + client reconnect
```rust
SwarmCmd::RestartContainer(id) => {
    // 1. Read node info
    let (img, nodes) = stack_read(|s| {
        let img = find_img(&id, &s.nodes)?;
        Ok((img, s.nodes.clone()))
    }).await?;

    // 2. Remove old client (brief CLIENTS write)
    clients_write(|c| img.remove_client(c)).await;

    // 3. Docker work (no locks)
    let hostname = domain(&id);
    let theconfig = img.make_config(&nodes, &docker).await?;
    stop_and_remove(&docker, &hostname).await?;
    let new_id = create_container(&docker, theconfig).await?;
    img.pre_startup(&docker, &nodes).await.ok();
    start_container(&docker, &new_id).await?;
    img.post_startup(proj, &docker).await?;

    // 4. Reconnect client (brief CLIENTS write)
    let nodes = stack_read(|s| s.nodes.clone()).await;
    let mut cm = CLIENTS.write().await;
    img.connect_client(proj, &mut cm, &docker, &nodes, is_shutdown).await?;
    img.post_client(&cm).await?;

    Some(serde_json::to_string("{}")?)
}
```

## Functions That Need Signature Changes

These functions currently take `&mut State` or `&mut Clients`. They need to be updated:

| Function | Current | New |
|---|---|---|
| `restart_node_container` (dock.rs) | `&mut State` | Takes `Image`, `&[Node]`, `Docker`, returns new client to insert |
| `update_node_and_make_client` (builder.rs) | `&mut State` | Split into `update_node` (no state) + separate `make_client` call using `CLIENTS` |
| `make_client` (builder.rs) | `&mut State` | Takes `&mut ClientMap` directly, or callers use `clients_write` |
| `add_user` / `delete_sub_admin` / `update_user` (boltwall) | `&mut State, &mut bool` | Take extracted boltwall info, return changes to apply |
| `update_request_per_seconds` / `update_max_request_size` (boltwall) | `&mut State, &mut bool, Docker` | Split: read config, do Docker work, write back |
| `sign_up_admin_pubkey` (app_login) | `&mut bool, &mut State` | Takes extracted user data, returns mutation to apply |
| `change_swarm_user_password_by_user_admin` (swarm) | `&mut State` | Takes user list, returns new hash |
| `add_new_lightning_peer` / `update_lightning_peer` (swarm) | `&mut State` | Takes `&mut Stack` (only needs stack, not clients) |
| `handle_assign_reserved_swarm_to_active` (swarm) | `&mut State` | Split: read config, do HTTP, write back |
| `handle_update_ssl_cert` (renew_ssl_cert) | `&mut State` | Split: read cert info, do S3/HTTP, write back |
| `update_env_variables` (swarm) | `&mut State` | Split: read config, do Docker work, write back |

## Safety Rules

Same as the super admin plan, plus:

### 1. Never hold both STACK and CLIENTS locks simultaneously
If you need data from both, read STACK first (clone what you need), drop it, then access CLIENTS. This prevents deadlocks from inconsistent lock ordering.

```rust
// GOOD: sequential, no overlap
let nodes = stack_read(|s| s.nodes.clone()).await;
let client = clients_read(|c| c.lnd.get(tag).cloned()).await?;
let info = client.get_info().await?;

// BAD: holding both locks
let stack = STACK.read().await;
let clients = CLIENTS.read().await; // potential deadlock if someone else does the reverse
```

### 2. Clone clients out of the CLIENTS read lock, then drop the lock
```rust
// GOOD: lock held for microseconds
let client = clients_read(|c| c.lnd.get(tag).cloned()).await.context("no lnd")?;
client.get_info().await?;  // CLIENTS lock is free, client is an independent clone

// BAD: CLIENTS read lock held during RPC
let cm = CLIENTS.read().await;
let client = cm.lnd.get(tag).context("no lnd")?;
client.get_info().await?;  // other commands can't insert/remove clients
```

### 3. The `add_new_lightning_peer` + re-borrow pattern
Currently `Lnd::AddPeer` and `Cln::AddPeer` call `add_new_lightning_peer(&mut state)` then re-borrow `state.clients.lnd.get_mut(tag)`. With separate locks this becomes natural — write to STACK, then separately clone the client from CLIENTS.

## File Changes Summary

| File | Changes |
|---|---|
| `config.rs` | Replace `Mutex<State>` with `RwLock<Stack>`. Add `stack_read`/`stack_write`. Remove `State` struct. |
| New: `clients.rs` or in `config.rs` | `ClientMap` (Clone for 4 clients, `Arc` for BitcoinRPC, `Arc<Mutex>` for LndRPC). `CLIENTS` static. `clients_read`/`clients_write` helpers. |
| `handler.rs` | Remove single `STATE.lock()`. Each arm uses targeted helpers. Remove `spawn_handler`. Remove `must_save_stack`. |
| `routes.rs` | Stack binary routes call `handle()` directly. Pass `Docker` + project as managed state. |
| `builder.rs` | `build_stack` returns `ClientMap`. `add_node`/`make_client` take `&mut ClientMap`. `update_node_from_state` split. |
| `dock.rs` | `restart_node_container` no longer takes `&mut State`. |
| `images/*.rs` | `connect_client` inserts plain clients (Arc for btc, Arc<Mutex> for lnd). `post_client` takes `&ClientMap`. |
| `auto_restart_cron.rs` | Split: read node info, Docker work outside lock, reconnect client. |
| `renew_ssl_cert.rs` | Split: read cert info, S3/HTTP outside, write back. |
| `backup.rs` | `STATE.lock()` → `stack_read()` |
| `app_login.rs` | `STATE.lock()` → `stack_read()` |
| `service/public_ip.rs` | `STATE.lock()` → `stack_read()`/`stack_write()` |
| `conn/boltwall/*.rs` | Functions that take `&mut State` → take extracted info, return changes |
| `conn/swarm/*.rs` | Same — split I/O from state mutation |
| `rocket_utils.rs` | **Unchanged** — `CmdRequest` still used by other binaries |
| `tests/handler_test.rs` | New integration test |

## Implementation Order

1. ~~**Phase 0:** Write integration test (bitcoind handler flow)~~ **DONE** — `tests/handler_test.rs` created. Tests GetConfig, access denied, and full bitcoind GetInfo through a real container. Run: `cargo test --test handler_test -- --nocapture`. Note: requires `RUST_ENV=local` (set automatically in test) to avoid awslogs Docker driver.
2. ~~**Phase 1a:** Make clients cloneable where trivial (derive Clone on Proxy/Relay/Hsmd/Cln, relax ClnRPC `&mut self` → `&self`)~~ **DONE**
3. ~~**Phase 1b:** Define `ClientMap` (`Arc` for BitcoinRPC, `Arc<Mutex>` for LndRPC, plain Clone for rest), add `CLIENTS` static~~ **DONE**
4. ~~**Phase 1c:** Change `STACK` from `Mutex<State>` to `RwLock<Stack>`, add helpers~~ **DONE**
5. ~~**Phase 1d:** Update `handler.rs` — transitional take/put pattern with STACK+CLIENTS~~ **DONE**
6. ~~**Phase 1e:** Update builder, dock, images to work with `ClientMap`~~ **DONE**
7. ~~**Phase 1f:** Update all external STATE consumers (crons, app_login, backup, etc.)~~ **DONE**
8. **Phase 2:** Remove channel infrastructure, routes call `handle()` directly
9. **Phase 3:** Split cron jobs (auto_restart, renew_ssl_cert, auto_updater)
10. Run integration test, verify everything works

## Testing

The integration test from Phase 0 should pass after each phase. Additionally verify:

1. `GetConfig` responds while `UpdateNode` is pulling an image
2. `Login` works while `RestartContainer` is running
3. Two `Lnd::GetInfo` calls to different nodes execute concurrently
4. Auto-updater doesn't block UI commands
5. `ChangePassword` (bcrypt) doesn't block other commands
