# SuperAdmin Concurrent Command Handling — Implementation Plan

## Problem

All super admin commands flow through a single `mpsc` channel processed by `spawn_super_handler`, which runs one command at a time. The global `Mutex<Super>` state is held for the **entire duration** of every command — including multi-minute AWS API calls and a hardcoded 40-second sleep.

When `CreateNewEc2Instance` is running, every other command (including `GetConfig` on page load, `check-domain` on the frontend) queues up and never gets a response. The UI hangs with a loading spinner.

If 3 people create swarms simultaneously, the last person waits ~6+ minutes (3x the provisioning time).

## Core Principle

Treat state like a database. Short-lived reads and writes — never hold a lock during I/O.

```rust
// CURRENT: hold the lock for the entire command (minutes)
let mut state = STATE.lock().await;
do_aws_stuff(&mut state).await;   // lock held during slow I/O
state.save();

// NEW: brief lock → slow work → brief lock
let info = STATE.read(|s| s.get_host("swarm-1").clone());  // milliseconds
let result = do_aws_stuff(info).await;                       // no lock, takes as long as it wants
STATE.write(|s| s.set_result("swarm-1", result));            // milliseconds
```

Every command follows this pattern. No exceptions. The only question per command is whether it needs a read, a write, or a read-then-work-then-write.

## State Access Helpers

Add these to `state.rs` to make the pattern easy to use everywhere:

```rust
use rocket::tokio::sync::RwLock;

pub static STATE: Lazy<RwLock<Super>> = Lazy::new(|| RwLock::new(Default::default()));

/// Read something from state. Lock held only for the duration of `f`.
pub async fn state_read<F, T>(f: F) -> T
where
    F: FnOnce(&Super) -> T,
{
    let state = STATE.read().await;
    f(&state)
}

/// Mutate state and save to disk. Lock held only for the duration of `f`.
pub async fn state_write<F, T>(proj: &str, f: F) -> T
where
    F: FnOnce(&mut Super) -> T,
{
    let mut state = STATE.write().await;
    let result = f(&mut state);
    put_config_file(proj, &state).await;
    result
}
```

Now every command is trivially correct:

```rust
// Read: multiple readers in parallel, no blocking
let host = state_read(|s| s.find_swarm_by_host("x").cloned()).await;

// Write: brief exclusive lock, auto-saves
state_write(proj, |s| s.stacks.push(new_swarm)).await;

// Atomic claim: take a resource so no one else can
let key = state_write(proj, |s| {
    s.anthropic_keys.as_mut().and_then(|k| if k.len() > 1 { Some(k.remove(0)) } else { None })
}).await;
```

## Architecture Change

### Before

```
HTTP request → mpsc channel → single handler loop → STATE.lock() (held entire command) → response
```

### After

```
HTTP request → tokio::spawn(super_handle()) → state_read()/state_write() as needed → response
```

## Command Patterns

Every command falls into one of these patterns:

### Pattern 1: Read and return

```rust
let result = state_read(|s| s.get_thing().clone()).await;
Ok(serde_json::to_string(&result)?)
```

Commands: `GetInstanceType`, `GetAnthropicKey`, `GetChildSwarmCredentials`, `GetAwsInstanceTypes` (no lock needed at all)

### Pattern 2: Mutate and return

```rust
state_write(proj, |s| s.add_swarm(info)).await;
Ok(ok_string())
```

Commands: `AddNewSwarm`, `UpdateSwarm`, `DeleteSwarm`, `SetChildSwarm`, `AddAnthropicKey`, `ChangeLightningBotLabel`

### Pattern 3: Read state, do I/O, return (no write needed)

```rust
let (host, creds) = state_read(|s| {
    let sw = s.find_swarm_by_host(&id.host);
    (sw.host.clone(), sw.credentials.clone())
}).await;
let result = http_call(&host, &creds).await?;
Ok(serde_json::to_string(&result)?)
```

Commands: `GetChildSwarmConfig`, `GetChildSwarmContainers`, `Stop/Start/Restart/UpdateChildSwarmContainers`, `GetSwarmChildImageVersions`, `ChangeChildSwarmPassword`, `GetLightningBotsDetails`, `CreateInvoiceForLightningBot`, `UpdateChildSwarmEnv`

### Pattern 4: No state needed (pure external I/O)

```rust
let result = aws_call(instance_id).await?;
Ok(serde_json::to_string(&result)?)
```

Commands: `StopEc2Instance`, `GetSuperAdminLogs`, `RestartSuperAdmin`, `GetSslCertExpiry`, `RenewSslCert`, `UploadSSlCert`, `GetEc2CpuUtilization`

### Pattern 5: Read state, do I/O, write result back

```rust
// read what we need (milliseconds)
let info = state_read(|s| s.get_swarm_info(&id)).await;
// slow work (seconds/minutes, no lock)
let result = aws_call(info).await?;
// write result back (milliseconds, operates on CURRENT state)
state_write(proj, |s| {
    if let Some(sw) = s.find_swarm_mut(&id) {
        sw.apply_result(result);
    }
}).await;
```

Commands: `GetConfig`, `UpdateAwsInstanceType`, `UpdateChildSwarmPublicIp`

### Pattern 6: Claim resources, do I/O, write result (with rollback)

This is only for `CreateNewEc2Instance` — the one command that needs to reserve shared resources (anthropic key, daily limit slot) before doing slow work.

```rust
// 1. Claim resources atomically (milliseconds)
let claimed = state_write(proj, |s| {
    if s.ec2_limit_exceeded() { return Err("daily limit") }
    let key = s.anthropic_keys.as_mut()
        .and_then(|k| if k.len() > 1 { Some(k.remove(0)) } else { None });
    s.ec2_limit.count += 1;
    Ok(ClaimedResources { key, .. })
}).await?;

// 2. Slow work — no lock (minutes)
let result = create_ec2_and_setup(claimed).await;

// 3a. On success: save the new swarm (milliseconds)
if let Ok(new_swarm) = result {
    state_write(proj, |s| s.add_remote_stack(new_swarm)).await;
}
// 3b. On failure: return claimed resources (milliseconds)
if let Err(e) = result {
    state_write(proj, |s| {
        if let Some(key) = claimed.key {
            s.anthropic_keys.as_mut().map(|k| k.insert(0, key));
        }
        s.ec2_limit.count -= 1;
    }).await;
    return Err(e);
}
```

### Login/password (bcrypt outside lock)

```rust
// Login: read hash, verify outside lock
let hash = state_read(|s| s.get_user_hash(username).cloned()).await;
let valid = bcrypt::verify(password, &hash)?;  // CPU-intensive, no lock

// ChangePassword: read hash, verify+hash outside, write new hash
let hash = state_read(|s| s.get_user_hash(username).cloned()).await;
bcrypt::verify(old_password, &hash)?;
let new_hash = bcrypt::hash(new_password)?;
state_write(proj, |s| s.set_user_hash(username, new_hash)).await;
```

## Safety Rules

These prevent subtle concurrency bugs. All enforced naturally by the `state_read`/`state_write` helpers.

### 1. Never use stale array indexes across lock boundaries

The current code uses `.position()` + `stacks[pos]` extensively (28 index accesses, 18 position lookups). This is safe today because the lock is held the whole time. After the refactoring, any function that gets split — where the position is found in one lock scope and used in a later lock scope — must switch to ID-based lookup in the write phase.

**Within a single `state_read`/`state_write` closure**, index access is fine — the lock is held, nothing can shift. The danger is only across closure boundaries.

Specific functions that need this fix during splitting:
- `update_aws_instance_type` (`util.rs`) — uses `unwrapped_swarm_pos` index across what will become separate lock scopes
- `update_child_swarm_public_ip` (`service/child_swarm/update_public_ip.rs`) — uses `swarm_pos` index
- `create_swarm_ec2` (`util.rs`) — uses `swarm_pos` in `update_swarm`
- `UpdateSwarm` handler (`mod.rs:256`) — uses position to index into `stacks[ui]`

```rust
// BAD: index may shift between read and write
let pos = state_read(|s| s.stacks.iter().position(|s| s.host == host)).await;
state_write(proj, |s| s.stacks[pos].field = value).await; // pos might be wrong!

// GOOD: re-find by stable identifier in the write closure
state_write(proj, |s| {
    if let Some(sw) = s.stacks.iter_mut().find(|s| s.ec2_instance_id == id) {
        sw.field = value;
    }
}).await;
```

### 2. Claim resources atomically, don't just read them

```rust
// BAD: two commands read the same key
let key = state_read(|s| s.anthropic_keys[0].clone()).await;

// GOOD: claim (remove) under write
let key = state_write(proj, |s| s.anthropic_keys.as_mut()
    .and_then(|k| if k.len() > 1 { Some(k.remove(0)) } else { None })
).await;
```

### 3. Never snapshot-then-replace

```rust
// BAD: overwrites all concurrent changes
let mut snapshot = state_read(|s| s.clone()).await;
snapshot.stacks.push(new_swarm);
state_write(proj, |s| *s = snapshot).await; // clobbers everything since the read!

// GOOD: mutate specific fields on current state
state_write(proj, |s| s.stacks.push(new_swarm)).await;
```

### 4. Rollbacks should be idempotent

Don't assume state hasn't changed — "remove key X if present" not "insert key at position 0".

## File Changes

### 1. `state.rs` — RwLock + helpers

- Change `Mutex<Super>` to `RwLock<Super>`
- Add `state_read()` and `state_write()` helpers
- Update `hydrate()` from `.lock()` to `.write()`

### 2. `mod.rs` — Remove channel, rewrite `super_handle()`

**Remove:**
- `spawn_super_handler()` function
- `mpsc::channel::<CmdRequest>` creation
- Passing `tx` to `launch_rocket`

**Rewrite `super_handle()`**: Remove the single `STATE.lock()` at the top. Each match arm uses `state_read`/`state_write` as needed per the patterns above.

**`access()` check**: Currently reads `state.users` from the already-locked state. After the refactoring, this needs a brief `state_read` at the top of `super_handle()` before dispatching to the command:

```rust
let allowed = state_read(|s| access(&cmd, s, user_id)).await;
if !allowed { return Err(anyhow!("access denied")); }
```

**`must_save_stack` pattern**: Currently a boolean flag checked at the end of `super_handle()` to decide whether to call `put_config_file`. This goes away — `state_write()` auto-saves, so commands that mutate state just use `state_write` and the save happens inline.

### 3. `routes.rs` — Remove channel, call `super_handle()` directly

Remove `mpsc::Sender<CmdRequest>` from route handlers. Replace with:

```rust
match super_handle(&proj.0, cmd, tag, &Some(claims.user)).await {
    Ok(res) => Ok(res),
    Err(err) => Ok(fmt_err(&err.to_string())),
}
```

Create super-specific `login` and `update_password` routes. The shared versions in `src/routes.rs` use `mpsc::Sender<CmdRequest>` and are still used by every other binary (stack, tome, cln, v1, demo) — do NOT modify them. Copy the route signatures into super's `routes.rs` and rewrite them to call `super_handle()` directly. Stop importing `login` and `update_password` from `sphinx_swarm::routes`.

Keep importing `all_options`, `events`, `logs`, `logstream`, `refresh_jwt` from `sphinx_swarm::routes` — these don't use the channel.

Pass project name (`String`) as Rocket managed state instead of the channel sender.

### 4. `rocket_utils.rs` — Stop importing `CmdRequest` (do NOT delete it)

`CmdRequest` in `src/rocket_utils.rs` is used by every other binary (stack, tome, cln, v1, demo) and the shared `src/routes.rs`. Do not modify it. Super simply stops importing it.

### 5. External STATE consumers — use `state_read`/`state_write`

| File                              | Change                                                                                                       |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `checker.rs` (2 calls)            | `state_read()` — already drops lock early, just switch to helper                                             |
| `service/check_domain.rs`         | `state_read()` — read-only                                                                                   |
| `service/log_group_migration.rs`  | `state_read()` — read-only                                                                                   |
| `util.rs:get_swarm_details_by_id` | Split: `state_read()` to get host/creds, drop, HTTP call outside, return. Currently holds lock across HTTP.  |
| `reserve_swarm.rs` (3 calls)      | Already has good split pattern — just switch `.lock()` to `state_read()`/`state_write()`                     |

## Functions That Need Splitting

These functions currently take `&mut Super` and do I/O. They need to be broken into "extract info" + "do I/O" + "write result" parts:

| Function                       | Current signature            | What to split out                                                  |
| ------------------------------ | ---------------------------- | ------------------------------------------------------------------ |
| `get_config`                   | `&mut Super`                 | AWS `DescribeInstances` call → `get_config_aws_data()` (no state)  |
| `create_swarm_ec2`             | `&CreateEc2InstanceInfo, &mut Super` | All AWS/HTTP calls → separate functions taking extracted info |
| `update_aws_instance_type`     | `&mut Super`                 | EC2 modify + Route53 calls → `do_instance_type_aws_update()`       |
| `update_child_swarm_public_ip` | `&mut Super`                 | Route53 call → separate function                                   |
| `handle_assign_reserved_swarm` | `&mut Super`                 | HTTP calls to child swarm → separate function                      |
| `get_swarm_details_by_id`      | (locks STATE internally)     | HTTP call to child swarm → separate function                       |

## Implementation Order

1. Add `state_read()`/`state_write()` helpers to `state.rs`, change `Mutex` to `RwLock`
2. Remove channel infrastructure (`spawn_super_handler`, `CmdRequest`, channel creation)
3. Rewrite routes to call `super_handle()` directly (including super-specific login/password routes)
4. Update external consumers (`checker`, `check_domain`, `log_group_migration`, `reserve_swarm`, `util`)
5. Rewrite `super_handle()` — each match arm uses `state_read`/`state_write` per the patterns
6. Split `get_config` (AWS call outside lock)
7. Split `update_aws_instance_type` (validate/AWS/apply)
8. Split `update_child_swarm_public_ip` (read/Route53/apply)
9. Split `create_swarm_ec2` + `handle_assign_reserved_swarm` (claim/validate/create/save with rollback)
10. Test everything

## Testing

Verify:

1. `GetConfig` and `check-domain` respond while `CreateNewEc2Instance` is in progress
2. Multiple concurrent `CreateNewEc2Instance` calls don't claim the same anthropic key
3. `AddNewSwarm` during `CreateNewEc2Instance` doesn't lose either swarm
4. Daily limit is correctly enforced under concurrent access
5. Rollback correctly returns resources on failure
