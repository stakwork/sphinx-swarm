//! FluentBit sidecar telemetry for the `GetFluentbitStats` swarm command.
//!
//! Scrapes the co-located FluentBit HTTP_Server over the `fluent-bit` bridge
//! (container DNS alias `fluentbit`, port 2020) and returns structured
//! counters. Modeled on the node_exporter scrape in `host_stats` but with a
//! hostname-string allowlist (not a private-range blocklist), no DNS/IP
//! re-check, no TTL cache, and a closed set of error reasons.
//!
//! Default target: `http://fluentbit:2020/api/v1/metrics`.
//!
//! Live Fluent Bit 3.2.10 serves **JSON** at `/api/v1/metrics` and Prometheus
//! text at `/api/v1/metrics/prometheus`. The allowlist pins the in-repo path
//! (`/api/v1/metrics`), so the collector accepts both encodings. Metric names
//! and `name=` label shapes were confirmed from a live `fluent/fluent-bit:3.2.10`
//! capture (see `tests/fixtures/fluentbit_prometheus_sample.txt`).

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};

use crate::host_stats::{split_labels_value, CollectorError};

/// Cheap in-network GET. Do not copy HostStorage's 8s/15s budgets.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

const MAX_BODY_BYTES: usize = 1024 * 1024; // 1 MiB

pub const DEFAULT_FLUENTBIT_METRICS_URL: &str = "http://fluentbit:2020/api/v1/metrics";

const COLLECTOR: &str = "fluentbit";
const REQUIRED_PORT: u16 = 2020;
const REQUIRED_PATH: &str = "/api/v1/metrics";

const INPUT_PREFIX: &str = "forward";
const OUTPUT_PREFIX: &str = "cloudwatch_logs";

const REASON_TIMEOUT: &str = "timeout";
const REASON_UNREACHABLE: &str = "unreachable";
const REASON_INVALID_TARGET: &str = "invalid-target";
const REASON_EMPTY_METRICS: &str = "empty-metrics";
const REASON_UNPARSEABLE: &str = "unparseable";
const REASON_REDIRECTED: &str = "redirected";
const REASON_TOO_LARGE: &str = "too-large";

const WARN_BAD_OVERRIDE: &str =
    "FLUENTBIT_METRICS_URL rejected; falling back to default FluentBit metrics target";

const ALLOWED_HOSTS: &[&str] = &[
    "fluentbit",
    "fluent_bit",
    "localhost",
    "127.0.0.1",
    "::1",
];

static FLUENTBIT_TARGET: OnceCell<String> = OnceCell::new();

/// Lifetime totals for one container, from the Fluent Bit Lua dump.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FluentbitContainerStats {
    pub container_name: String,
    pub input_bytes: i64,
    pub input_records: i64,
}

/// Hive-stable response for `SwarmCmd::GetFluentbitStats`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FluentbitStats {
    pub available: bool,
    /// Unix timestamp (seconds). Fresh on every call — no cache.
    pub collected_at: i64,
    pub input_bytes: Option<i64>,
    pub input_records: Option<i64>,
    pub output_proc_bytes: Option<i64>,
    pub output_proc_records: Option<i64>,
    /// Sum of every `fluentbit_filter_drop_records_total` series (lua + throttle).
    pub filter_drop_records: Option<i64>,
    pub output_dropped_records: Option<i64>,
    pub output_errors: Option<i64>,
    pub retries_failed: Option<i64>,
    pub uptime_seconds: Option<i64>,
    pub errors: Vec<CollectorError>,
    /// Per-container lifetime totals from the sidecar dump. Omitted when the
    /// dump is missing, empty, unparseable, or oversized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<FluentbitContainerStats>>,
}

#[derive(Default, Debug, Clone)]
struct ParsedCounters {
    input_bytes: Option<i64>,
    input_records: Option<i64>,
    output_proc_bytes: Option<i64>,
    output_proc_records: Option<i64>,
    filter_drop_records: Option<i64>,
    output_dropped_records: Option<i64>,
    output_errors: Option<i64>,
    retries_failed: Option<i64>,
    uptime_seconds: Option<i64>,
}

impl ParsedCounters {
    fn has_any(&self) -> bool {
        self.input_bytes.is_some()
            || self.input_records.is_some()
            || self.output_proc_bytes.is_some()
            || self.output_proc_records.is_some()
            || self.filter_drop_records.is_some()
            || self.output_dropped_records.is_some()
            || self.output_errors.is_some()
            || self.retries_failed.is_some()
            || self.uptime_seconds.is_some()
    }

    fn into_stats(self, collected_at: i64, errors: Vec<CollectorError>) -> FluentbitStats {
        let available = self.has_any() && errors.is_empty();
        FluentbitStats {
            available,
            collected_at,
            input_bytes: self.input_bytes,
            input_records: self.input_records,
            output_proc_bytes: self.output_proc_bytes,
            output_proc_records: self.output_proc_records,
            filter_drop_records: self.filter_drop_records,
            output_dropped_records: self.output_dropped_records,
            output_errors: self.output_errors,
            retries_failed: self.retries_failed,
            uptime_seconds: self.uptime_seconds,
            errors,
            containers: None,
        }
    }
}

// ─── Target resolution (once, hostname-string allowlist) ─────────────────────

/// Pin the scrape target for the process lifetime. Called after dotenv in the
/// stack binary so a `.env` override is visible. Runtime `set_var` cannot
/// repoint the collector afterwards.
pub fn init_fluentbit_target() {
    let _ = FLUENTBIT_TARGET.get_or_init(resolve_fluentbit_target);
}

/// The pinned scrape target. Always a URL string — a bad override falls back
/// to the default rather than disabling the collector.
pub fn fluentbit_target() -> String {
    FLUENTBIT_TARGET
        .get_or_init(resolve_fluentbit_target)
        .clone()
}

fn resolve_fluentbit_target() -> String {
    resolve_fluentbit_target_from(std::env::var("FLUENTBIT_METRICS_URL").ok().as_deref())
}

/// Pure resolver: invalid/empty override → default. Never returns empty.
pub(crate) fn resolve_fluentbit_target_from(raw: Option<&str>) -> String {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(url) => match validate_fluentbit_url(url) {
            Some(ok) => ok,
            None => {
                log::warn!("{}", WARN_BAD_OVERRIDE);
                DEFAULT_FLUENTBIT_METRICS_URL.to_string()
            }
        },
        None => DEFAULT_FLUENTBIT_METRICS_URL.to_string(),
    }
}

/// Strict hostname-string allowlist. Does **not** resolve DNS and does **not**
/// pin a `SocketAddr` (a sidecar recreate must keep working).
pub fn validate_fluentbit_url(raw: &str) -> Option<String> {
    let parsed = url::Url::parse(raw).ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    let host = host
        .strip_prefix('[')
        .map(|h| h.trim_end_matches(']'))
        .unwrap_or(&host);
    if is_imds_host(host) {
        return None;
    }
    if !ALLOWED_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(host)) {
        return None;
    }
    if parsed.port() != Some(REQUIRED_PORT) {
        return None;
    }
    if parsed.path() != REQUIRED_PATH {
        return None;
    }
    Some(parsed.to_string())
}

fn is_imds_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    if h == "169.254.169.254" || h == "fd00:ec2::254" || h == "::ffff:169.254.169.254" {
        return true;
    }
    match h.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(v4)) => v4.octets() == [169, 254, 169, 254],
        Ok(std::net::IpAddr::V6(v6)) => {
            v6.octets() == [0xfd, 0x00, 0x0e, 0xc2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02, 0x54]
                || v6
                    .to_ipv4_mapped()
                    .map(|v4| v4.octets() == [169, 254, 169, 254])
                    .unwrap_or(false)
        }
        Err(_) => false,
    }
}

// ─── HTTP scrape ─────────────────────────────────────────────────────────────

fn reqwest_client() -> reqwest::Client {
    static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
        reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest client for fluentbit")
    });
    CLIENT.clone()
}

fn map_reqwest_reason(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        REASON_TIMEOUT
    } else if err.is_redirect() {
        REASON_REDIRECTED
    } else if err.is_builder() {
        REASON_INVALID_TARGET
    } else {
        REASON_UNREACHABLE
    }
}

async fn read_body_capped(mut resp: reqwest::Response) -> Result<String, &'static str> {
    if let Some(len) = resp.content_length() {
        if len > MAX_BODY_BYTES as u64 {
            return Err(REASON_TOO_LARGE);
        }
    }
    let mut buf = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                if buf.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
                    return Err(REASON_TOO_LARGE);
                }
                buf.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => return Err(map_reqwest_reason(&e)),
        }
    }
    String::from_utf8(buf).map_err(|_| REASON_UNPARSEABLE)
}

/// Scrape `url` (already allowlisted in production). Closed reason set only.
pub(crate) async fn scrape_fluentbit_url(url: &str) -> Result<String, &'static str> {
    let client = reqwest_client();
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => return Err(map_reqwest_reason(&e)),
    };
    let status = resp.status();
    if status.is_redirection() {
        return Err(REASON_REDIRECTED);
    }
    if !status.is_success() {
        return Err(REASON_UNREACHABLE);
    }
    read_body_capped(resp).await
}

// ─── Parsers ─────────────────────────────────────────────────────────────────

fn plugin_name_matches(name: &str, prefix: &str) -> bool {
    name == prefix
        || name
            .strip_prefix(prefix)
            .map(|rest| rest.starts_with('.'))
            .unwrap_or(false)
}

fn add_counter(slot: &mut Option<i64>, value: i64) {
    *slot = Some(slot.unwrap_or(0).saturating_add(value));
}

fn parse_metric_number(value_str: &str) -> Result<i64, &'static str> {
    let token = value_str.split_whitespace().next().unwrap_or("");
    let n: f64 = token.parse().map_err(|_| REASON_UNPARSEABLE)?;
    if !n.is_finite() || n < 0.0 {
        return Err(REASON_UNPARSEABLE);
    }
    if n > i64::MAX as f64 {
        return Err(REASON_UNPARSEABLE);
    }
    Ok(n as i64)
}

/// Never panics on malformed/truncated input.
pub(crate) fn parse_fluentbit_metrics(body: &str) -> Result<ParsedCounters, &'static str> {
    let trimmed = body.trim_start();
    if trimmed.starts_with('{') {
        parse_json_metrics(trimmed)
    } else {
        parse_prometheus_metrics(body)
    }
}

fn parse_prometheus_metrics(body: &str) -> Result<ParsedCounters, &'static str> {
    let mut out = ParsedCounters::default();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (metric, rest, had_labels) = match line.find('{') {
            Some(brace) => (line[..brace].trim(), line[brace + 1..].trim(), true),
            None => match line.rsplit_once(char::is_whitespace) {
                Some((m, v)) if !m.contains(char::is_whitespace) => (m, v, false),
                _ => continue,
            },
        };

        match metric {
            "fluentbit_input_bytes_total"
            | "fluentbit_input_records_total"
            | "fluentbit_output_proc_bytes_total"
            | "fluentbit_output_proc_records_total"
            | "fluentbit_filter_drop_records_total"
            | "fluentbit_output_dropped_records_total"
            | "fluentbit_output_errors_total"
            | "fluentbit_output_retries_failed_total"
            | "fluentbit_uptime" => {}
            _ => continue,
        }

        let (name, value_str) = if had_labels {
            let (labels, value_str) =
                split_labels_value(rest).map_err(|_| REASON_UNPARSEABLE)?;
            (labels.get("name").cloned().unwrap_or_default(), value_str)
        } else {
            (String::new(), rest)
        };
        let value = parse_metric_number(value_str)?;

        match metric {
            "fluentbit_input_bytes_total" if plugin_name_matches(&name, INPUT_PREFIX) => {
                add_counter(&mut out.input_bytes, value);
            }
            "fluentbit_input_records_total" if plugin_name_matches(&name, INPUT_PREFIX) => {
                add_counter(&mut out.input_records, value);
            }
            "fluentbit_output_proc_bytes_total"
                if plugin_name_matches(&name, OUTPUT_PREFIX) =>
            {
                add_counter(&mut out.output_proc_bytes, value);
            }
            "fluentbit_output_proc_records_total"
                if plugin_name_matches(&name, OUTPUT_PREFIX) =>
            {
                add_counter(&mut out.output_proc_records, value);
            }
            "fluentbit_filter_drop_records_total" => {
                add_counter(&mut out.filter_drop_records, value);
            }
            "fluentbit_output_dropped_records_total"
                if plugin_name_matches(&name, OUTPUT_PREFIX) =>
            {
                add_counter(&mut out.output_dropped_records, value);
            }
            "fluentbit_output_errors_total" if plugin_name_matches(&name, OUTPUT_PREFIX) => {
                add_counter(&mut out.output_errors, value);
            }
            "fluentbit_output_retries_failed_total"
                if plugin_name_matches(&name, OUTPUT_PREFIX) =>
            {
                add_counter(&mut out.retries_failed, value);
            }
            "fluentbit_uptime" => {
                out.uptime_seconds = Some(value);
            }
            _ => {}
        }
    }
    Ok(out)
}

fn json_i64(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= 0 {
                    return Some(i);
                }
            }
            n.as_f64()
                .filter(|f| f.is_finite() && *f >= 0.0 && *f <= i64::MAX as f64)
                .map(|f| f as i64)
        }
        _ => None,
    }
}

fn parse_json_metrics(body: &str) -> Result<ParsedCounters, &'static str> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|_| REASON_UNPARSEABLE)?;
    let obj = match v.as_object() {
        Some(o) => o,
        None => return Ok(ParsedCounters::default()),
    };
    let mut out = ParsedCounters::default();

    if let Some(input) = obj.get("input").and_then(|x| x.as_object()) {
        for (name, plugin) in input {
            if !plugin_name_matches(name, INPUT_PREFIX) {
                continue;
            }
            if let Some(n) = plugin.get("bytes").and_then(json_i64) {
                add_counter(&mut out.input_bytes, n);
            }
            if let Some(n) = plugin.get("records").and_then(json_i64) {
                add_counter(&mut out.input_records, n);
            }
        }
    }
    if let Some(filter) = obj.get("filter").and_then(|x| x.as_object()) {
        for (_name, plugin) in filter {
            if let Some(n) = plugin.get("drop_records").and_then(json_i64) {
                add_counter(&mut out.filter_drop_records, n);
            }
        }
    }
    if let Some(output) = obj.get("output").and_then(|x| x.as_object()) {
        for (name, plugin) in output {
            if !plugin_name_matches(name, OUTPUT_PREFIX) {
                continue;
            }
            if let Some(n) = plugin.get("proc_bytes").and_then(json_i64) {
                add_counter(&mut out.output_proc_bytes, n);
            }
            if let Some(n) = plugin.get("proc_records").and_then(json_i64) {
                add_counter(&mut out.output_proc_records, n);
            }
            if let Some(n) = plugin.get("dropped_records").and_then(json_i64) {
                add_counter(&mut out.output_dropped_records, n);
            }
            if let Some(n) = plugin.get("errors").and_then(json_i64) {
                add_counter(&mut out.output_errors, n);
            }
            if let Some(n) = plugin.get("retries_failed").and_then(json_i64) {
                add_counter(&mut out.retries_failed, n);
            }
        }
    }
    Ok(out)
}

const MAX_CONTAINER_STATS: usize = 64;

/// Parse the Lua sidecar dump `{"containers":[...]}`. Never panics.
///
/// Returns `None` on any parse failure, a missing/empty `containers` array,
/// or when every entry is dropped. Never returns `Some(vec![])`.
pub(crate) fn parse_container_stats_dump(bytes: &[u8]) -> Option<Vec<FluentbitContainerStats>> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let obj = v.as_object()?;
    let arr = obj.get("containers")?.as_array()?;
    if arr.is_empty() {
        return None;
    }

    let mut out = Vec::new();
    for entry in arr {
        let Some(eobj) = entry.as_object() else {
            continue;
        };
        let name = match eobj.get("container_name") {
            Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
            _ => continue,
        };
        out.push(FluentbitContainerStats {
            container_name: name,
            input_bytes: eobj.get("input_bytes").and_then(json_i64).unwrap_or(0),
            input_records: eobj.get("input_records").and_then(json_i64).unwrap_or(0),
        });
    }
    if out.is_empty() {
        return None;
    }

    out.sort_by(|a, b| {
        b.input_bytes
            .cmp(&a.input_bytes)
            .then_with(|| b.input_records.cmp(&a.input_records))
            .then_with(|| a.container_name.cmp(&b.container_name))
    });
    out.truncate(MAX_CONTAINER_STATS);
    Some(out)
}

/// ~64 KiB cap on a downloaded dump before parsing.
pub(crate) const CONTAINER_STATS_DUMP_MAX_BYTES: usize = 64 * 1024;

/// Merge a downloaded dump into `stats.containers`. Never touches `available`
/// or `errors`. Returns a closed warn reason when a dump was present but
/// unusable; `None` on success or a normal miss (empty/missing containers).
pub(crate) fn merge_container_stats_dump(
    stats: &mut FluentbitStats,
    bytes: &[u8],
) -> Option<&'static str> {
    stats.containers = None;
    if bytes.len() > CONTAINER_STATS_DUMP_MAX_BYTES {
        return Some(REASON_TOO_LARGE);
    }
    match parse_container_stats_dump(bytes) {
        Some(parsed) => {
            stats.containers = Some(parsed);
            None
        }
        None => {
            if serde_json::from_slice::<serde_json::Value>(bytes).is_err() {
                Some(REASON_UNPARSEABLE)
            } else {
                None
            }
        }
    }
}

fn fluentbit_error(reason: &'static str) -> CollectorError {
    CollectorError {
        collector: COLLECTOR.to_string(),
        reason: reason.to_string(),
    }
}

fn unavailable(collected_at: i64, reason: &'static str) -> FluentbitStats {
    ParsedCounters::default().into_stats(collected_at, vec![fluentbit_error(reason)])
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn stats_from_body(body: &str, collected_at: i64) -> FluentbitStats {
    match parse_fluentbit_metrics(body) {
        Ok(parsed) if parsed.has_any() => parsed.into_stats(collected_at, vec![]),
        Ok(_) => unavailable(collected_at, REASON_EMPTY_METRICS),
        Err(reason) => unavailable(collected_at, reason),
    }
}

pub(crate) fn containers_log_label(stats: &FluentbitStats) -> String {
    match &stats.containers {
        Some(cs) => cs.len().to_string(),
        None => "none".to_string(),
    }
}

fn log_stats(stats: &FluentbitStats, elapsed_ms: u128) {
    log::info!(
        "GetFluentbitStats available={} input_bytes={} input_records={} output_proc_bytes={} output_proc_records={} filter_drop_records={} output_dropped_records={} output_errors={} retries_failed={} uptime_seconds={} containers={} errors={} elapsed_ms={}",
        stats.available,
        stats.input_bytes.is_some(),
        stats.input_records.is_some(),
        stats.output_proc_bytes.is_some(),
        stats.output_proc_records.is_some(),
        stats.filter_drop_records.is_some(),
        stats.output_dropped_records.is_some(),
        stats.output_errors.is_some(),
        stats.retries_failed.is_some(),
        stats.uptime_seconds.is_some(),
        containers_log_label(stats),
        stats.errors.len(),
        elapsed_ms
    );
}

/// Fresh scrape on every call. Never panics; failures land in `errors[]`.
pub async fn get_fluentbit_stats() -> FluentbitStats {
    collect_from_url(&fluentbit_target()).await
}

pub(crate) async fn collect_from_url(url: &str) -> FluentbitStats {
    let started = Instant::now();
    let collected_at = now_unix();
    let stats = match scrape_fluentbit_url(url).await {
        Ok(body) => stats_from_body(&body, collected_at),
        Err(reason) => unavailable(collected_at, reason),
    };
    log_stats(&stats, started.elapsed().as_millis());
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const SAMPLE: &str = include_str!("../tests/fixtures/fluentbit_prometheus_sample.txt");
    const PARTIAL: &str = include_str!("../tests/fixtures/fluentbit_prometheus_partial.txt");
    const MALFORMED: &str = include_str!("../tests/fixtures/fluentbit_prometheus_malformed.txt");
    const EMPTY_EXPECTED: &str =
        include_str!("../tests/fixtures/fluentbit_prometheus_empty_expected.txt");

    fn ok_url(host: &str) -> String {
        format!("http://{host}:2020/api/v1/metrics")
    }

    // ── fixtures ──────────────────────────────────────────────────────────

    #[test]
    fn full_fixture_populates_all_counters() {
        let stats = stats_from_body(SAMPLE, 1_700_000_000);
        assert!(stats.available);
        assert!(stats.errors.is_empty());
        assert_eq!(stats.input_bytes, Some(0));
        assert_eq!(stats.input_records, Some(0));
        assert_eq!(stats.output_proc_bytes, Some(0));
        assert_eq!(stats.output_proc_records, Some(0));
        // lua.0 + throttle.1 both present at 0 — sum, not missing.
        assert_eq!(stats.filter_drop_records, Some(0));
        assert_eq!(stats.output_dropped_records, Some(0));
        assert_eq!(stats.output_errors, Some(0));
        assert_eq!(stats.retries_failed, Some(0));
        assert_eq!(stats.uptime_seconds, Some(258));
        assert_eq!(stats.collected_at, 1_700_000_000);
        assert!(stats.containers.is_none());
    }

    #[test]
    fn partial_fixture_missing_counters_are_none() {
        let stats = stats_from_body(PARTIAL, 1);
        assert!(stats.available, "present counters must keep available true");
        assert!(stats.errors.is_empty());
        assert_eq!(stats.input_bytes, Some(0));
        assert_eq!(stats.input_records, Some(0));
        assert_eq!(stats.output_proc_bytes, Some(0));
        assert_eq!(stats.output_proc_records, Some(0));
        assert_eq!(stats.output_dropped_records, Some(0));
        assert_eq!(stats.retries_failed, Some(0));
        assert_eq!(stats.filter_drop_records, None);
        assert_eq!(stats.output_errors, None);
        assert_eq!(stats.uptime_seconds, None);
        assert!(stats.containers.is_none());
    }

    #[test]
    fn malformed_fixture_is_unparseable_and_does_not_panic() {
        let stats = stats_from_body(MALFORMED, 1);
        assert!(!stats.available);
        assert_eq!(stats.errors.len(), 1);
        assert_eq!(stats.errors[0].collector, "fluentbit");
        assert_eq!(stats.errors[0].reason, REASON_UNPARSEABLE);
        assert!(stats.input_bytes.is_none());
        assert!(stats.filter_drop_records.is_none());
        assert!(stats.containers.is_none());
    }

    #[test]
    fn empty_expected_fixture_is_unavailable() {
        let stats = stats_from_body(EMPTY_EXPECTED, 1);
        assert!(!stats.available);
        assert_eq!(stats.errors.len(), 1);
        assert_eq!(stats.errors[0].reason, REASON_EMPTY_METRICS);
        assert!(stats.input_bytes.is_none());
        assert!(stats.uptime_seconds.is_none());
        assert!(stats.containers.is_none());
    }

    #[test]
    fn filter_drop_series_are_summed() {
        let body = r#"
fluentbit_filter_drop_records_total{name="lua.0"} 3
fluentbit_filter_drop_records_total{name="throttle.1"} 5
fluentbit_uptime 1
"#;
        let stats = stats_from_body(body, 1);
        assert!(stats.available);
        assert_eq!(stats.filter_drop_records, Some(8));
    }

    #[test]
    fn missing_counter_is_none_not_zero() {
        let body = "fluentbit_uptime 12\n";
        let stats = stats_from_body(body, 1);
        assert!(stats.available);
        assert_eq!(stats.uptime_seconds, Some(12));
        assert_eq!(stats.input_bytes, None);
        assert_eq!(stats.filter_drop_records, None);
        assert_eq!(stats.output_errors, None);
    }

    #[test]
    fn storage_backlog_is_not_counted_as_forward_input() {
        let body = r#"
fluentbit_input_bytes_total{name="storage_backlog.1"} 99
fluentbit_input_records_total{name="storage_backlog.1"} 7
fluentbit_uptime 1
"#;
        let stats = stats_from_body(body, 1);
        assert_eq!(stats.input_bytes, None);
        assert_eq!(stats.input_records, None);
        assert_eq!(stats.uptime_seconds, Some(1));
    }

    #[test]
    fn live_json_metrics_body_parses() {
        // Captured from GET /api/v1/metrics on fluent/fluent-bit:3.2.10.
        let body = r#"{"input":{"forward.0":{"records":0,"bytes":0},"storage_backlog.1":{"records":0,"bytes":0}},"filter":{"lua.0":{"drop_records":0,"add_records":0,"records":0,"bytes":0,"drop_bytes":0},"throttle.1":{"drop_records":0,"add_records":0,"records":0,"bytes":0,"drop_bytes":0}},"output":{"cloudwatch_logs.0":{"proc_records":0,"proc_bytes":0,"errors":0,"retries":0,"retries_failed":0,"dropped_records":0,"retried_records":0}}}"#;
        let stats = stats_from_body(body, 1);
        assert!(stats.available);
        assert_eq!(stats.input_bytes, Some(0));
        assert_eq!(stats.input_records, Some(0));
        assert_eq!(stats.output_proc_bytes, Some(0));
        assert_eq!(stats.output_proc_records, Some(0));
        assert_eq!(stats.filter_drop_records, Some(0));
        assert_eq!(stats.output_dropped_records, Some(0));
        assert_eq!(stats.output_errors, Some(0));
        assert_eq!(stats.retries_failed, Some(0));
        assert_eq!(stats.uptime_seconds, None);
    }

    #[test]
    fn json_filter_drops_sum_lua_and_throttle() {
        let body = r#"{"filter":{"lua.0":{"drop_records":4},"throttle.1":{"drop_records":6}},"input":{"forward.0":{"bytes":1,"records":2}}}"#;
        let stats = stats_from_body(body, 1);
        assert_eq!(stats.filter_drop_records, Some(10));
        assert_eq!(stats.input_bytes, Some(1));
        assert_eq!(stats.input_records, Some(2));
    }

    #[test]
    fn closed_reason_set_never_interpolates() {
        for reason in [
            REASON_TIMEOUT,
            REASON_UNREACHABLE,
            REASON_INVALID_TARGET,
            REASON_EMPTY_METRICS,
            REASON_UNPARSEABLE,
            REASON_REDIRECTED,
            REASON_TOO_LARGE,
        ] {
            let stats = unavailable(1, reason);
            assert_eq!(stats.errors[0].reason, reason);
            assert!(!stats.errors[0].reason.contains("http"));
            assert!(!stats.errors[0].reason.contains('\n'));
        }
    }

    #[test]
    fn contract_shape_round_trips() {
        let stats = FluentbitStats {
            available: true,
            collected_at: 1_730_000_000,
            input_bytes: Some(1),
            input_records: None,
            output_proc_bytes: Some(2),
            output_proc_records: Some(3),
            filter_drop_records: Some(4),
            output_dropped_records: None,
            output_errors: Some(0),
            retries_failed: None,
            uptime_seconds: Some(9),
            errors: vec![],
            containers: None,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["available"], true);
        assert_eq!(json["collected_at"], 1_730_000_000);
        assert!(json["input_records"].is_null());
        assert_eq!(json["output_errors"], 0);
        assert!(
            json.get("containers").is_none(),
            "containers: None must be omitted from serialized JSON"
        );
        let back: FluentbitStats = serde_json::from_value(json).unwrap();
        assert_eq!(back.available, stats.available);
        assert_eq!(back.collected_at, stats.collected_at);
        assert_eq!(back.input_bytes, stats.input_bytes);
        assert!(back.input_records.is_none());
        assert_eq!(back.uptime_seconds, Some(9));
        assert!(back.containers.is_none());

        let populated = FluentbitStats {
            available: true,
            collected_at: 1_730_000_000,
            input_bytes: Some(1),
            input_records: None,
            output_proc_bytes: Some(2),
            output_proc_records: Some(3),
            filter_drop_records: Some(4),
            output_dropped_records: None,
            output_errors: Some(0),
            retries_failed: None,
            uptime_seconds: Some(9),
            errors: vec![],
            containers: Some(vec![FluentbitContainerStats {
                container_name: "hive".to_string(),
                input_bytes: 10,
                input_records: 2,
            }]),
        };
        let json = serde_json::to_value(&populated).unwrap();
        assert!(json.get("containers").is_some());
        assert_eq!(json["containers"][0]["container_name"], "hive");
        assert_eq!(json["containers"][0]["input_bytes"], 10);
        assert_eq!(json["containers"][0]["input_records"], 2);
        let back: FluentbitStats = serde_json::from_value(json).unwrap();
        assert_eq!(back.containers, populated.containers);
    }

    // ── container stats dump ──────────────────────────────────────────────

    #[test]
    fn parse_container_stats_dump_table() {
        struct Case {
            name: &'static str,
            body: &'static [u8],
            expect: Option<Vec<FluentbitContainerStats>>,
        }
        let cases = [
            Case {
                name: "valid dump with multiple containers",
                body: br#"{"containers":[
                    {"container_name":"boltwall","input_bytes":100,"input_records":5},
                    {"container_name":"hive","input_bytes":200,"input_records":9}
                ]}"#,
                expect: Some(vec![
                    FluentbitContainerStats {
                        container_name: "hive".to_string(),
                        input_bytes: 200,
                        input_records: 9,
                    },
                    FluentbitContainerStats {
                        container_name: "boltwall".to_string(),
                        input_bytes: 100,
                        input_records: 5,
                    },
                ]),
            },
            Case {
                name: "empty containers array",
                body: br#"{"containers":[]}"#,
                expect: None,
            },
            Case {
                name: "missing containers key",
                body: br#"{"other":1}"#,
                expect: None,
            },
            Case {
                name: "empty top-level object",
                body: b"{}",
                expect: None,
            },
            Case {
                name: "malformed json",
                body: b"{not json",
                expect: None,
            },
            Case {
                name: "truncated json",
                body: br#"{"containers":[{"container_name":"hive""#,
                expect: None,
            },
            Case {
                name: "empty container_name dropped",
                body: br#"{"containers":[
                    {"container_name":"","input_bytes":9,"input_records":1},
                    {"container_name":"hive","input_bytes":1,"input_records":1}
                ]}"#,
                expect: Some(vec![FluentbitContainerStats {
                    container_name: "hive".to_string(),
                    input_bytes: 1,
                    input_records: 1,
                }]),
            },
            Case {
                name: "missing container_name dropped",
                body: br#"{"containers":[
                    {"input_bytes":9,"input_records":1},
                    {"container_name":"ok","input_bytes":2,"input_records":3}
                ]}"#,
                expect: Some(vec![FluentbitContainerStats {
                    container_name: "ok".to_string(),
                    input_bytes: 2,
                    input_records: 3,
                }]),
            },
            Case {
                name: "all names empty => None not empty vec",
                body: br#"{"containers":[{"container_name":"","input_bytes":1,"input_records":1}]}"#,
                expect: None,
            },
            Case {
                name: "sort by bytes desc, then records desc, then name asc",
                body: br#"{"containers":[
                    {"container_name":"b","input_bytes":10,"input_records":1},
                    {"container_name":"a","input_bytes":10,"input_records":1},
                    {"container_name":"c","input_bytes":10,"input_records":5},
                    {"container_name":"z","input_bytes":50,"input_records":0}
                ]}"#,
                expect: Some(vec![
                    FluentbitContainerStats {
                        container_name: "z".to_string(),
                        input_bytes: 50,
                        input_records: 0,
                    },
                    FluentbitContainerStats {
                        container_name: "c".to_string(),
                        input_bytes: 10,
                        input_records: 5,
                    },
                    FluentbitContainerStats {
                        container_name: "a".to_string(),
                        input_bytes: 10,
                        input_records: 1,
                    },
                    FluentbitContainerStats {
                        container_name: "b".to_string(),
                        input_bytes: 10,
                        input_records: 1,
                    },
                ]),
            },
        ];
        for case in cases {
            let got = parse_container_stats_dump(case.body);
            assert_eq!(got, case.expect, "case: {}", case.name);
        }
    }

    #[test]
    fn parse_container_stats_dump_caps_at_64_with_sort() {
        let mut entries = Vec::new();
        for i in 0..80 {
            entries.push(format!(
                r#"{{"container_name":"c{:02}","input_bytes":{},"input_records":{}}}"#,
                i,
                i,
                80 - i
            ));
        }
        let body = format!(r#"{{"containers":[{}]}}"#, entries.join(","));
        let parsed = parse_container_stats_dump(body.as_bytes()).expect("should parse");
        assert_eq!(parsed.len(), 64);
        assert_eq!(parsed[0].container_name, "c79");
        assert_eq!(parsed[0].input_bytes, 79);
        assert_eq!(parsed[63].container_name, "c16");
        assert_eq!(parsed[63].input_bytes, 16);
        for w in parsed.windows(2) {
            assert!(
                w[0].input_bytes >= w[1].input_bytes,
                "must be sorted by input_bytes desc"
            );
        }
    }

    fn sample_stats() -> FluentbitStats {
        FluentbitStats {
            available: true,
            collected_at: 1,
            input_bytes: Some(9),
            input_records: Some(3),
            output_proc_bytes: None,
            output_proc_records: None,
            filter_drop_records: None,
            output_dropped_records: None,
            output_errors: None,
            retries_failed: None,
            uptime_seconds: Some(1),
            errors: vec![],
            containers: None,
        }
    }

    #[test]
    fn merge_container_stats_dump_success_and_miss_leave_aggregates() {
        let mut stats = sample_stats();
        let warn = merge_container_stats_dump(
            &mut stats,
            br#"{"containers":[{"container_name":"hive","input_bytes":4,"input_records":2}]}"#,
        );
        assert!(warn.is_none());
        assert_eq!(
            stats.containers.as_ref().map(|c| c.len()),
            Some(1)
        );
        assert!(stats.available);
        assert!(stats.errors.is_empty());
        assert_eq!(stats.input_bytes, Some(9));

        let mut stats = sample_stats();
        stats.available = false;
        stats.errors = vec![fluentbit_error(REASON_UNREACHABLE)];
        let warn = merge_container_stats_dump(&mut stats, br#"{"containers":[]}"#);
        assert!(warn.is_none());
        assert!(stats.containers.is_none());
        assert!(!stats.available);
        assert_eq!(stats.errors[0].reason, REASON_UNREACHABLE);

        let mut stats = sample_stats();
        let warn = merge_container_stats_dump(&mut stats, b"{not json");
        assert_eq!(warn, Some(REASON_UNPARSEABLE));
        assert!(stats.containers.is_none());
        assert!(stats.available);
        assert!(stats.errors.is_empty());

        let mut stats = sample_stats();
        let oversized = vec![b'x'; CONTAINER_STATS_DUMP_MAX_BYTES + 1];
        let warn = merge_container_stats_dump(&mut stats, &oversized);
        assert_eq!(warn, Some(REASON_TOO_LARGE));
        assert!(stats.containers.is_none());
        assert!(stats.available);
        assert_eq!(stats.input_bytes, Some(9));
    }

    // ── SSRF allowlist ────────────────────────────────────────────────────

    #[test]
    fn allowlist_accepts_fluentbit_and_loopback() {
        for host in ["fluentbit", "127.0.0.1", "localhost", "[::1]"] {
            let url = ok_url(host);
            assert!(
                validate_fluentbit_url(&url).is_some(),
                "expected allow {url}"
            );
        }
        assert!(validate_fluentbit_url("https://fluentbit:2020/api/v1/metrics").is_some());
    }

    #[test]
    fn underscore_host_parse_behavior() {
        let raw = "http://fluent_bit:2020/api/v1/metrics";
        match url::Url::parse(raw) {
            Ok(u) => {
                assert_eq!(u.host_str(), Some("fluent_bit"));
                assert!(
                    validate_fluentbit_url(raw).is_some(),
                    "url crate accepted fluent_bit so the allowlist must too"
                );
            }
            Err(_) => {
                assert!(
                    validate_fluentbit_url(raw).is_none(),
                    "url crate rejected fluent_bit; do not silently rewrite it"
                );
            }
        }
    }

    #[test]
    fn allowlist_rejects_imds_and_other_hosts() {
        let rejected = [
            "http://169.254.169.254:2020/api/v1/metrics",
            "http://[fd00:ec2::254]:2020/api/v1/metrics",
            "http://[::ffff:169.254.169.254]:2020/api/v1/metrics",
            "gopher://fluentbit:2020/api/v1/metrics",
            "http://user:pass@fluentbit:2020/api/v1/metrics",
            "http://fluentbit:24224/api/v1/metrics",
            "http://fluentbit:2020/api/v1/metrics/prometheus",
            "http://fluentbit/api/v1/metrics",
            "http://10.0.0.5:2020/api/v1/metrics",
            "http://192.168.1.1:2020/api/v1/metrics",
            "http://evil.example:2020/api/v1/metrics",
            "http://fluentbit:2020/api/v1/uptime",
            "::not a url::",
        ];
        for url in rejected {
            assert!(
                validate_fluentbit_url(url).is_none(),
                "expected reject {url}"
            );
        }
    }

    #[test]
    fn bad_override_falls_back_to_default() {
        let resolved = resolve_fluentbit_target_from(Some(
            "http://169.254.169.254:2020/api/v1/metrics",
        ));
        assert_eq!(resolved, DEFAULT_FLUENTBIT_METRICS_URL);
        let empty = resolve_fluentbit_target_from(Some("  "));
        assert_eq!(empty, DEFAULT_FLUENTBIT_METRICS_URL);
        let none = resolve_fluentbit_target_from(None);
        assert_eq!(none, DEFAULT_FLUENTBIT_METRICS_URL);
        let good = resolve_fluentbit_target_from(Some("http://127.0.0.1:2020/api/v1/metrics"));
        assert_eq!(good, "http://127.0.0.1:2020/api/v1/metrics");
    }

    // ── HTTP scrape (redirect / size / freshness) ─────────────────────────

    async fn spawn_raw_http(status_and_headers: &str, body: &[u8]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock http");
        let addr = listener.local_addr().expect("local_addr");
        let header = format!(
            "{status_and_headers}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut payload = header.into_bytes();
        payload.extend_from_slice(body);
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf).await;
                let _ = sock.write_all(&payload).await;
            }
        });
        format!("http://127.0.0.1:{}", addr.port())
    }

    async fn spawn_raw_http_no_cl(status_line: &str, extra_headers: &str, body: &[u8]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock http");
        let addr = listener.local_addr().expect("local_addr");
        let header = format!("{status_line}\r\n{extra_headers}Connection: close\r\n\r\n");
        let mut payload = header.into_bytes();
        payload.extend_from_slice(body);
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf).await;
                let _ = sock.write_all(&payload).await;
            }
        });
        format!("http://127.0.0.1:{}", addr.port())
    }

    #[tokio::test]
    async fn redirect_to_imds_is_not_followed() {
        let url = spawn_raw_http(
            "HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/latest/meta-data",
            b"",
        )
        .await;
        let err = scrape_fluentbit_url(&url).await.expect_err("302");
        assert_eq!(err, REASON_REDIRECTED);

        let url = spawn_raw_http(
            "HTTP/1.1 302 Found\r\nLocation: http://[fd00:ec2::254]/latest/meta-data",
            b"",
        )
        .await;
        let stats = collect_from_url(&url).await;
        assert!(!stats.available);
        assert_eq!(stats.errors[0].reason, REASON_REDIRECTED);
        assert!(stats.input_bytes.is_none());
    }

    #[tokio::test]
    async fn content_length_over_cap_is_too_large() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf).await;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    MAX_BODY_BYTES + 1
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                // Do not send the claimed body — collector must not buffer it.
            }
        });
        let url = format!("http://127.0.0.1:{}", addr.port());
        let err = scrape_fluentbit_url(&url).await.expect_err("cap");
        assert_eq!(err, REASON_TOO_LARGE);
    }

    #[tokio::test]
    async fn streamed_body_over_cap_is_too_large() {
        let oversize = vec![b'x'; MAX_BODY_BYTES + 64];
        let url = spawn_raw_http_no_cl("HTTP/1.1 200 OK", "", &oversize).await;
        let err = scrape_fluentbit_url(&url).await.expect_err("cap");
        assert_eq!(err, REASON_TOO_LARGE);
    }

    #[tokio::test]
    async fn sequential_scrapes_are_not_cached() {
        let url = spawn_raw_http("HTTP/1.1 200 OK", SAMPLE.as_bytes()).await;
        let a = collect_from_url(&url).await;
        assert!(a.available);
        let first_at = a.collected_at;

        let url = spawn_raw_http(
            "HTTP/1.1 200 OK",
            b"fluentbit_uptime 99\nfluentbit_input_bytes_total{name=\"forward.0\"} 1\n",
        )
        .await;
        let b = collect_from_url(&url).await;
        assert!(b.available);
        assert_eq!(b.uptime_seconds, Some(99));
        assert_eq!(b.input_bytes, Some(1));
        // Different payload proves we did not reuse a cached HostStorage-style entry.
        assert_ne!(a.uptime_seconds, b.uptime_seconds);
        assert!(b.collected_at >= first_at);
    }

    #[tokio::test]
    async fn get_fluentbit_stats_fresh_collected_at() {
        let a = get_fluentbit_stats().await;
        let mut b = get_fluentbit_stats().await;
        if a.collected_at == b.collected_at {
            tokio::time::sleep(Duration::from_millis(1100)).await;
            b = get_fluentbit_stats().await;
        }
        assert_ne!(
            a.collected_at, b.collected_at,
            "no 60s TTL cache may pin collected_at"
        );
        assert!(!a.available);
        assert!(!b.available);
    }
}
