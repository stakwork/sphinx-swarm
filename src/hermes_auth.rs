//! Drives `hermes auth ...` inside the hermes container on behalf of callers
//! that can't reach the docker socket themselves (repo2graph, an external UI).
//!
//! `hermes auth add xai-oauth --no-browser` is a device-code flow: it prints a
//! verification URL plus a user code within the first second, then polls the
//! provider — for minutes — until someone approves the login in a browser. No
//! stdin, no TTY needed, but far too long to hold an HTTP request open. So
//! `start()` kicks the exec off, returns as soon as the URL is on the wire,
//! and `status()` is polled for the rest.

use crate::dock::exec_no_tty;
use crate::images::hermes::HERMES_BIN;
use crate::secrets;
use crate::utils::domain;
use anyhow::{anyhow, Result};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::Docker;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use rocket::tokio;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Container to exec into. Matches the node name used by the presets.
const HERMES_NODE: &str = "hermes";

pub const DEFAULT_PROVIDER: &str = "xai-oauth";

/// How long `start()` waits for the verification URL before replying anyway.
const FIRST_OUTPUT_TIMEOUT_MS: u64 = 8_000;
const POLL_INTERVAL_MS: u64 = 200;

/// Login output is a handful of lines; anything past this is a runaway CLI.
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// Finished sessions are only kept so a late poll can still read the result.
const MAX_SESSIONS: usize = 20;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthSession {
    pub session_id: String,
    pub provider: String,
    /// Everything the CLI has printed so far — the verification URL and user
    /// code live in here.
    pub output: String,
    pub done: bool,
    pub exit_code: Option<i64>,
}

static SESSIONS: Lazy<Mutex<HashMap<String, Arc<Mutex<AuthSession>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Providers are interpolated into an exec argv. That's not a shell, so
/// there's no quoting hazard, but an unchecked value could still smuggle in
/// flags (`--config`, `--help`) and make the CLI do something else entirely.
fn validate_provider(provider: &str) -> Result<String> {
    if provider.is_empty() || provider.len() > 64 {
        return Err(anyhow!("invalid hermes provider"));
    }
    // A leading dash would make the CLI read it as a flag, not a provider.
    if provider.starts_with('-') {
        return Err(anyhow!("invalid hermes provider (cannot start with '-')"));
    }
    if !provider
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(anyhow!(
            "invalid hermes provider (expected lowercase, digits and dashes)"
        ));
    }
    Ok(provider.to_string())
}

pub fn provider_or_default(provider: &Option<String>) -> Result<String> {
    match provider {
        Some(p) => validate_provider(p),
        None => Ok(DEFAULT_PROVIDER.to_string()),
    }
}

/// `hermes auth add <provider> --no-browser`, backgrounded.
pub async fn start(docker: &Docker, provider: &str) -> Result<AuthSession> {
    let provider = validate_provider(provider)?;
    let container = domain(HERMES_NODE);

    let cmd = vec![
        HERMES_BIN.to_string(),
        "auth".to_string(),
        "add".to_string(),
        provider.clone(),
        "--no-browser".to_string(),
    ];

    let exec_id = docker
        .create_exec(
            &container,
            CreateExecOptions {
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                // No TTY: keeps the CLI from emitting ANSI escapes and
                // spinner redraws, so the URL comes back as plain text.
                tty: Some(false),
                cmd: Some(cmd),
                ..Default::default()
            },
        )
        .await?
        .id;

    let started = docker.start_exec(&exec_id, None).await?;
    let mut output = match started {
        StartExecResults::Attached { output, .. } => output,
        StartExecResults::Detached => return Err(anyhow!("hermes auth exec detached")),
    };

    let session_id = secrets::random_word(12);
    let session = Arc::new(Mutex::new(AuthSession {
        session_id: session_id.clone(),
        provider,
        output: String::new(),
        done: false,
        exit_code: None,
    }));

    {
        let mut sessions = SESSIONS.lock().await;
        prune(&mut sessions).await;
        sessions.insert(session_id.clone(), session.clone());
    }

    let drain_into = session.clone();
    let docker = docker.clone();
    let exec_id_for_task = exec_id.clone();
    tokio::spawn(async move {
        while let Some(Ok(msg)) = output.next().await {
            let mut s = drain_into.lock().await;
            if s.output.len() < MAX_OUTPUT_BYTES {
                s.output.push_str(&msg.to_string());
            }
        }
        let exit_code = docker
            .inspect_exec(&exec_id_for_task)
            .await
            .ok()
            .and_then(|i| i.exit_code);
        let mut s = drain_into.lock().await;
        s.exit_code = exit_code;
        s.done = true;
    });

    // Hold the caller just long enough for the verification URL to show up,
    // so the common case is a single round trip.
    let deadline = FIRST_OUTPUT_TIMEOUT_MS / POLL_INTERVAL_MS;
    for _ in 0..deadline {
        {
            let s = session.lock().await;
            if !s.output.trim().is_empty() || s.done {
                return Ok(s.clone());
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
    }

    let snapshot = session.lock().await.clone();
    Ok(snapshot)
}

pub async fn status(session_id: &str) -> Result<AuthSession> {
    let session = {
        let sessions = SESSIONS.lock().await;
        sessions.get(session_id).cloned()
    };
    match session {
        Some(s) => Ok(s.lock().await.clone()),
        None => Err(anyhow!("unknown hermes auth session")),
    }
}

/// `hermes auth list <provider>` — fast, exits on its own.
pub async fn list(docker: &Docker, provider: &str) -> Result<String> {
    let provider = validate_provider(provider)?;
    run(docker, vec!["auth", "list", &provider]).await
}

/// `hermes auth logout <provider>` — drops every stored credential for it.
pub async fn logout(docker: &Docker, provider: &str) -> Result<String> {
    let provider = validate_provider(provider)?;
    run(docker, vec!["auth", "logout", &provider]).await
}

async fn run(docker: &Docker, args: Vec<&str>) -> Result<String> {
    let mut cmd = vec![HERMES_BIN.to_string()];
    cmd.extend(args.into_iter().map(|a| a.to_string()));
    exec_no_tty(docker, &domain(HERMES_NODE), cmd).await
}

/// Keep the map from growing without bound. In-flight logins are never
/// dropped; finished ones are, oldest-insertion-order first.
async fn prune(sessions: &mut HashMap<String, Arc<Mutex<AuthSession>>>) {
    if sessions.len() < MAX_SESSIONS {
        return;
    }
    let mut finished = Vec::new();
    for (id, s) in sessions.iter() {
        if s.lock().await.done {
            finished.push(id.clone());
        }
    }
    for id in finished {
        sessions.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_validation_rejects_flag_smuggling() {
        assert!(validate_provider("xai-oauth").is_ok());
        assert!(validate_provider("anthropic-oauth").is_ok());

        assert!(validate_provider("--help").is_err());
        assert!(validate_provider("xai oauth").is_err());
        assert!(validate_provider("xai/../etc").is_err());
        assert!(validate_provider("XAI-OAUTH").is_err());
        assert!(validate_provider("").is_err());
    }

    #[test]
    fn test_provider_defaults_to_xai_oauth() {
        assert_eq!(provider_or_default(&None).unwrap(), "xai-oauth");
        assert_eq!(
            provider_or_default(&Some("anthropic-oauth".to_string())).unwrap(),
            "anthropic-oauth"
        );
        assert!(provider_or_default(&Some("--config".to_string())).is_err());
    }
}
