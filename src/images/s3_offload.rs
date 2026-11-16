//! Env for a service that moves its data to S3 when its host opts in: the
//! LLM gateway's log payloads (`bifrost.rs`) and strut's run archive
//! (`repo2graph.rs`). Each service has its own contract of
//! `<VARS>_BUCKET`, `_PREFIX`, `_REGION`, `_ENDPOINT`, `_ACCESS_KEY_ID` and
//! `_SECRET_ACCESS_KEY`, and its own switch, so each is turned on by itself.

use crate::backup::swarm_prefix_for_host;
use crate::utils::getenv;

/// One service's S3 contract, and where its data goes in the fleet bucket.
pub struct S3Offload {
    /// The contract's var prefix, e.g. `BIFROST_LOGS_S3`.
    pub vars: &'static str,
    /// Top-level key root of this service's per-swarm default prefix,
    /// `<root>/swarm-<host-slug>`. Each service has its own root, so each can
    /// have its own bucket lifecycle rules. None may start with `swarm`: the
    /// backup prefixes (`swarm<N>/`, `swarm-<host-slug>/`) are swept by
    /// backup retention.
    pub root: &'static str,
    /// For the log lines: the service, what it moves, and where that stays
    /// when it doesn't.
    pub service: &'static str,
    pub what: &'static str,
    pub if_off: &'static str,
}

impl S3Offload {
    fn var(&self, name: &str) -> String {
        format!("{}_{}", self.vars, name)
    }

    /// The container env, read from this host's env by `lookup`.
    ///
    /// - Off (`Ok(vec![])`) unless `<VARS>_BUCKET` is set. Never derived from
    ///   `AWS_S3_BUCKET_NAME`: switching it on is a deliberate per-host act.
    /// - The prefix is the host's `<VARS>_PREFIX`, else
    ///   `<root>/swarm-<host-slug>` from `HOST`.
    /// - The region is the host's `<VARS>_REGION`, else `AWS_REGION`. The S3
    ///   clients need one even with an endpoint (R2 takes `auto`).
    /// - Endpoint and access keys pass through only when set (hosts not on
    ///   AWS); neither key means the instance role.
    ///
    /// `Err(reason)` when it is asked for but can't be done safely; the
    /// caller then passes none of the vars, so the data stays local rather
    /// than being mixed with another swarm's or lost.
    pub fn env(&self, lookup: impl Fn(&str) -> Option<String>) -> Result<Vec<String>, String> {
        let get = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let (bucket_var, prefix_var, region_var) =
            (self.var("BUCKET"), self.var("PREFIX"), self.var("REGION"));
        let (key_id_var, secret_var) = (self.var("ACCESS_KEY_ID"), self.var("SECRET_ACCESS_KEY"));

        let Some(bucket) = get(&bucket_var) else {
            return Ok(vec![]);
        };

        let prefix = match get(&prefix_var) {
            Some(explicit) => {
                let explicit = explicit.trim_matches('/').to_string();
                // `swarm7/...` or `swarm-host/...` would be swept by backup
                // retention.
                if explicit.is_empty() || explicit.starts_with("swarm") {
                    return Err(format!(
                        "{} {:?} is empty or under a backup prefix \
                         (swarm...), whose old objects backup retention deletes",
                        prefix_var, explicit
                    ));
                }
                explicit
            }
            None => match get("HOST") {
                Some(host) => format!("{}/{}", self.root, swarm_prefix_for_host(&host)),
                None => {
                    return Err(format!(
                        "{} is set but neither {} nor HOST is, and a shared \
                         default prefix would mix swarms' data",
                        bucket_var, prefix_var
                    ))
                }
            },
        };

        let access_key_id = get(&key_id_var);
        let secret_access_key = get(&secret_var);
        if access_key_id.is_some() != secret_access_key.is_some() {
            return Err(format!(
                "only one of {} / {} is set (both or neither)",
                key_id_var, secret_var
            ));
        }

        let Some(region) = get(&region_var).or_else(|| get("AWS_REGION")) else {
            return Err(format!(
                "{} is set but neither {} nor AWS_REGION is, and the S3 \
                 client needs a region",
                bucket_var, region_var
            ));
        };

        let mut env = vec![
            format!("{}={}", bucket_var, bucket),
            format!("{}={}", prefix_var, prefix),
            format!("{}={}", region_var, region),
        ];
        if let Some(endpoint) = get(&self.var("ENDPOINT")) {
            env.push(format!("{}={}", self.var("ENDPOINT"), endpoint));
        }
        if let (Some(id), Some(secret)) = (access_key_id, secret_access_key) {
            env.push(format!("{}={}", key_id_var, id));
            env.push(format!("{}={}", secret_var, secret));
        }
        Ok(env)
    }

    /// `env` read from the process env (this host's `.env`), logging where
    /// the data goes, or why it stays local.
    pub fn host_env(&self) -> Vec<String> {
        match self.env(|name| getenv(name).ok()) {
            Ok(vars) => {
                let prefix = format!("{}=", self.var("PREFIX"));
                if let Some(dest) = vars.iter().find_map(|v| v.strip_prefix(&prefix)) {
                    log::info!(
                        "{}: offloading {} to S3 under {}/",
                        self.service,
                        self.what,
                        dest
                    );
                }
                vars
            }
            Err(why) => {
                log::error!(
                    "{}: NOT offloading {} to S3, {}: {}",
                    self.service,
                    self.what,
                    self.if_off,
                    why
                );
                vec![]
            }
        }
    }
}
