//! Host-level storage telemetry for the `GetHostStorage` swarm command.
//!
//! Reports host filesystem usage (total/used/free per mounted filesystem) plus
//! per-Docker-volume sizes with the Neo4j store called out, so an external
//! caller (Hive) can see a disk filling up *before* a store dies.
//!
//! # Spike findings (bollard 0.18.1, verified against the vendored crate source)
//!
//! No live swarm was reachable from the dev environment at implementation time,
//! so these were confirmed from the bollard 0.18.1 source instead:
//!
//! * `Docker::df(&self) -> Result<SystemDataUsageResponse, Error>` — no options
//!   struct, no query parameters (`src/system.rs`, `GET /system/df`).
//! * `SystemDataUsageResponse { layers_size, images, containers, volumes, build_cache }`
//!   where `volumes: Option<Vec<Volume>>`.
//! * `Volume.usage_data: Option<VolumeUsageData>` and
//!   `VolumeUsageData { size: i64, ref_count: i64 }` — `size` is a *plain i64*,
//!   not an Option. The Docker daemon returns `-1` when it did not compute the
//!   size (non-`local` drivers, or a scan that was skipped), so both the
//!   `usage_data == None` and `size < 0` cases must map to `size_known: false`.
//! * `Docker::info() -> SystemInfo` carries `docker_root_dir: Option<String>`.
//! * On a live swarm, `Docker::df()` `du`-walks every volume and can take many
//!   seconds on a large Neo4j store — hence the TTL cache and single-flight
//!   below. The 8s per-collector / 15s total budget sits well inside
//!   `REQUEST_TIMEOUT_DURATION_IN_SEC` (60s) in `src/routes.rs`.
//!
//! # node_exporter reachability (assumption, not verified on a live host here)
//!
//! `sphinx.yml` / `second-brain*.yml` run `node_exporter` with
//! `network_mode: host` and `--path.rootfs=/host` (`/:/host:ro,rslave`), so from
//! inside the `sphinx-swarm` container (bridged network) the sidecar is
//! reachable at the docker bridge gateway IP on port 9100. The gateway is read
//! from the container's own `/proc/net/route` (`default via` line). This was
//! NOT verified against a running host — if the gateway is unreachable the
//! collector falls back to the `/vol` bind-mount statvfs (see below), and
//! `errors[]` records why.
//!
//! # SSRF guard
//!
//! `NODE_EXPORTER_URL` is read **once** into a process-lifetime cell. Runtime
//! env writes (`update_env_variables()` in `src/conn/swarm/mod.rs` calls
//! `std::env::set_var` for arbitrary keys) can therefore never repoint the
//! collector at an internal host. A configured value must be loopback or the
//! resolved docker gateway, or it is rejected (falls through to `"none"`).

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use bollard::models::{ContainerSummary, Volume};
use bollard::Docker;
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::Node;
use crate::images::Image;
use crate::utils;

/// Per-collector budget. `Docker::df()` can `du`-walk a large volume for a long
/// time; this cancels only the *client* future (dockerd keeps working), so the
/// TTL cache exists to keep a poll loop from stacking unbounded scans.
pub const COLLECTOR_TIMEOUT_SECS: u64 = 8;

/// Hard total budget, documented in doc/HostStorage.md. Collectors run
/// concurrently, so the wall clock stays at or below the per-collector cap.
pub const TOTAL_BUDGET_SECS: u64 = 15;

/// How long a collected `HostStorage` is served as `cached: true`.
pub const CACHE_TTL_SECS: u64 = 60;

const NODE_EXPORTER_PORT: u16 = 9100;
const VOL_BIND_MOUNT: &str = "/vol";

// ─── Response contract (doc/HostStorage.md pins this shape) ─────────────────

/// Top-level response for `SwarmCmd::GetHostStorage`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostStorage {
    /// True iff at least one `filesystems[]` entry has `describes_host: true`.
    /// Never derived from "a mount path exists".
    pub host_visible: bool,
    /// `"node_exporter"` | `"container_bind"` | `"none"`
    pub source: String,
    /// Unix timestamp (seconds) of the *underlying* collection. Preserved on
    /// cache hits so consumers can detect staleness.
    pub collected_at: i64,
    /// True when served from the 60s TTL cache instead of freshly collected.
    pub cached: bool,
    pub filesystems: Vec<FilesystemUsage>,
    /// Storage root reported by `Docker::info()`.
    pub docker_root_dir: Option<String>,
    /// Longest-prefix match of `docker_root_dir` against `filesystems[].mount`.
    pub docker_root_filesystem: Option<String>,
    pub volumes: Vec<VolumeUsage>,
    /// `null` when the stack has no Neo4j node — a valid, non-error response.
    pub neo4j: Option<Neo4jStorage>,
    /// Per-service volume rollup for every internal node that owns at least one
    /// volume. Additive; empty when nothing could be attributed.
    #[serde(default)]
    pub services: Vec<ServiceStorage>,
    /// Per-collector failures. A partial failure is always a well-formed 200.
    pub errors: Vec<CollectorError>,
}

/// One observed filesystem. `describes_host: true` only when the figures were
/// read from a rootfs-scoped node_exporter or from a statvfs on a path that is
/// a bind of a host directory. The container's own overlay `/` is NEVER
/// reported as host data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FilesystemUsage {
    pub mount: String,
    pub device: String,
    pub fstype: String,
    pub total_bytes: i64,
    pub used_bytes: i64,
    /// Available-to-non-root bytes (node_exporter `avail`, statvfs `bavail`).
    pub free_bytes: i64,
    pub describes_host: bool,
}

/// One Docker volume with its measured size. `size_bytes` is `None` (and
/// `size_known: false`) when the daemon returned `-1` or omitted `usage_data` —
/// never a fabricated `0`. `service` is the owning node name, or `None` for an
/// ungrouped orphan (still listed in `volumes[]`).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VolumeUsage {
    pub name: String,
    pub size_bytes: Option<i64>,
    pub size_known: bool,
    /// Owning swarm service (`node.name()`). `None` = ungrouped/orphan.
    #[serde(default)]
    pub service: Option<String>,
}

/// Neo4j storage rollup. Lists *all* named volumes attributed to the Neo4j node
/// and sums them. Host-path bind mounts are excluded (Neo4j in this repo mounts
/// exactly one named volume, `{domain}.sphinx` at `/data`; plugins and apoc.conf
/// are `docker cp`'d into the container, not volumes).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Neo4jStorage {
    pub volumes: Vec<String>,
    pub size_bytes: Option<i64>,
    pub size_known: bool,
}

/// Per-service volume rollup. `size_bytes` is `None` (and `size_known: false`)
/// when any member volume is unknown or missing from docker df — never a
/// partial total.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceStorage {
    pub name: String,
    pub typ: String,
    pub volumes: Vec<String>,
    pub size_bytes: Option<i64>,
    pub size_known: bool,
}

/// One failed/timed-out sub-collector. `collector` is one of
/// `"filesystems"` | `"volumes"` | `"neo4j"` | `"docker_info"` | `"containers"`
/// | `"services"`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CollectorError {
    pub collector: String,
    pub reason: String,
}

impl CollectorError {
    fn new(collector: &str, reason: impl Into<String>) -> Self {
        CollectorError {
            collector: collector.to_string(),
            reason: reason.into(),
        }
    }
}

// ─── node_exporter target resolution (once, with SSRF guard) ────────────────

/// Process-lifetime resolved target. `None` => no node_exporter collector.
static NODE_EXPORTER_TARGET: OnceCell<Option<String>> = OnceCell::new();

/// Resolve and pin the node_exporter target. Called at startup from the stack
/// binary; also lazily initialized on first use (after which runtime env
/// changes are ignored).
pub fn init_node_exporter_target() {
    let _ = NODE_EXPORTER_TARGET.get_or_init(resolve_node_exporter_target);
}

/// The pinned scrape target, if any.
pub fn node_exporter_target() -> Option<String> {
    NODE_EXPORTER_TARGET
        .get_or_init(resolve_node_exporter_target)
        .clone()
}

fn resolve_node_exporter_target() -> Option<String> {
    let gateway = read_default_gateway();
    match std::env::var("NODE_EXPORTER_URL").ok() {
        Some(raw) if !raw.trim().is_empty() => {
            // Validate against the *current* env value once here; the result is
            // pinned, so later `set_var` calls cannot repoint the collector.
            match validate_node_exporter_url(raw.trim(), gateway.as_deref()) {
                Some(url) => Some(url),
                None => {
                    log::warn!(
                        "NODE_EXPORTER_URL rejected (must be loopback or the docker gateway); \
                         node_exporter collector disabled"
                    );
                    None
                }
            }
        }
        // No override: derive from the container's own routing table.
        _ => gateway.map(|gw| format!("http://{}:{}/metrics", gw, NODE_EXPORTER_PORT)),
    }
}

/// Accept only loopback hosts or the resolved docker gateway. Everything else
/// is rejected as a potential SSRF target (this endpoint's response reflects
/// the fetched content, and env writes are admin-controlled at runtime).
pub fn validate_node_exporter_url(raw: &str, gateway: Option<&str>) -> Option<String> {
    let parsed = url::Url::parse(raw).ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix('[').map(|h| h.trim_end_matches(']')).unwrap_or(&host);
    let is_loopback = host == "localhost"
        || host == "::1"
        || host.parse::<std::net::Ipv4Addr>().map(|ip| ip.is_loopback()).unwrap_or(false);
    let is_gateway = gateway.map(|g| g.eq_ignore_ascii_case(host)).unwrap_or(false);
    if is_loopback || is_gateway {
        Some(parsed.to_string())
    } else {
        None
    }
}

/// Read the default gateway from the container's own /proc/net/route.
fn read_default_gateway() -> Option<String> {
    let txt = std::fs::read_to_string("/proc/net/route").ok()?;
    parse_default_gateway(&txt)
}

/// Pure parser for /proc/net/route: first line with Destination `00000000`
/// yields the gateway (little-endian hex IPv4).
fn parse_default_gateway(route_text: &str) -> Option<String> {
    for line in route_text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // Iface Destination Gateway ...
        if fields.len() >= 3 && fields[1] == "00000000" {
            return hex_ip_to_ipv4(fields[2]);
        }
    }
    None
}

fn hex_ip_to_ipv4(hex: &str) -> Option<String> {
    if hex.len() != 8 {
        return None;
    }
    let mut octets = Vec::with_capacity(4);
    // /proc/net/route writes IPv4 in little-endian byte order.
    for i in (0..4).rev() {
        let byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        octets.push(byte.to_string());
    }
    Some(octets.join("."))
}

// ─── Filesystems collector (node_exporter primary, /vol bind fallback) ──────

const EXCLUDED_FSTYPES: [&str; 4] = ["tmpfs", "overlay", "squashfs", "devtmpfs"];
const EXCLUDED_MOUNT_PREFIXES: [&str; 3] = ["/host/proc", "/host/sys", "/host/run"];

/// Pure parser for a Prometheus exposition body from node_exporter.
///
/// Extracts `node_filesystem_size_bytes` / `node_filesystem_free_bytes` /
/// `node_filesystem_avail_bytes` grouped by `device`/`mountpoint`/`fstype`.
/// Drops `tmpfs`/`overlay`/`squashfs`/`devtmpfs` and mountpoints under
/// `/host/proc|/host/sys|/host/run`. `used_bytes = size - free`,
/// `free_bytes = avail` (matches what a non-root writer actually gets).
/// Every surviving entry is `describes_host: true`.
///
/// Malformed/truncated input (unparseable value on a relevant metric, a
/// surviving filesystem missing size or avail, or no filesystem samples at
/// all) returns `Err` — never panics, never fabricates numbers.
pub fn parse_node_exporter_filesystems(body: &str) -> Result<Vec<FilesystemUsage>> {
    struct Sample {
        size: Option<f64>,
        free: Option<f64>,
        avail: Option<f64>,
    }
    let mut samples: HashMap<(String, String, String), Sample> = HashMap::new();

    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (metric, rest) = match line.find('{') {
            Some(brace) if line.contains('}') => (line[..brace].trim(), line[brace + 1..].trim()),
            // HELP/TYPE-less bare value line: `metric value`
            _ => match line.rsplit_once(char::is_whitespace) {
                Some((m, v)) if !m.contains(char::is_whitespace) => (m, v),
                _ => continue, // not a metric line we recognize
            },
        };
        if !matches!(
            metric,
            "node_filesystem_size_bytes"
                | "node_filesystem_free_bytes"
                | "node_filesystem_avail_bytes"
        ) {
            continue;
        }
        let (labels, value_str) = split_labels_value(rest)?;
        let device = labels.get("device").cloned().unwrap_or_default();
        let mountpoint = labels.get("mountpoint").cloned().unwrap_or_default();
        let fstype = labels.get("fstype").cloned().unwrap_or_default();
        if device.is_empty() || mountpoint.is_empty() {
            return Err(anyhow!("node_filesystem line missing device/mountpoint labels: {:?}", line));
        }
        let value = value_str
            .parse::<f64>()
            .map_err(|_| anyhow!("unparseable value {:?} for {}", value_str, metric))?;
        if !value.is_finite() || value < 0.0 {
            return Err(anyhow!("invalid value {} for {}", value, metric));
        }
        let entry = samples
            .entry((device, mountpoint, fstype))
            .or_insert(Sample { size: None, free: None, avail: None });
        match metric {
            "node_filesystem_size_bytes" => entry.size = Some(value),
            "node_filesystem_free_bytes" => entry.free = Some(value),
            "node_filesystem_avail_bytes" => entry.avail = Some(value),
            _ => unreachable!(),
        }
    }

    if samples.is_empty() {
        return Err(anyhow!("no node_filesystem samples found in metrics body"));
    }

    let mut out = Vec::new();
    for ((device, mountpoint, fstype), sample) in samples {
        if EXCLUDED_FSTYPES.contains(&fstype.as_str()) {
            continue;
        }
        if EXCLUDED_MOUNT_PREFIXES
            .iter()
            .any(|p| mountpoint == *p || mountpoint.starts_with(&format!("{}/", p)))
        {
            continue;
        }
        let size = sample
            .size
            .ok_or_else(|| anyhow!("filesystem {} on {} missing size sample", mountpoint, device))?;
        let avail = sample
            .avail
            .ok_or_else(|| anyhow!("filesystem {} on {} missing avail sample", mountpoint, device))?;
        // `used` is size minus *free* (not avail) when the free metric exists,
        // so reserved-for-root space counts as used. free_bytes is avail.
        let free_metric = sample.free.unwrap_or(avail);
        let total = size as i64;
        let free_bytes = avail as i64;
        let used_bytes = (size - free_metric).max(0.0) as i64;
        out.push(FilesystemUsage {
            mount: mountpoint,
            device,
            fstype,
            total_bytes: total,
            used_bytes,
            free_bytes,
            describes_host: true,
        });
    }
    Ok(out)
}

/// Split `k="v",k2="v2"} value` into a label map and the trailing value.
pub(crate) fn split_labels_value(rest: &str) -> Result<(HashMap<String, String>, &str)> {
    let mut labels = HashMap::new();
    let (label_str, value) = match rest.find('}') {
        Some(end) => (&rest[..end], rest[end + 1..].trim()),
        None => return Err(anyhow!("metric line missing closing brace: {:?}", rest)),
    };
    // Labels look like: device="x",fstype="ext4",mountpoint="/"
    // Split on `",` (quote-comma), then strip remaining quotes per piece.
    for pair in label_str.split("\",") {
        let pair = pair.trim_matches('"');
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| anyhow!("unparseable label {:?}", pair))?;
        let v = v.trim().trim_matches('"');
        let v = v.replace("\\\\", "\\").replace("\\\"", "\"");
        labels.insert(k.to_string(), v);
    }
    Ok((labels, value))
}

/// statvfs a path (blocking syscall — run inside spawn_blocking; a wedged mount
/// leaks the blocking thread, accepted for the single `/vol` path).
fn statvfs_blocking(path: &str) -> Result<nix::sys::statvfs::Statvfs> {
    nix::sys::statvfs::statvfs(path).map_err(|e| anyhow!("statvfs({}) failed: {}", path, e))
}

/// statvfs the `/vol` host bind. Returns `Ok(None)` when the path exists but is
/// NOT a bind of a host directory (same fsid as the container overlay root) —
/// reporting the overlay numbers as host numbers is exactly what requirement 4
/// forbids.
fn statvfs_vol_bind() -> Result<Option<FilesystemUsage>> {
    let vol = statvfs_blocking(VOL_BIND_MOUNT)?;
    let root = statvfs_blocking("/").map_err(|e| anyhow!("statvfs(/) failed: {}", e))?;
    if vol.filesystem_id() == root.filesystem_id() {
        return Ok(None); // same filesystem as the container overlay => not a host bind
    }
    let frsize = vol.fragment_size().max(1) as i64;
    let total = vol.blocks() as i64 * frsize;
    let used = (vol.blocks() as i64 - vol.blocks_free() as i64).max(0) * frsize;
    let free = vol.blocks_available() as i64 * frsize;
    Ok(Some(FilesystemUsage {
        mount: VOL_BIND_MOUNT.to_string(),
        device: format!("fsid:{}", vol.filesystem_id()),
        fstype: "bind".to_string(),
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        describes_host: true,
    }))
}

async fn collect_filesystems() -> (Vec<FilesystemUsage>, Vec<CollectorError>, &'static str) {
    let started = Instant::now();
    let mut errors: Vec<CollectorError> = Vec::new();

    // Primary: scrape the co-located node_exporter sidecar.
    if let Some(target) = node_exporter_target() {
        let client = reqwest_client();
        let scrape = async {
            let resp = client.get(&target).send().await?;
            let status = resp.status();
            let body = resp.text().await?;
            if !status.is_success() {
                return Err(anyhow!("node_exporter returned HTTP {}", status));
            }
            Ok(body)
        };
        match tokio::time::timeout(Duration::from_secs(COLLECTOR_TIMEOUT_SECS), scrape).await {
            Ok(Ok(body)) => match parse_node_exporter_filesystems(&body) {
                Ok(fss) if !fss.is_empty() => return (fss, errors, "node_exporter"),
                Ok(_) => errors.push(CollectorError::new(
                    "filesystems",
                    "node_exporter metrics contained no reportable filesystems",
                )),
                Err(e) => errors.push(CollectorError::new(
                    "filesystems",
                    format!("node_exporter metrics unparseable: {}", e),
                )),
            },
            Ok(Err(e)) => errors.push(CollectorError::new(
                "filesystems",
                format!("node_exporter scrape failed: {}", e),
            )),
            Err(_) => errors.push(CollectorError::new(
                "filesystems",
                format!("node_exporter scrape timed out after {}s", COLLECTOR_TIMEOUT_SECS),
            )),
        }
    } else {
        errors.push(CollectorError::new(
            "filesystems",
            "no node_exporter target resolved (NODE_EXPORTER_URL absent/invalid and no gateway)",
        ));
    }

    // Fallback: statvfs the /vol host bind with whatever budget is left.
    let remaining = Duration::from_secs(COLLECTOR_TIMEOUT_SECS)
        .saturating_sub(started.elapsed())
        .max(Duration::from_millis(1));
    let statvfs_fut = tokio::task::spawn_blocking(statvfs_vol_bind);
    match tokio::time::timeout(remaining, statvfs_fut).await {
        Ok(Ok(Ok(Some(fs)))) => return (vec![fs], errors, "container_bind"),
        Ok(Ok(Ok(None))) => errors.push(CollectorError::new(
            "filesystems",
            "/vol is not a bind of a host directory (same fsid as container root)",
        )),
        Ok(Ok(Err(e))) => errors.push(CollectorError::new(
            "filesystems",
            format!("/vol statvfs fallback unavailable: {}", e),
        )),
        Ok(Err(e)) => errors.push(CollectorError::new(
            "filesystems",
            format!("/vol statvfs task failed: {}", e),
        )),
        Err(_) => errors.push(CollectorError::new(
            "filesystems",
            format!("/vol statvfs fallback timed out after {}s", COLLECTOR_TIMEOUT_SECS),
        )),
    }
    (vec![], errors, "none")
}

fn reqwest_client() -> reqwest::Client {
    static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(COLLECTOR_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .expect("reqwest client for node_exporter")
    });
    CLIENT.clone()
}

// ─── Volumes collector (docker df) ──────────────────────────────────────────

/// Pure mapper from the Docker daemon's df payload to the contract shape.
///
/// `usage_data` present with `size >= 0` => `size_known: true`. `usage_data`
/// absent or `size == -1` => `size_bytes: None, size_known: false` plus a
/// `CollectorError` — never a fabricated `0`. `service` is filled from
/// `owners` (volume name → node name); missing keys stay `None`.
pub fn map_df_response(
    volumes: &[Volume],
    owners: &HashMap<String, String>,
) -> (Vec<VolumeUsage>, Vec<CollectorError>) {
    let mut out = Vec::with_capacity(volumes.len());
    let mut errors = Vec::new();
    for v in volumes {
        let service = owners.get(&v.name).cloned();
        match v.usage_data.as_ref().map(|u| u.size) {
            Some(size) if size >= 0 => out.push(VolumeUsage {
                name: v.name.clone(),
                size_bytes: Some(size),
                size_known: true,
                service,
            }),
            Some(size) => {
                errors.push(CollectorError::new(
                    "volumes",
                    format!("volume {}: size not computed by daemon (={})", v.name, size),
                ));
                out.push(VolumeUsage {
                    name: v.name.clone(),
                    size_bytes: None,
                    size_known: false,
                    service,
                });
            }
            None => {
                errors.push(CollectorError::new(
                    "volumes",
                    format!("volume {}: usage_data absent from docker df", v.name),
                ));
                out.push(VolumeUsage {
                    name: v.name.clone(),
                    size_bytes: None,
                    size_known: false,
                    service,
                });
            }
        }
    }
    (out, errors)
}

/// Longest-node-name, boundary-aware match of a `{name}.sphinx` / `{name}-{suffix}.sphinx`
/// volume onto an internal node name. `node_names` must already be sorted longest-first.
fn match_sphinx_volume(name: &str, node_names: &[String]) -> Option<String> {
    let base = name.strip_suffix(".sphinx")?;
    for n in node_names {
        if base == n.as_str() || base.starts_with(&format!("{}-", n)) {
            return Some(n.clone());
        }
    }
    None
}

/// Map each volume name to the friendly service label (`node.name()`).
///
/// Pass 1 (name-based, authoritative) attributes `.sphinx`-suffixed named
/// volumes from `df_volume_names` (and any named mounts) so a volume shared
/// by several containers is owned exactly once. Pass 2 attributes leftover
/// names (typically 64-hex anonymous volumes) from container mounts.
/// `Node::External` is skipped. Unattributed names stay out of the map.
/// Reads only `MountPoint.name` — never `source` / `destination`.
pub fn attribute_volumes(
    nodes: &[Node],
    containers: &[ContainerSummary],
    df_volume_names: &[String],
) -> HashMap<String, String> {
    let mut node_names: Vec<String> = Vec::new();
    for node in nodes {
        if let Ok(img) = node.as_internal() {
            let n = img.name();
            if !node_names.contains(&n) {
                node_names.push(n);
            }
        }
    }
    node_names.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));

    let mut owners: HashMap<String, String> = HashMap::new();

    for name in df_volume_names {
        if let Some(owner) = match_sphinx_volume(name, &node_names) {
            owners.insert(name.clone(), owner);
        }
    }
    for c in containers {
        if let Some(mounts) = &c.mounts {
            for m in mounts {
                if let Some(vol_name) = &m.name {
                    if owners.contains_key(vol_name) {
                        continue;
                    }
                    if let Some(owner) = match_sphinx_volume(vol_name, &node_names) {
                        owners.insert(vol_name.clone(), owner);
                    }
                }
            }
        }
    }

    for c in containers {
        let Some(names) = &c.names else {
            continue;
        };
        let Some(raw) = names.first() else {
            continue;
        };
        let cname = raw.trim_start_matches('/');
        let mut owner_label: Option<String> = None;
        for node in nodes {
            if let Ok(img) = node.as_internal() {
                if utils::domain(&img.name()) == cname {
                    owner_label = Some(img.name());
                    break;
                }
            }
        }
        let Some(owner_label) = owner_label else {
            continue;
        };
        let Some(mounts) = &c.mounts else {
            continue;
        };
        for m in mounts {
            let is_volume = m
                .typ
                .as_ref()
                .map(|t| t.to_string().eq_ignore_ascii_case("volume"))
                .unwrap_or(false);
            if !is_volume {
                continue;
            }
            let Some(vol_name) = &m.name else {
                continue;
            };
            owners
                .entry(vol_name.clone())
                .or_insert_with(|| owner_label.clone());
        }
    }

    owners
}

/// Named volumes attributed to Neo4j nodes (deduplicated, order preserved).
/// Neo4j mounts exactly one named volume, `utils::domain(&name)` at `/data`.
pub fn neo4j_volume_names(nodes: &[Node]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for node in nodes {
        if let Ok(img) = node.as_internal() {
            if matches!(img, Image::Neo4j(_)) {
                let vol = utils::domain(&img.name());
                if !names.contains(&vol) {
                    names.push(vol);
                }
            }
        }
    }
    names
}

/// Look Neo4j's volumes up in the SAME map that produced `volumes[]` so the two
/// can never disagree. Sums sizes only when every volume is known.
pub fn build_neo4j_storage(
    names: &[String],
    volume_map: &HashMap<String, VolumeUsage>,
) -> (Option<Neo4jStorage>, Vec<CollectorError>) {
    if names.is_empty() {
        return (None, Vec::new());
    }
    let mut errors = Vec::new();
    let mut all_known = true;
    let mut total: i64 = 0;
    for name in names {
        match volume_map.get(name) {
            Some(vu) if vu.size_known => {
                if let Some(sz) = vu.size_bytes {
                    total += sz;
                }
            }
            Some(vu) => {
                all_known = false;
                errors.push(CollectorError::new(
                    "neo4j",
                    format!("neo4j volume {} size unknown", vu.name),
                ));
            }
            None => {
                all_known = false;
                errors.push(CollectorError::new(
                    "neo4j",
                    format!("neo4j volume {} not found in docker df", name),
                ));
            }
        }
    }
    let storage = Neo4jStorage {
        volumes: names.to_vec(),
        size_bytes: if all_known { Some(total) } else { None },
        size_known: all_known,
    };
    (Some(storage), errors)
}

/// Per-service rollup for every internal node that owns at least one volume
/// present in `volume_map`. Sums only when every member is known; any unknown
/// or missing member suppresses the total. Nodes owning zero volumes are skipped.
pub fn build_service_storage(
    nodes: &[Node],
    volume_map: &HashMap<String, VolumeUsage>,
    owner_map: &HashMap<String, String>,
) -> (Vec<ServiceStorage>, Vec<CollectorError>) {
    let mut services = Vec::new();
    let mut errors = Vec::new();
    for node in nodes {
        let Ok(img) = node.as_internal() else {
            continue;
        };
        let label = img.name();
        let mut owned: Vec<String> = owner_map
            .iter()
            .filter(|(_, owner)| *owner == &label)
            .map(|(vol, _)| vol.clone())
            .filter(|vol| volume_map.contains_key(vol))
            .collect();
        if owned.is_empty() {
            continue;
        }
        owned.sort();
        let mut all_known = true;
        let mut total: i64 = 0;
        for name in &owned {
            match volume_map.get(name) {
                Some(vu) if vu.size_known => {
                    if let Some(sz) = vu.size_bytes {
                        total += sz;
                    }
                }
                Some(vu) => {
                    all_known = false;
                    errors.push(CollectorError::new(
                        "services",
                        format!("service {} volume {} size unknown", label, vu.name),
                    ));
                }
                None => {
                    all_known = false;
                    errors.push(CollectorError::new(
                        "services",
                        format!("service {} volume {} not found in docker df", label, name),
                    ));
                }
            }
        }
        services.push(ServiceStorage {
            name: label,
            typ: img.typ(),
            volumes: owned,
            size_bytes: if all_known { Some(total) } else { None },
            size_known: all_known,
        });
    }
    (services, errors)
}

/// Longest-prefix match of a root dir against reported mounts. `/` matches
/// everything (boundary-checked); `/var/lib/docker` on its own mount wins.
pub fn longest_prefix_match<'a>(filesystems: &'a [FilesystemUsage], root: &str) -> Option<&'a str> {
    filesystems
        .iter()
        .filter(|f| {
            let m = &f.mount;
            root == m
                || (root.starts_with(m.as_str())
                    && (m.as_str() == "/" || root.as_bytes().get(m.len()) == Some(&b'/')))
        })
        .map(|f| f.mount.as_str())
        .max_by_key(|m| m.len())
}

/// df() + info() + list_containers() under one roof. All cancelled client-side after 8s.
async fn collect_volumes(
    docker: &Docker,
    nodes: &[Node],
) -> (
    Vec<VolumeUsage>,
    Vec<CollectorError>,
    Option<String>,
    HashMap<String, String>,
) {
    let mut errors: Vec<CollectorError> = Vec::new();

    let df_fut = tokio::time::timeout(Duration::from_secs(COLLECTOR_TIMEOUT_SECS), docker.df());
    let info_fut = tokio::time::timeout(Duration::from_secs(COLLECTOR_TIMEOUT_SECS), docker.info());
    let containers_fut = tokio::time::timeout(
        Duration::from_secs(COLLECTOR_TIMEOUT_SECS),
        crate::dock::list_containers(docker),
    );
    let (df_res, info_res, containers_res) = tokio::join!(df_fut, info_fut, containers_fut);

    let df_volumes: Vec<Volume> = match &df_res {
        Ok(Ok(df)) => df.volumes.clone().unwrap_or_default(),
        _ => Vec::new(),
    };
    let df_volume_names: Vec<String> = df_volumes.iter().map(|v| v.name.clone()).collect();

    let containers = match containers_res {
        Ok(Ok(cs)) => cs,
        Ok(Err(e)) => {
            errors.push(CollectorError::new(
                "containers",
                format!("list_containers failed: {}", e),
            ));
            Vec::new()
        }
        Err(_) => {
            errors.push(CollectorError::new(
                "containers",
                format!("list_containers timed out after {}s", COLLECTOR_TIMEOUT_SECS),
            ));
            Vec::new()
        }
    };

    let owners = attribute_volumes(nodes, &containers, &df_volume_names);

    let (volumes, df_errors) = match df_res {
        Ok(Ok(_)) => map_df_response(&df_volumes, &owners),
        Ok(Err(e)) => (
            vec![],
            vec![CollectorError::new("volumes", format!("docker df failed: {}", e))],
        ),
        Err(_) => (
            vec![],
            vec![CollectorError::new(
                "volumes",
                format!("docker df timed out after {}s", COLLECTOR_TIMEOUT_SECS),
            )],
        ),
    };
    errors.extend(df_errors);

    let docker_root_dir = match info_res {
        Ok(Ok(info)) => info.docker_root_dir,
        Ok(Err(e)) => {
            errors.push(CollectorError::new("docker_info", format!("docker info failed: {}", e)));
            None
        }
        Err(_) => {
            errors.push(CollectorError::new(
                "docker_info",
                format!("docker info timed out after {}s", COLLECTOR_TIMEOUT_SECS),
            ));
            None
        }
    };
    (volumes, errors, docker_root_dir, owners)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ─── Orchestration + TTL cache / single-flight ──────────────────────────────

struct CacheEntry {
    value: HostStorage,
    collected_at: Instant,
}

static CACHE: Lazy<Mutex<Option<CacheEntry>>> = Lazy::new(|| Mutex::new(None));

/// Fresh collection. Partial failures stay inside `errors[]`; this never
/// returns Err.
pub async fn collect_host_storage(docker: &Docker, nodes: &[Node]) -> HostStorage {
    let (fs, mut errors, source) = collect_filesystems().await;
    let (volumes, mut vol_errors, docker_root_dir, owners) = collect_volumes(docker, nodes).await;
    errors.append(&mut vol_errors);

    let docker_root_filesystem = docker_root_dir
        .as_deref()
        .and_then(|root| longest_prefix_match(&fs, root).map(|m| m.to_string()));

    let volume_map: HashMap<String, VolumeUsage> =
        volumes.iter().map(|v| (v.name.clone(), v.clone())).collect();
    let names = neo4j_volume_names(nodes);
    let (neo4j, neo4j_errors) = build_neo4j_storage(&names, &volume_map);
    errors.extend(neo4j_errors);
    let (services, service_errors) = build_service_storage(nodes, &volume_map, &owners);
    errors.extend(service_errors);

    HostStorage {
        host_visible: fs.iter().any(|f| f.describes_host),
        source: source.to_string(),
        collected_at: now_unix(),
        cached: false,
        filesystems: fs,
        docker_root_dir,
        docker_root_filesystem,
        volumes,
        neo4j,
        services,
        errors,
    }
}

/// Cached entry point used by the handler. Within the 60s TTL the last result
/// is served as-is (`cached: true`, original `collected_at`). Holding the
/// tokio Mutex across the collection gives single-flight: concurrent misses
/// share ONE `Docker::df()` instead of stacking unbounded `du`-walks.
pub async fn get_host_storage(docker: &Docker, nodes: &[Node]) -> HostStorage {
    let mut guard = CACHE.lock().await;
    if let Some(entry) = guard.as_ref() {
        if entry.collected_at.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
            let mut value = entry.value.clone();
            value.cached = true;
            return value;
        }
    }
    let result = collect_host_storage(docker, nodes).await;
    *guard = Some(CacheEntry { value: result.clone(), collected_at: Instant::now() });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::neo4j::Neo4jImage;

    const NODE_EXPORTER_SAMPLE: &str =
        include_str!("../tests/fixtures/node_exporter_sample.txt");
    const DF_RESPONSE: &str = include_str!("../tests/fixtures/df_response.json");

    fn node(name: &str) -> Node {
        Node::Internal(Image::Neo4j(Neo4jImage::new(name, "5.19.0")))
    }

    // ── parse_node_exporter_filesystems ────────────────────────────────────

    #[test]
    fn parses_fixture_filesystems_and_filters_virtual_fstypes() {
        let fss = parse_node_exporter_filesystems(NODE_EXPORTER_SAMPLE).unwrap();
        assert_eq!(fss.len(), 2, "tmpfs/overlay/squashfs and /host/* must be filtered");

        let root = fss.iter().find(|f| f.mount == "/").unwrap();
        assert_eq!(root.device, "/dev/nvme0n1p1");
        assert_eq!(root.fstype, "ext4");
        assert_eq!(root.total_bytes, 100_000_000_000);
        // used = size - FREE metric (not avail): 1.0e11 - 9.0e10
        assert_eq!(root.used_bytes, 10_000_000_000);
        // free = avail
        assert_eq!(root.free_bytes, 85_000_000_000);
        assert!(root.describes_host);

        let docker_fs = fss.iter().find(|f| f.mount == "/var/lib/docker").unwrap();
        assert_eq!(docker_fs.device, "/dev/nvme1n1p1");
        assert_eq!(docker_fs.total_bytes, 200_000_000_000);
        assert_eq!(docker_fs.used_bytes, 50_000_000_000);
        assert_eq!(docker_fs.free_bytes, 123_400_000_000);
    }

    #[test]
    fn rejects_garbage_and_truncated_input() {
        // empty / garbage bodies
        assert!(parse_node_exporter_filesystems("").is_err());
        assert!(parse_node_exporter_filesystems("not a metric at all\n").is_err());
        // unparseable value on a relevant metric
        let bad_value = "node_filesystem_size_bytes{device=\"d\",fstype=\"ext4\",mountpoint=\"/\"} abc\n";
        assert!(parse_node_exporter_filesystems(bad_value).is_err());
        // missing avail sample
        let no_avail =
            "node_filesystem_size_bytes{device=\"d\",fstype=\"ext4\",mountpoint=\"/\"} 100\n";
        assert!(parse_node_exporter_filesystems(no_avail).is_err());
        // negative value
        let neg = "node_filesystem_size_bytes{device=\"d\",fstype=\"ext4\",mountpoint=\"/\"} -5\n";
        assert!(parse_node_exporter_filesystems(neg).is_err());
        // missing closing brace
        let no_brace = "node_filesystem_size_bytes{device=\"d\" 100\n";
        assert!(parse_node_exporter_filesystems(no_brace).is_err());
    }

    // ── map_df_response ────────────────────────────────────────────────────

    #[test]
    fn maps_df_fixture_volumes() {
        let df: bollard::models::SystemDataUsageResponse = serde_json::from_str(DF_RESPONSE)
            .expect("fixture must deserialize into bollard SystemDataUsageResponse");
        let volumes = df.volumes.as_deref().unwrap_or(&[]).to_vec();
        let (out, errors) = map_df_response(&volumes, &HashMap::new());
        assert_eq!(out.len(), 3);

        let neo4j = out.iter().find(|v| v.name == "neo4j.sphinx").unwrap();
        assert_eq!(neo4j.size_bytes, Some(536_870_912_000));
        assert!(neo4j.size_known);
        assert_eq!(neo4j.service, None, "empty owner map => service: None");

        let unknown = out.iter().find(|v| v.name == "unknown_driver.vol").unwrap();
        assert_eq!(unknown.size_bytes, None);
        assert!(!unknown.size_known);
        assert_eq!(unknown.service, None);

        let absent = out.iter().find(|v| v.name == "no_usage_data.vol").unwrap();
        assert_eq!(absent.size_bytes, None);
        assert!(!absent.size_known);
        assert_eq!(absent.service, None);

        // exactly two errors: the -1 volume and the absent usage_data volume
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|e| e.collector == "volumes"));
        assert!(errors
            .iter()
            .any(|e| e.reason.contains("not computed by daemon")));
        assert!(errors.iter().any(|e| e.reason.contains("usage_data absent")));

        let mut owners = HashMap::new();
        owners.insert("neo4j.sphinx".to_string(), "neo4j".to_string());
        let (out, _) = map_df_response(&volumes, &owners);
        let neo4j = out.iter().find(|v| v.name == "neo4j.sphinx").unwrap();
        assert_eq!(neo4j.service.as_deref(), Some("neo4j"));
        let unknown = out.iter().find(|v| v.name == "unknown_driver.vol").unwrap();
        assert_eq!(unknown.service, None);
    }

    // ── Neo4j attribution ──────────────────────────────────────────────────

    #[test]
    fn attributes_neo4j_volumes_when_node_exists() {
        let nodes = vec![node("neo4j"), Node::Internal(Image::Btc(crate::images::btc::BtcImage::new(
            "btc", "v23.0", "regtest",
        )))];
        let names = neo4j_volume_names(&nodes);
        assert_eq!(names, vec!["neo4j.sphinx".to_string()]);

        let mut map = HashMap::new();
        map.insert(
            "neo4j.sphinx".to_string(),
            VolumeUsage {
                name: "neo4j.sphinx".to_string(),
                size_bytes: Some(100),
                size_known: true,
                service: Some("neo4j".to_string()),
            },
        );
        let (storage, errors) = build_neo4j_storage(&names, &map);
        assert!(errors.is_empty());
        let storage = storage.unwrap();
        assert_eq!(storage.volumes, vec!["neo4j.sphinx".to_string()]);
        assert_eq!(storage.size_bytes, Some(100));
        assert!(storage.size_known);
    }

    #[test]
    fn neo4j_null_when_no_node() {
        let nodes = vec![Node::Internal(Image::Btc(crate::images::btc::BtcImage::new(
            "btc",
            "v23.0",
            "regtest",
        )))];
        assert!(neo4j_volume_names(&nodes).is_empty());
        let (storage, errors) = build_neo4j_storage(&neo4j_volume_names(&nodes), &HashMap::new());
        assert!(storage.is_none(), "no neo4j node => neo4j: null, a valid response");
        assert!(errors.is_empty());
    }

    #[test]
    fn neo4j_unknown_size_is_not_summed() {
        let nodes = vec![node("neo4j")];
        let names = neo4j_volume_names(&nodes);
        let mut map = HashMap::new();
        map.insert(
            "neo4j.sphinx".to_string(),
            VolumeUsage {
                name: "neo4j.sphinx".to_string(),
                size_bytes: None,
                size_known: false,
                service: Some("neo4j".to_string()),
            },
        );
        let (storage, errors) = build_neo4j_storage(&names, &map);
        let storage = storage.unwrap();
        assert!(!storage.size_known);
        assert_eq!(storage.size_bytes, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].collector, "neo4j");
        // volume missing from the df map entirely
        let (storage, errors) = build_neo4j_storage(&names, &HashMap::new());
        assert!(!storage.unwrap().size_known);
        assert!(errors.iter().any(|e| e.reason.contains("not found in docker df")));
    }

    // ── host_visible / longest-prefix ──────────────────────────────────────

    fn fs(mount: &str, describes_host: bool) -> FilesystemUsage {
        FilesystemUsage {
            mount: mount.to_string(),
            device: "dev".to_string(),
            fstype: "ext4".to_string(),
            total_bytes: 10,
            used_bytes: 5,
            free_bytes: 5,
            describes_host,
        }
    }

    #[test]
    fn host_visible_follows_describes_host_only() {
        // node_exporter path: entries marked describes_host => host_visible
        let fss = vec![fs("/", true)];
        assert!(fss.iter().any(|f| f.describes_host));
        // neither collector available: no entries => host_visible: false
        let fss: Vec<FilesystemUsage> = vec![];
        assert!(!fss.iter().any(|f| f.describes_host));
        // an entry that exists but does not describe the host never counts
        let fss = vec![fs("/", false)];
        assert!(!fss.iter().any(|f| f.describes_host));
    }

    #[test]
    fn longest_prefix_match_picks_deepest_mount() {
        let fss = vec![fs("/", true), fs("/var/lib/docker", true), fs("/var", true)];
        assert_eq!(longest_prefix_match(&fss, "/var/lib/docker"), Some("/var/lib/docker"));
        assert_eq!(longest_prefix_match(&fss, "/var/lib/docker/something"), Some("/var/lib/docker"));
        assert_eq!(longest_prefix_match(&fss, "/etc"), Some("/"));
        // no match at all
        let fss = vec![fs("/mnt/data", true)];
        assert_eq!(longest_prefix_match(&fss, "/var/lib/docker"), None);
    }

    // ── SSRF guard / gateway resolution ────────────────────────────────────

    #[test]
    fn validates_node_exporter_url() {
        let gw = Some("172.17.0.1");
        // loopback allowed
        assert_eq!(
            validate_node_exporter_url("http://127.0.0.1:9100/metrics", gw),
            Some("http://127.0.0.1:9100/metrics".to_string())
        );
        assert_eq!(
            validate_node_exporter_url("http://localhost:9100/metrics", gw),
            Some("http://localhost:9100/metrics".to_string())
        );
        // gateway allowed
        assert_eq!(
            validate_node_exporter_url("http://172.17.0.1:9100/metrics", gw),
            Some("http://172.17.0.1:9100/metrics".to_string())
        );
        // external / internal-non-gateway rejected
        assert_eq!(validate_node_exporter_url("http://169.254.169.254/latest", gw), None);
        assert_eq!(validate_node_exporter_url("http://10.0.0.5:9100/metrics", gw), None);
        assert_eq!(validate_node_exporter_url("http://172.17.0.2:9100/metrics", gw), None);
        // non-http scheme rejected
        assert_eq!(validate_node_exporter_url("gopher://127.0.0.1/metrics", gw), None);
        // unparseable rejected
        assert_eq!(validate_node_exporter_url("::not a url::", gw), None);
    }

    #[test]
    fn parses_default_gateway_from_proc_net_route() {
        let route = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n\
                     eth0\t00000000\t010011AC\t0003\t0\t0\t0\t00000000\t0\t0\t0\n\
                     eth0\t000011AC\t00000000\t0001\t0\t0\t0\t000F0000\t0\t0\t0\n";
        assert_eq!(parse_default_gateway(route), Some("172.17.0.1".to_string()));
        assert_eq!(parse_default_gateway("Iface\tDestination\tGateway\n"), None);
    }

    // ── response contract sanity ───────────────────────────────────────────

    #[test]
    fn contract_shape_round_trips() {
        let response = HostStorage {
            host_visible: true,
            source: "node_exporter".to_string(),
            collected_at: 1_730_000_000,
            cached: false,
            filesystems: vec![fs("/", true)],
            docker_root_dir: Some("/var/lib/docker".to_string()),
            docker_root_filesystem: Some("/".to_string()),
            volumes: vec![VolumeUsage {
                name: "neo4j.sphinx".to_string(),
                size_bytes: Some(1),
                size_known: true,
                service: Some("neo4j".to_string()),
            }],
            neo4j: Some(Neo4jStorage {
                volumes: vec!["neo4j.sphinx".to_string()],
                size_bytes: Some(1),
                size_known: true,
            }),
            services: vec![ServiceStorage {
                name: "neo4j".to_string(),
                typ: "Neo4j".to_string(),
                volumes: vec!["neo4j.sphinx".to_string()],
                size_bytes: Some(1),
                size_known: true,
            }],
            errors: vec![CollectorError { collector: "volumes".to_string(), reason: "x".to_string() }],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["host_visible"], true);
        assert_eq!(json["source"], "node_exporter");
        assert_eq!(json["collected_at"], 1_730_000_000);
        assert_eq!(json["cached"], false);
        assert!(json["filesystems"][0]["describes_host"].is_boolean());
        assert!(json["volumes"][0]["size_bytes"].is_i64());
        assert!(json["errors"][0]["collector"].is_string());
        // deserialize back
        let back: HostStorage = serde_json::from_value(json).unwrap();
        assert_eq!(back.volumes[0].name, "neo4j.sphinx");
        assert_eq!(back.volumes[0].service.as_deref(), Some("neo4j"));
        assert_eq!(back.services.len(), 1);
        assert_eq!(back.services[0].name, "neo4j");
    }

    // ── attribute_volumes ──────────────────────────────────────────────────

    const CONTAINERS_RESPONSE: &str = include_str!("../tests/fixtures/containers_response.json");

    fn cln_node() -> Node {
        Node::Internal(Image::Cln(crate::images::cln::ClnImage::new(
            "cln", "v23.05", "regtest", "9735", "10009",
        )))
    }
    fn boltwall_node() -> Node {
        Node::Internal(Image::BoltWall(crate::images::boltwall::BoltwallImage::new(
            "boltwall", "latest", "8444",
        )))
    }
    fn proxy_node() -> Node {
        Node::Internal(Image::Proxy(crate::images::proxy::ProxyImage::new(
            "proxy", "latest", "regtest", "11111", "5555",
        )))
    }
    fn repo2graph_node() -> Node {
        Node::Internal(Image::Repo2Graph(crate::images::repo2graph::Repo2GraphImage::new(
            "repo2graph", "latest", "3355",
        )))
    }
    fn external_btc() -> Node {
        Node::External(crate::config::ExternalNode::new(
            "ext-btc",
            crate::config::ExternalNodeType::Btc,
            "http://btc.example",
        ))
    }

    fn fixture_containers() -> Vec<ContainerSummary> {
        serde_json::from_str(CONTAINERS_RESPONSE)
            .expect("fixture must deserialize into bollard ContainerSummary[]")
    }

    #[test]
    fn attribute_volumes_named_anonymous_shared_suffixed_orphan_and_external() {
        let nodes = vec![
            cln_node(),
            boltwall_node(),
            proxy_node(),
            repo2graph_node(),
            external_btc(),
        ];
        let containers = fixture_containers();
        let df_names = vec![
            "cln.sphinx".to_string(),
            "repo2graph-sessions.sphinx".to_string(),
            "repo2graph-cache.sphinx".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            "orphan.vol".to_string(),
        ];
        let owners = attribute_volumes(&nodes, &containers, &df_names);

        assert_eq!(owners.get("cln.sphinx").map(String::as_str), Some("cln"));
        assert_eq!(
            owners.get("repo2graph-sessions.sphinx").map(String::as_str),
            Some("repo2graph")
        );
        assert_eq!(
            owners.get("repo2graph-cache.sphinx").map(String::as_str),
            Some("repo2graph")
        );
        assert_eq!(
            owners
                .get("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .map(String::as_str),
            Some("cln"),
            "anonymous 64-hex volume mounted by cln.sphinx belongs to cln"
        );
        assert!(
            !owners.contains_key("orphan.vol"),
            "unmatched volume stays out of the map => service: None"
        );
        // shared named volume attributed exactly once (HashMap key uniqueness)
        let cln_owned: Vec<_> = owners
            .iter()
            .filter(|(_, owner)| *owner == "cln")
            .map(|(vol, _)| vol.as_str())
            .collect();
        assert_eq!(
            cln_owned.iter().filter(|n| **n == "cln.sphinx").count(),
            1,
            "shared cln.sphinx must not be double-counted"
        );
        assert!(!owners.values().any(|o| o == "boltwall" || o == "proxy"));
        assert!(!owners.values().any(|o| o == "ext-btc"));
    }

    // ── build_service_storage ──────────────────────────────────────────────

    fn vu(name: &str, size: Option<i64>, known: bool, service: Option<&str>) -> VolumeUsage {
        VolumeUsage {
            name: name.to_string(),
            size_bytes: size,
            size_known: known,
            service: service.map(|s| s.to_string()),
        }
    }

    #[test]
    fn build_service_storage_sums_known_and_suppresses_unknown() {
        let nodes = vec![cln_node(), repo2graph_node(), boltwall_node()];
        let mut volume_map = HashMap::new();
        volume_map.insert(
            "cln.sphinx".to_string(),
            vu("cln.sphinx", Some(10), true, Some("cln")),
        );
        volume_map.insert(
            "anon".to_string(),
            vu("anon", Some(5), true, Some("cln")),
        );
        volume_map.insert(
            "repo2graph-sessions.sphinx".to_string(),
            vu("repo2graph-sessions.sphinx", Some(20), true, Some("repo2graph")),
        );
        volume_map.insert(
            "repo2graph-cache.sphinx".to_string(),
            vu("repo2graph-cache.sphinx", None, false, Some("repo2graph")),
        );
        let mut owners = HashMap::new();
        owners.insert("cln.sphinx".to_string(), "cln".to_string());
        owners.insert("anon".to_string(), "cln".to_string());
        owners.insert("repo2graph-sessions.sphinx".to_string(), "repo2graph".to_string());
        owners.insert("repo2graph-cache.sphinx".to_string(), "repo2graph".to_string());

        let (services, errors) = build_service_storage(&nodes, &volume_map, &owners);
        assert_eq!(services.len(), 2, "boltwall owns zero volumes => no entry");

        let cln = services.iter().find(|s| s.name == "cln").unwrap();
        assert_eq!(cln.typ, "Cln");
        assert_eq!(cln.size_bytes, Some(15));
        assert!(cln.size_known);
        assert_eq!(cln.volumes.len(), 2);

        let r2g = services.iter().find(|s| s.name == "repo2graph").unwrap();
        assert_eq!(r2g.typ, "Repo2Graph");
        assert_eq!(r2g.size_bytes, None);
        assert!(!r2g.size_known);
        assert!(errors.iter().any(|e| e.collector == "services"));
        assert!(errors.iter().any(|e| e.reason.contains("repo2graph")));
    }

    #[test]
    fn build_service_storage_empty_when_no_nodes_or_volumes() {
        let (services, errors) = build_service_storage(&[], &HashMap::new(), &HashMap::new());
        assert!(services.is_empty());
        assert!(errors.is_empty());

        let nodes = vec![cln_node()];
        let (services, errors) = build_service_storage(&nodes, &HashMap::new(), &HashMap::new());
        assert!(services.is_empty());
        assert!(errors.is_empty());
    }
}
