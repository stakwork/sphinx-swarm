# SuperAdmin Concurrent Command Handling — Implementation Plan

## Problem

All super admin commands flow through a single `mpsc` channel processed by `spawn_super_handler`, which runs one command at a time. The global `Mutex<Super>` state is held for the **entire duration** of every command — including multi-minute AWS API calls and a hardcoded 40-second sleep.

When `CreateNewEc2Instance` is running, every other command (including `GetConfig` on page load) queues up and never gets a response. The UI hangs with a loading spinner.

If 3 people create swarms simultaneously, the last person waits ~6+ minutes (3x the provisioning time).

## Solution Overview

1. **Remove the `mpsc` channel** — routes call `super_handle()` directly
2. **Replace `Mutex<Super>` with `RwLock<Super>`** — multiple readers proceed in parallel
3. **Split slow commands into phases** — brief lock → drop → slow I/O → brief lock
4. **Split `create_swarm_ec2` specifically** — the biggest bottleneck

## Architecture Change

### Before
```
HTTP request → mpsc channel → single handler loop → STATE.lock() (held entire command) → response
```

### After
```
HTTP request → tokio::spawn(super_handle()) → per-command STATE.read()/write() → response
```

## RwLock Rules

- `STATE.read()` — shared, multiple readers run simultaneously
- `STATE.write()` — exclusive, blocks everyone until dropped
- Readers wait for writers; writers wait for all readers + other writers
- **Goal**: hold write locks for milliseconds (state mutation + YAML save), never during I/O

## File Changes

### 1. `state.rs` — Change Mutex to RwLock

```rust
// Before
pub static STATE: Lazy<Mutex<Super>> = Lazy::new(|| Mutex::new(Default::default()));

// After
pub static STATE: Lazy<RwLock<Super>> = Lazy::new(|| RwLock::new(Default::default()));
```

Update `hydrate()` from `.lock()` to `.write()`.

### 2. `mod.rs` — Remove channel, rewrite `super_handle()`

**Remove:**
- `spawn_super_handler()` function
- `mpsc::channel::<CmdRequest>` creation
- Passing `tx` to `launch_rocket`

**Rewrite `super_handle()`** with per-command locking. Each command arm acquires its own lock(s):

#### Command Categories

**Pure reads (brief `STATE.read()`):**
- `GetAwsInstanceTypes` — no lock needed at all
- `GetInstanceType` — read lock, return
- `GetAnthropicKey` — read lock, return
- `GetChildSwarmCredentials` — read lock, return

**Fast writes (brief `STATE.write()`, mutate, save, return):**
- `AddNewSwarm`
- `UpdateSwarm`
- `DeleteSwarm`
- `SetChildSwarm`
- `AddAnthropicKey`
- `ChangeLightningBotLabel`

**Login/password (bcrypt outside lock):**
- `Login` — read lock to clone user hash, drop, bcrypt verify (no lock)
- `ChangePassword` — read lock to get hash, drop, bcrypt verify+hash (no lock), write lock to save new hash

**Read state then HTTP (read lock briefly, drop, HTTP):**
- `GetChildSwarmConfig` — read lock to `find_swarm_by_host()`, drop, HTTP to child
- `GetChildSwarmContainers` — same pattern
- `Stop/Start/Restart/UpdateChildSwarmContainers` — same pattern
- `GetSwarmChildImageVersions` — same pattern
- `ChangeChildSwarmPassword` — same pattern
- `GetLightningBotsDetails` — same pattern
- `CreateInvoiceForLightningBot` — same pattern
- `UpdateChildSwarmEnv` — same pattern

**No lock needed (pure external I/O):**
- `StopEc2Instance` — AWS call only
- `GetSuperAdminLogs` — Docker logs only
- `RestartSuperAdmin` — HTTP call only
- `GetSslCertExpiry` — cert check only
- `RenewSslCert` — cert renewal only
- `UploadSSlCert` — S3 upload only
- `GetEc2CpuUtilization` — CloudWatch only

**Phase-split commands (see detailed sections below):**
- `GetConfig` — AWS call outside lock, write lock to update
- `CreateNewEc2Instance` — most complex, see dedicated section
- `UpdateAwsInstanceType` — validate under read lock, AWS outside, write lock to apply
- `UpdateChildSwarmPublicIp` — read info, Route53 outside lock, write to save

### 3. `routes.rs` — Remove channel, call `super_handle()` directly

Remove `mpsc::Sender<CmdRequest>` from route handlers. Replace with:
```rust
match super_handle(&proj.0, cmd, tag, &Some(claims.user)).await {
    Ok(res) => Ok(res),
    Err(err) => Ok(fmt_err(&err.to_string())),
}
```

Create super-specific `login` and `update_password` routes (don't share stack binary's routes, which call the stack handler).

Pass project name (`String`) as Rocket managed state instead of the channel sender.

### 4. External STATE consumers — change `.lock()` to `.read()/.write()`

| File | Lock type | Reason |
|------|-----------|--------|
| `checker.rs` (2 calls) | `.read()` | Read-only — gets hosts, sends messages |
| `service/check_domain.rs` | `.read()` | Read-only — checks domain existence |
| `service/log_group_migration.rs` | `.read()` | Read-only — reads instance IDs |
| `util.rs:get_swarm_details_by_id` | `.read()` | Read-only |
| `reserve_swarm.rs` (3 calls) | `.write()` / `.read()` / `.write()` | Already has good phase-splitting pattern |

### 5. `rocket_utils.rs` — Remove `CmdRequest`

Delete the `CmdRequest` struct and its `new()` method. Keep `CORS`, `Error`, `Result`.

## Phase-Split Command Details

### GetConfig

**Current problem**: `get_config()` calls AWS DescribeInstances while holding write lock.

**Split:**
```
Phase 1 (no lock):     get_config_aws_data() — AWS API call
Phase 2 (write lock):  get_config_update_state(&mut state, aws_data) — fast state rebuild
                        put_config_file()
```

The `get_config` function needs splitting into two functions:
- `get_config_aws_data()` — returns the AWS instance HashMap
- `get_config_update_state(state, aws_data)` — processes stacks against AWS data, updates state

### UpdateAwsInstanceType

**Split:**
```
Phase 1 (read lock):   validate_instance_type_update() — validate inputs, extract ec2_id + domains
Phase 2 (no lock):     do_instance_type_aws_update() — EC2 modify + Route53
Phase 3 (write lock):  apply_instance_type_update() — set new instance type, save
```

**IMPORTANT**: Phase 3 must find the swarm by `ec2_instance_id`, NOT by array index. The index may have changed between phases if another command added/removed swarms.

### UpdateChildSwarmPublicIp

**Split:**
```
Phase 1 (read lock):   get_public_ip_route53_info() — determine if Route53 update needed
Phase 2 (no lock):     add_domain_name_to_route53() if needed
Phase 3 (write lock):  apply_public_ip_update() — set new IP, save
```

Phase 3 finds by `swarm.id`, not index. Safe.

### CreateNewEc2Instance (THE BIG ONE)

**Current problem**: Holds write lock for entire duration — daily limit check, vanity address validation (Route53), instance name check (AWS), reserved swarm path (HTTP), EC2 creation, 40-second sleep, Route53 DNS setup, state mutation. Total: 1-3 minutes.

**Split into phases:**

```
Phase 1 — Claim resources (brief write lock):
  - Check/increment daily limit (MUST be atomic — claim, don't just read)
  - Remove anthropic_key[0] from pool (claim it, don't just clone it)
  - Clone reserved_instances info for later use
  - Clone reserved_domains for vanity validation
  - Save config (limit incremented, key removed)
  - Drop lock
  
  If daily limit exceeded → return error (no resources claimed)

Phase 2 — Validate (no lock):
  - Validate instance type
  - Check vanity address format
  - Check vanity address in Route53 (AWS call)
  - Check instance name doesn't exist (AWS call)
  
  If validation fails → need to UNDO resource claims:
    - Write lock → re-add anthropic key, decrement daily limit → save → drop

Phase 3 — Try reserved swarm path (no lock):
  - If reserved swarm available, call handle_assign_reserved_swarm_no_lock()
  - This needs its own splitting (see below)
  - If reserved path fails, continue to Phase 4

Phase 4 — Create EC2 (no lock, THE SLOW PART):
  - create_ec2_instance() — AWS API
  - sleep(40s)
  - get_instance_ip() — AWS API
  - add_domain_name_to_route53() — AWS API

Phase 5 — Save result (brief write lock):
  - Build RemoteStack from EC2 result
  - state.add_remote_stack(new_swarm)
  - Save config
  - Drop lock
```

**Key insight for Phase 1**: Resources (daily limit slot, anthropic key) must be **claimed** (removed from pool) under the write lock, not just read. This prevents two concurrent commands from claiming the same resource. If the command fails later, we undo the claim.

**Splitting `handle_assign_reserved_swarm`:**

This function currently takes `&mut Super` and does HTTP to the reserved swarm instance. It needs the same treatment:

```
Phase 3a (no lock): Read reserved instance info (already cloned in Phase 1)
Phase 3b (no lock): HTTP to child swarm (update password, set env vars)
Phase 3c (write lock): Remove instance from reserved pool, add to stacks, save
```

### Error handling and rollback

When a command claims resources in Phase 1 but fails in Phase 2-4:

```rust
// Phase 1: claim resources
let (anthropic_key, ..) = {
    let mut state = STATE.write().await;
    let key = state.anthropic_keys.as_mut()
        .and_then(|keys| if keys.len() > 1 { Some(keys.remove(0)) } else { None });
    state.ec2_limit.count += 1;
    put_config_file(proj, &state).await;
    (key, ..)
}; // lock dropped

// Phase 2-4: slow work
let result = do_ec2_creation(..).await;

if result.is_err() {
    // Rollback: return claimed resources
    let mut state = STATE.write().await;
    if let Some(key) = anthropic_key {
        state.anthropic_keys.as_mut().map(|keys| keys.insert(0, key));
    }
    state.ec2_limit.count -= 1;
    put_config_file(proj, &state).await;
    return Err(result.unwrap_err());
}

// Phase 5: save result
let mut state = STATE.write().await;
state.add_remote_stack(..);
put_config_file(proj, &state).await;
```

## Critical Rules for Phase-Split Commands

### 1. Never use stale array indexes across lock boundaries

```rust
// BAD: index may shift if another command adds/removes items
let pos = { let s = STATE.read().await; s.stacks.iter().position(|s| s.host == host) };
// ... slow work ...
let mut s = STATE.write().await;
s.stacks[pos].field = value; // pos might be wrong now!

// GOOD: re-find by stable identifier
let mut s = STATE.write().await;
if let Some(swarm) = s.stacks.iter_mut().find(|s| s.ec2_instance_id == id) {
    swarm.field = value;
}
```

### 2. Claim resources atomically, don't just read them

```rust
// BAD: two commands read the same key
let key = { let s = STATE.read().await; s.anthropic_keys[0].clone() };
// Both commands now have the same key!

// GOOD: claim (remove) under write lock
let key = {
    let mut s = STATE.write().await;
    s.anthropic_keys.as_mut().and_then(|k| if k.len() > 1 { Some(k.remove(0)) } else { None })
};
// Only one command gets the key
```

### 3. Always operate on current state in write phase

```rust
// BAD: save stale snapshot
let snapshot = { let s = STATE.read().await; s.clone() };
// ... modify snapshot ...
let mut s = STATE.write().await;
*s = snapshot; // overwrites all changes made since the read!

// GOOD: mutate specific fields on live state
let mut s = STATE.write().await;
s.stacks.push(new_swarm); // pushes to current list
```

### 4. If rollback needed, do it under a new write lock

Don't assume state hasn't changed — the rollback should be idempotent (e.g., "remove key X if present" rather than "insert key at position 0").

## Testing

Write tests that verify:
1. `GetConfig` responds while `CreateNewEc2Instance` is in progress
2. Multiple concurrent `CreateNewEc2Instance` calls don't claim the same anthropic key
3. `AddNewSwarm` during `CreateNewEc2Instance` doesn't lose either swarm
4. Daily limit is correctly enforced under concurrent access
5. Rollback correctly returns resources on failure

## Implementation Order

1. Change `Mutex` to `RwLock` in `state.rs`
2. Remove channel infrastructure (`spawn_super_handler`, `CmdRequest`, channel creation)
3. Rewrite routes to call `super_handle()` directly (including super-specific login/password routes)
4. Update external consumers (checker, check_domain, log_group_migration, reserve_swarm, util)
5. Rewrite `super_handle()` with per-command locking
6. Split `get_config` into `get_config_aws_data()` + `get_config_update_state()`
7. Split `UpdateAwsInstanceType` into validate/AWS/apply phases
8. Split `UpdateChildSwarmPublicIp` into read/Route53/apply phases
9. Split `create_swarm_ec2` into claim/validate/create/save phases (biggest change)
10. Split `handle_assign_reserved_swarm` for the reserved swarm path
11. Test everything
