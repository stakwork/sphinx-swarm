//! Hourly per-container memory log, for finding a container that leaks.
//!
//! A host with no swap that slowly runs out of memory never kills anything:
//! it starts re-reading program files from disk and stops answering, SSH
//! included (sphinx-v2-one, 2026-10-28, after ~5 GB built up over 99 days).
//! Host-level history (`sar`) shows the growth but not who grew, so every
//! hour this appends one JSON line to `MEMORY_LOG_PATH` (default
//! `/vol/memory-log.jsonl`, the host's `/home/admin/vol`, which outlives the
//! swarm container and reboots): the host's total and available memory, and
//! for every running container its anonymous memory — what a leak grows, and
//! what cannot be reclaimed without swap — and its usage as `docker stats`
//! shows it. Rotated to `<path>.1` past 10 MB (about a year of lines).
//!
//! `MEMORY_LOG_CRON` overrides the schedule (default hourly).

use crate::utils::getenv;
use anyhow::Result;
use bollard::container::{ListContainersOptions, MemoryStats, MemoryStatsStats, StatsOptions};
use bollard::Docker;
use futures_util::StreamExt;
use rocket::tokio;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::AsyncWriteExt;
use tokio_cron_scheduler::{Job, JobScheduler};

pub static LOG_MEMORY: AtomicBool = AtomicBool::new(false);

const DEFAULT_PATH: &str = "/vol/memory-log.jsonl";
const MAX_BYTES: u64 = 10 * 1024 * 1024;
const MB: u64 = 1024 * 1024;

#[derive(Serialize, Debug, PartialEq)]
pub struct Sample {
    pub at: String,
    pub host_total_mb: Option<u64>,
    pub host_available_mb: Option<u64>,
    /// Largest anonymous memory first.
    pub containers: Vec<ContainerMemory>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct ContainerMemory {
    pub name: String,
    pub anon_mb: u64,
    pub usage_mb: u64,
}

pub async fn memory_log_cron(docker: Docker) -> Result<JobScheduler> {
    log::info!(":Memory log cron");
    let sched = JobScheduler::new().await?;

    let cron_time = match getenv("MEMORY_LOG_CRON") {
        Ok(env) => env,
        Err(_) => "0 0 * * * *".to_string(),
    };

    sched
        .add(Job::new_async(cron_time.as_str(), |_uuid, _l| {
            Box::pin(async move {
                if !LOG_MEMORY.load(Ordering::Relaxed) {
                    LOG_MEMORY.store(true, Ordering::Relaxed);
                }
            })
        })?)
        .await?;

    sched.start().await?;
    let path = getenv("MEMORY_LOG_PATH").unwrap_or(DEFAULT_PATH.to_string());
    tokio::spawn(async move {
        loop {
            let go = LOG_MEMORY.load(Ordering::Relaxed);
            if go {
                if let Err(e) = log_memory(&docker, &path).await {
                    log::error!("memory log failed: {:?}", e);
                }

                LOG_MEMORY.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}

async fn log_memory(docker: &Docker, path: &str) -> Result<()> {
    let sample = collect(docker).await?;
    let top: Vec<String> = sample
        .containers
        .iter()
        .take(3)
        .map(|c| format!("{} {}MB", c.name, c.anon_mb))
        .collect();
    log::info!(
        "memory log: host available {:?}MB, top {}",
        sample.host_available_mb,
        top.join(", ")
    );
    append(path, &serde_json::to_string(&sample)?).await
}

async fn collect(docker: &Docker) -> Result<Sample> {
    let meminfo = tokio::fs::read_to_string("/proc/meminfo")
        .await
        .unwrap_or_default();
    let (host_total_mb, host_available_mb) = parse_meminfo(&meminfo);

    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await?;

    let mut out = Vec::new();
    for container in containers {
        let (Some(id), Some(name)) = (container.id.as_ref(), container_name(&container.names))
        else {
            continue;
        };
        let mut stream = docker
            .stats(
                id,
                Some(StatsOptions {
                    stream: false,
                    one_shot: true,
                }),
            )
            .take(1);
        if let Some(Ok(stats)) = stream.next().await {
            let (anon_mb, usage_mb) = memory_mb(&stats.memory_stats);
            out.push(ContainerMemory {
                name,
                anon_mb,
                usage_mb,
            });
        }
    }
    out.sort_by(|a, b| b.anon_mb.cmp(&a.anon_mb));

    Ok(Sample {
        at: chrono::Utc::now().to_rfc3339(),
        host_total_mb,
        host_available_mb,
        containers: out,
    })
}

fn container_name(names: &Option<Vec<String>>) -> Option<String> {
    names
        .as_ref()?
        .first()
        .map(|n| n.trim_start_matches('/').to_string())
}

/// (anonymous memory, usage less inactive file cache — what `docker stats`
/// shows), in MB. cgroup v2 reports `anon`; v1 reports `total_rss`.
pub fn memory_mb(m: &MemoryStats) -> (u64, u64) {
    let usage = m.usage.unwrap_or(0);
    let (anon, inactive_file) = match m.stats {
        Some(MemoryStatsStats::V2(s)) => (s.anon, s.inactive_file),
        Some(MemoryStatsStats::V1(s)) => (s.total_rss, s.total_inactive_file),
        None => (0, 0),
    };
    (anon / MB, usage.saturating_sub(inactive_file) / MB)
}

/// (MemTotal, MemAvailable) in MB. Inside a container `/proc/meminfo` is
/// the host's.
pub fn parse_meminfo(text: &str) -> (Option<u64>, Option<u64>) {
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            .map(|kb| kb / 1024)
    };
    (field("MemTotal:"), field("MemAvailable:"))
}

async fn append(path: &str, line: &str) -> Result<()> {
    if let Ok(meta) = tokio::fs::metadata(path).await {
        if meta.len() > MAX_BYTES {
            tokio::fs::rename(path, format!("{}.1", path)).await?;
        }
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(format!("{}\n", line).as_bytes()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meminfo_total_and_available() {
        let text = "MemTotal:        7998464 kB\nMemFree:          186328 kB\nMemAvailable:     295132 kB\n";
        assert_eq!(parse_meminfo(text), (Some(7811), Some(288)));
        assert_eq!(parse_meminfo(""), (None, None));
    }

    #[test]
    fn memory_from_cgroup_v2() {
        let m: MemoryStats = serde_json::from_value(serde_json::json!({
            "usage": 1_200 * MB,
            "limit": 8_000 * MB,
            "stats": {
                "anon": 900 * MB, "file": 300 * MB, "kernel_stack": 0, "slab": 0,
                "sock": 0, "shmem": 0, "file_mapped": 0, "file_dirty": 0,
                "file_writeback": 0, "anon_thp": 0, "inactive_anon": 0,
                "active_anon": 900 * MB, "inactive_file": 250 * MB,
                "active_file": 50 * MB, "unevictable": 0, "slab_reclaimable": 0,
                "slab_unreclaimable": 0, "pgfault": 0, "pgmajfault": 0,
                "workingset_refault": 0, "workingset_activate": 0,
                "workingset_nodereclaim": 0, "pgrefill": 0, "pgscan": 0,
                "pgsteal": 0, "pgactivate": 0, "pgdeactivate": 0,
                "pglazyfree": 0, "pglazyfreed": 0, "thp_fault_alloc": 0,
                "thp_collapse_alloc": 0
            }
        }))
        .unwrap();
        assert_eq!(memory_mb(&m), (900, 950));
    }

    #[test]
    fn container_name_drops_the_slash() {
        let names = Some(vec!["/mixer.sphinx".to_string()]);
        assert_eq!(container_name(&names), Some("mixer.sphinx".to_string()));
        assert_eq!(container_name(&None), None);
    }

    #[tokio::test]
    async fn append_rotates_past_the_cap() {
        let dir = std::env::temp_dir().join(format!("memlog-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("m.jsonl");
        let path = path.to_str().unwrap();
        append(path, "a").await.unwrap();
        append(path, "b").await.unwrap();
        assert_eq!(tokio::fs::read_to_string(path).await.unwrap(), "a\nb\n");

        tokio::fs::write(path, vec![b'x'; (MAX_BYTES + 1) as usize])
            .await
            .unwrap();
        append(path, "c").await.unwrap();
        assert_eq!(tokio::fs::read_to_string(path).await.unwrap(), "c\n");
        assert!(tokio::fs::metadata(format!("{}.1", path)).await.is_ok());
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}
