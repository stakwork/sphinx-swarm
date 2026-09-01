# Host Storage Telemetry — `GetHostStorage`

Read-only host-level storage telemetry, exposed through the existing authenticated
admin command endpoint. No new route, no new endpoint.

```
curl -H "x-jwt: $JWT" --get https://<swarm>/api/cmd \
  --data-urlencode 'tag=SWARM' \
  --data-urlencode 'txt={"cmd":"GetHostStorage"}'
```

(URL-encoding `txt` is required — the payload is JSON and `/api/cmd` reads it from
the `txt` query parameter. Note the `--get` flag; the request must be a GET.)

## Sources

The command combines three independent collectors (each capped at **8s**, whole
collection bounded by **15s** — well inside the 60s `REQUEST_TIMEOUT_DURATION_IN_SEC`):

1. **`node_exporter`** (primary) — scrapes the co-located `node_exporter`
   sidecar (`network_mode: host`, `--path.rootfs=/host`) at
   `http://{docker-gateway}:9100/metrics`. The gateway is read from the
   container's own `/proc/net/route`; overridable with `NODE_EXPORTER_URL`,
   which is read **once at startup** and must be loopback or the resolved
   docker gateway (anything else is rejected). Figures genuinely describe the
   host filesystems.
2. **`container_bind`** (fallback) — when the scrape fails, a `statvfs` on the
   container's `/vol` path (a bind of the host's `/home/admin/vol`).
3. **`none`** — neither is available. `filesystems: []` and
   `host_visible: false`; volume and Neo4j data still come back.

Docker volume sizes always come from the Docker daemon (`GET /system/df`), and
the daemon's storage root from `GET /info`.

## Response shape (v1)

```json
{ "host_visible": true,
  "source": "node_exporter",
  "collected_at": 1730000000,
  "cached": false,
  "filesystems": [
    {"mount": "/", "device": "/dev/nvme0n1p1", "fstype": "ext4",
     "total_bytes": 0, "used_bytes": 0, "free_bytes": 0, "describes_host": true}
  ],
  "docker_root_dir": "/var/lib/docker",
  "docker_root_filesystem": "/",
  "volumes": [{"name": "neo4j.sphinx", "size_bytes": 0, "size_known": true}],
  "neo4j": {"volumes": ["neo4j.sphinx"], "size_bytes": 0, "size_known": true},
  "errors": [{"collector": "volumes", "reason": "docker df timed out after 8s"}] }
```

### Field semantics

- **`host_visible`** is a testable predicate: `true` **iff** at least one entry
  in `filesystems[]` has `describes_host: true`. It is never derived from "a
  mount path exists". An entry is `describes_host: true` only when its figures
  were read from a rootfs-scoped `node_exporter` (`source: "node_exporter"`) or
  from a `statvfs` on a path that is a bind of a host directory
  (`source: "container_bind"`). The container's own overlay `/` is **never**
  reported as host data.
- **`source`** is one of `"node_exporter" | "container_bind" | "none"`.
- **`volumes[].size_bytes` is nullable.** Docker returns `-1` (or omits
  `usage_data`) for volumes it did not compute; that maps to
  `size_bytes: null, size_known: false` plus an `errors[]` entry — never a
  fabricated `0`.
- **`neo4j` is `null`** when no Neo4j node exists in the stack (a valid,
  non-error response). When present it lists *all* named volumes attributed to
  the Neo4j node and sums them. In this repo Neo4j mounts exactly one named
  volume (`<hostname>.sphinx` at `/data`); the plugins and `apoc.conf` are
  copied into the container filesystem (`docker cp`), not volumes. **Host-path
  bind mounts are excluded** from the attribution. `neo4j.size_bytes` is
  derived by lookup into the same map that produced `volumes[]`, so the two can
  never disagree; it is summed only when every volume's size is known.
- **`docker_root_dir` / `docker_root_filesystem`** come from `Docker::info()` and
  a longest-prefix match against `filesystems[].mount`, so an operator can tell
  which reported `free_bytes` actually governs the Neo4j volume — the daemon's
  storage root is frequently a separate device from `/`.
- **`errors[]` elements are objects**: `{"collector": "filesystems" | "volumes"
  | "neo4j" | "docker_info", "reason": "<string>"}`.
- **Partial failures return 200** with a fully-formed object; collector failures
  are folded into `errors[]` and never fail the request.
- **`used_bytes` / `free_bytes`**: `used = total - free` (free, not available,
  so root-reserved space counts as used); `free_bytes` is the *available* figure
  (`node_filesystem_avail_bytes` / statvfs `bavail`) — what a non-root writer
  would actually get.
- `filesystems[]` excludes virtual filesystems (`tmpfs`, `overlay`, `squashfs`,
  `devtmpfs`) and node_exporter's own rootfs-view mountpoints under
  `/host/proc`, `/host/sys`, `/host/run`.

## Timing & caching

- **Collection budget: 15s** (each collector individually capped at 8s;
  collectors run concurrently).
- **Cache TTL: 60s.** Within the TTL the last result is served as-is with
  `cached: true` and the **original** `collected_at` — use `collected_at` to
  detect staleness. Concurrent calls single-flight into one collection rather
  than stacking volume scans on a host that may already be near disk
  exhaustion. Cache hits log nothing.

## Compatibility policy

**Additive-only.** New fields may appear; existing field names, types and
nullability do not change within this major version. Any breaking change ships
as a future `GetHostStorageV2`.

## Errors

A collector failure is *not* a request failure. Transport-level failures (bad
JWT, malformed payload, outer timeout, access denied) surface as
`{"stack_error": "..."}` — handle both shapes. Collector-level failures live in
`errors[]`.

## Access control

Behind the existing `x-jwt` admin guard (`auth::AdminJwtClaims`). Admins,
sub-admins and super-admins may call it; unauthenticated callers are denied.
Host-capacity details are deliberately visible to sub-admins (they already
read container listings and API tokens).

## Related

`SwarmCmd::GetEc2CpuUtilization` (super-admin binary) reads EC2 CPU utilization
from CloudWatch — EC2-only and super-admin-only. Disk telemetry lives on the
per-swarm command surface instead because it must work on **any** deployment
and be readable with the swarm's **own** admin credential.

## Deployment notes

`node_exporter` is defined in `sphinx.yml`, `second-brain.yml`,
`second-brain-2.yml`, `docker-compose.yml` and `sphinxv2.yml`. Swarms deployed
from composes without it still work via the `/vol` fallback (or report
`host_visible: false` with an explanatory `errors[]` entry). `superadmin.yml`
and `config.yml` hosts are not swarm data hosts and are out of scope.
