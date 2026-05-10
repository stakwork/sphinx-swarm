# Plan: Stop publishing host ports when `PORT_BASED_SSL=1`

## The problem

`src/utils.rs::filter_out_reserved_ports_if_using_port_based_ssl` hard-codes a
list of "reserved" ports:

```rust
fn filter_out_reserved_ports_if_using_port_based_ssl(ports: Vec<String>) -> Vec<String> {
    if !is_using_port_based_ssl() {
        return ports;
    }
    ports.into_iter().filter(|p| {
        p != "7799" && p != "3355" && p != "8000" && p != "3100"
            && p != "6000" && p != "8444" && p != "9000" && p != "3333"
    }).collect()
}
```

These are the ports declared as Traefik entrypoints in `second-brain-2.yml`:

```yaml
- "--entrypoints.port7799.address=:7799"
- "--entrypoints.port3355.address=:3355"
- "--entrypoints.port6000.address=:6000"
- "--entrypoints.port8000.address=:8000"
- "--entrypoints.port3100.address=:3100"
- "--entrypoints.port8444.address=:8444"
- "--entrypoints.port8800.address=:8800"
- "--entrypoints.port9000.address=:9000"
- "--entrypoints.port3333.address=:3333"
```

### Why this filter exists

When `PORT_BASED_SSL=1`:

- The `load_balancer` (Traefik) container publishes those ports to the host.
- Every backend container is reached via Traefik on the docker network, NOT
  directly from the host.
- If a backend container ALSO publishes its own port to the host, the host
  port collides with Traefik's published port and bollard fails to start it.

### Why it's bad

The list is hard-coded, so any new image (e.g. the new Bifrost image on `8181`)
with port-based SSL silently fails to start until someone remembers to edit
`utils.rs`. Three places must stay in sync (image module, the YAML
entrypoints/ports, and this list).

## The real insight

The problem isn't "filter reserved ports". The problem is **publishing host
ports at all**. When `PORT_BASED_SSL=1`, Traefik owns the host network. No
swarm-managed container should ever publish to the host.

The internal port still gets exposed via `exposed_ports(...)`, which is what
makes the container reachable on the docker network — that's all Traefik
needs.

## The fix

Make `host_port` itself a no-op when `PORT_BASED_SSL=1`:

```rust
pub fn host_port(ports_in: Vec<String>) -> Option<PortMap> {
    if is_using_port_based_ssl() {
        // Traefik owns the host network; nothing else publishes to it.
        return Some(PortMap::new());
    }
    let mut ports = PortMap::new();
    for port in ports_in {
        ports.insert(
            tcp_port(&port),
            Some(vec![PortBinding {
                host_port: Some(port.to_string()),
                host_ip: None,
            }]),
        );
    }
    Some(ports)
}
```

Then delete `filter_out_reserved_ports_if_using_port_based_ssl` and the
hard-coded list entirely.

### Why this is safe (verified)

`PORT_BASED_SSL=1` is only used by the `second_brain` and `graph_mindset`
stacks (`secondbrain.rs` and `graphmindset.rs`). Their full image set is:

- second_brain: navfiber, graphmindset, neo4j, boltwall, jarvis, redis,
  repo2graph, stakgraph, quickwit, vector, hive-relay, optional bot, optional
  llama.
- graph_mindset: bot, navfiber, graphmindset, neo4j, boltwall, jarvis, redis,
  repo2graph, stakgraph.

Every public-facing one has Traefik labels (boltwall, stakgraph, repo2graph,
graphmindset, navfiber, hive-relay, vector). The rest (neo4j, redis, quickwit,
jarvis, bot, llama) are only ever reached on the docker network — they don't
need a host port at all.

Crucially, **neither stack contains CLN, LND, bitcoind, relay, proxy, or any
other image that needs a raw-TCP host port (peering 9735, MQTT 8883, etc.).**
Those live in `config.yml`, `sphinx.yml`, `bin/v1`, `bin/cln`, `bin/sphinx`
and never run with `PORT_BASED_SSL=1`. So a global "publish nothing when
port-based-ssl is on" rule is correct.

If we ever do mix the two — e.g. add CLN to a port-based-ssl stack — we'd
need to escape this rule for that container. That's a future problem; flag it
in the comment on `host_port`.

## Step-by-step

1. **Update `host_port` in `src/utils.rs`** to return an empty `PortMap` when
   `is_using_port_based_ssl()` is true. Add a comment explaining the
   invariant: "Traefik owns the host network; backend containers reach each
   other via the docker network only."
2. **Delete** `filter_out_reserved_ports_if_using_port_based_ssl` and remove
   its call site (the `ports_in = filter_out_…(ports_in);` line).
3. **Add a unit test** for `host_port`:
   - With `PORT_BASED_SSL` unset: `host_port(vec!["8181".into()])` returns a
     `PortMap` with one binding to host port 8181.
   - With `PORT_BASED_SSL=1`: same input returns an empty `PortMap`.
   - Restore env var afterwards (use a mutex if other tests touch env).
4. **Verify Bifrost boots** under `second-brain-2.yml`:
   - Add `8181` to `--entrypoints.port8181.address=:8181` and `8181:8181` in
     the `ports:` list of `second-brain-2.yml` (so Traefik publishes it).
   - Add `traefik_labels_port_based_ssl` wiring on the Bifrost image (the
     existing `traefik_labels` call in `bifrost.rs:89` already routes through
     `traefik_labels_port_based_ssl` when `PORT_BASED_SSL=1`).
   - Confirm the container starts and `https://bifrost.<host>:8181` resolves.
5. **Document** in `CLAUDE.md` (under "Important Gotchas"):
   > When `PORT_BASED_SSL=1`, Traefik publishes all public ports to the host.
   > No swarm-managed container publishes its own ports — they're reached
   > over the docker network only. To expose a new image publicly, add a
   > Traefik entrypoint + port mapping in `second-brain-2.yml` and call
   > `traefik_labels` (which auto-switches to port-based labels) on the image.
6. **Follow-up (separate PR, optional)**: generate the
   `--entrypoints.portNNNN.address=:NNNN` lines and `ports:` list in
   `second-brain-2.yml` from the active stack config so adding a new image
   touches only the image module.

## Acceptance criteria

- Adding a new image (Bifrost or anything else) does NOT require editing any
  port allow/deny list in `utils.rs`.
- `cargo test` passes; new tests cover `host_port` with and without the env
  var.
- `filter_out_reserved_ports_if_using_port_based_ssl` and its hard-coded list
  are gone.
- Existing non-port-based-ssl stacks (config.yml, sphinx.yml, cln, v1, btc,
  tome) behave identically.
- second_brain and graph_mindset stacks come up under `second-brain-2.yml`,
  publicly reachable via Traefik on the configured ports.
