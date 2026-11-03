//! The 1:1 voice service: a member talks to the org's agent (Jamie) by voice
//! from the Peel app. It is sphinx-voice's member mode (stakwork/sphinx-voice,
//! AGENTS.md "1:1 mode"): the same image as meeting-mode Jamie, run with
//! `node dist/one-to-one.js`.
//!
//! How it sits in the stack:
//!   - repo2graph's strut (the lab, at `/lab`) is its only caller. A member's
//!     `POST /lab/voice/calls` is forwarded to `POST http://voice.sphinx:8080/calls`
//!     with the member as `x-strut-actor` and `token` as the bearer
//!     (`STRUT_VOICE_URL` and `STRUT_VOICE_TOKEN`, see `repo2graph.rs`). The
//!     token is generated once, when the node is created, and persisted with
//!     the stack, like the browser's path.
//!   - It calls that strut back on the swarm network: every turn runs the
//!     orchestrator as the member (`x-strut-actor`), and speech goes through
//!     strut's `/audio/transcribe` and `/audio/speak`. Its credential is
//!     boltwall's `stakwork_secret`, which is repo2graph's `API_TOKEN`. Strut
//!     posts a turn's `run.end`, and its jobs', to `CALLBACK_URL_BASE`.
//!   - It connects out to the central LiveKit and joins each call's room
//!     itself. The keys come from the swarm's `.env` (`LIVEKIT_API_KEY`,
//!     `LIVEKIT_API_SECRET`, and `LIVEKIT_URL` when it is not the central
//!     one). The super writes them at creation when it has them, and an
//!     operator sets them on a running swarm through the super's env update.
//!     A stack whose `.env` has no keys gets no voice node.
//!   - Neither host-published nor traefik-fronted.
//!   - Nothing is kept: a call lives in the process. The volume is only what
//!     `host_config` always mounts.
//!   - On `latest` and in `auto_update`, like repo2graph: the two speak strut's
//!     contract (the orchestrator turn, `/audio/*`, `/voice/calls`) and move
//!     together.

use super::*;
use crate::config::Node;
use crate::images::boltwall::BoltwallImage;
use crate::images::repo2graph::Repo2GraphImage;
use crate::secrets;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

/// The central LiveKit, used when the swarm's `.env` names no other.
pub const LIVEKIT_URL: &str = "wss://livekit.sphinx.chat";

/// Where repo2graph serves its strut (mcp's `mountLab`).
const LAB_PATH: &str = "/lab";

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct VoiceImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    /// The secret strut sends as the bearer on `POST /calls`. A node written
    /// without one gets one.
    #[serde(default = "new_token")]
    pub token: String,
}

fn new_token() -> String {
    secrets::hex_secret_32()
}

impl VoiceImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            token: new_token(),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    /// Where strut reaches the service, for `/calls` and job callbacks.
    pub fn url(&self) -> String {
        format!("http://{}:{}", domain(&self.name), self.port)
    }
}

/// What the service reads from the swarm's `.env`.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceEnv {
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    /// `VOICE_MODEL`: the model a voice turn asks for. Unset, the service's
    /// default (`haiku`), which every swarm's gateway allows.
    pub model: Option<String>,
    /// `STRUT_TTS_VOICE`: the voice a call starts in. Unset, strut's default.
    pub tts_voice: Option<String>,
}

impl VoiceEnv {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| getenv(name).ok())
    }
    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let required = |name: &str| {
            get(name).ok_or_else(|| {
                anyhow!(
                    "Voice: {} is not set in .env (the central LiveKit's keys)",
                    name
                )
            })
        };
        Ok(Self {
            livekit_url: get("LIVEKIT_URL").unwrap_or_else(|| LIVEKIT_URL.to_string()),
            livekit_api_key: required("LIVEKIT_API_KEY")?,
            livekit_api_secret: required("LIVEKIT_API_SECRET")?,
            model: get("VOICE_MODEL"),
            tts_voice: get("STRUT_TTS_VOICE"),
        })
    }
}

/// Whether the swarm's `.env` has the LiveKit keys: the voice node is added to
/// a stack only then (`second_brain_imgs`, `migrate_stack`).
pub fn livekit_configured() -> bool {
    VoiceEnv::from_env().is_ok()
}

#[async_trait]
impl DockerConfig for VoiceImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let repo2graph = li.find_repo2graph().context("Voice: No Repo2Graph")?;
        let boltwall = li.find_boltwall();
        Ok(voice(self, &repo2graph, &boltwall, &VoiceEnv::from_env()?))
    }
}

impl DockerHubImage for VoiceImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "sphinx-voice".to_string(),
            // Nothing is kept (see the module doc). Not `/app`, where the
            // image's code and its turn-detector model are.
            root_volume: "/data".to_string(),
        }
    }
}

fn voice(
    node: &VoiceImage,
    repo2graph: &Repo2GraphImage,
    boltwall: &Option<BoltwallImage>,
    venv: &VoiceEnv,
) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();
    let root_vol = &repo.root_volume;

    let mut env = vec![
        format!("PORT={}", node.port),
        // The image binds loopback unless told otherwise.
        "LISTEN_HOST=0.0.0.0".to_string(),
        format!(
            "STRUT_URL=http://{}:{}{}",
            domain(&repo2graph.name),
            repo2graph.port,
            LAB_PATH
        ),
        format!("STRUT_VOICE_TOKEN={}", node.token),
        format!("CALLBACK_URL_BASE={}", node.url()),
        format!("LIVEKIT_URL={}", venv.livekit_url),
        format!("LIVEKIT_API_KEY={}", venv.livekit_api_key),
        format!("LIVEKIT_API_SECRET={}", venv.livekit_api_secret),
    ];
    if let Some(bw) = boltwall {
        if let Some(api_token) = &bw.stakwork_secret {
            env.push(format!("STRUT_API_KEY={}", api_token));
        }
    }
    if let Some(model) = &venv.model {
        env.push(format!("VOICE_MODEL={}", model));
    }
    if let Some(v) = &venv.tts_voice {
        env.push(format!("STRUT_TTS_VOICE={}", v));
    }

    // No host port (the empty `ports`) and no traefik labels: strut reaches it
    // on the swarm network, and it only connects out to LiveKit.
    Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        // The image's own command is meeting-mode Jamie.
        cmd: Some(vec!["node".to_string(), "dist/one-to-one.js".to_string()]),
        exposed_ports: exposed_ports(vec![node.port.clone()]),
        env: Some(env),
        host_config: host_config(&name, vec![], root_vol, None, None),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn img() -> VoiceImage {
        let mut v = VoiceImage::new("voice", "latest", "8080");
        v.links(vec!["repo2graph", "boltwall"]);
        v
    }

    fn venv() -> VoiceEnv {
        VoiceEnv {
            livekit_url: LIVEKIT_URL.to_string(),
            livekit_api_key: "lk-key".to_string(),
            livekit_api_secret: "lk-secret".to_string(),
            model: None,
            tts_voice: None,
        }
    }

    fn config(boltwall: &Option<BoltwallImage>, venv: &VoiceEnv) -> (VoiceImage, Config<String>) {
        let node = img();
        let r2g = Repo2GraphImage::new("repo2graph", "latest", "3355");
        let c = voice(&node, &r2g, boltwall, venv);
        (node, c)
    }

    #[test]
    fn test_voice_runs_member_mode_from_the_published_image() {
        let (_, c) = config(&None, &venv());
        assert_eq!(c.image.unwrap(), "sphinxlightning/sphinx-voice:latest");
        assert_eq!(c.hostname.unwrap(), "voice.sphinx");
        assert_eq!(
            c.cmd.unwrap(),
            vec!["node".to_string(), "dist/one-to-one.js".to_string()]
        );
    }

    #[test]
    fn test_voice_env_points_at_the_lab_strut_and_back_at_itself() {
        let mut bw = BoltwallImage::new("boltwall", "latest", "8444");
        bw.stakwork_secret = Some("swarm-api-token".to_string());
        let (node, c) = config(&Some(bw), &venv());
        let env = c.env.unwrap();
        for e in [
            "PORT=8080",
            "LISTEN_HOST=0.0.0.0",
            "STRUT_URL=http://repo2graph.sphinx:3355/lab",
            "STRUT_API_KEY=swarm-api-token",
            "CALLBACK_URL_BASE=http://voice.sphinx:8080",
            "LIVEKIT_URL=wss://livekit.sphinx.chat",
            "LIVEKIT_API_KEY=lk-key",
            "LIVEKIT_API_SECRET=lk-secret",
        ] {
            assert!(
                env.contains(&e.to_string()),
                "missing {}, got: {:?}",
                e,
                env
            );
        }
        assert!(env.contains(&format!("STRUT_VOICE_TOKEN={}", node.token)));
        // the service's own defaults stand
        assert!(!env.iter().any(|e| e.starts_with("VOICE_MODEL=")));
        assert!(!env.iter().any(|e| e.starts_with("STRUT_TTS_VOICE=")));
    }

    #[test]
    fn test_voice_without_a_boltwall_secret_sends_no_strut_key() {
        let (_, c) = config(&None, &venv());
        assert!(!c
            .env
            .unwrap()
            .iter()
            .any(|e| e.starts_with("STRUT_API_KEY=")));
    }

    #[test]
    fn test_voice_model_and_tts_voice_come_from_the_env_when_set() {
        let mut v = venv();
        v.model = Some("sonnet".to_string());
        v.tts_voice = Some("am_michael".to_string());
        let (_, c) = config(&None, &v);
        let env = c.env.unwrap();
        assert!(env.contains(&"VOICE_MODEL=sonnet".to_string()));
        assert!(env.contains(&"STRUT_TTS_VOICE=am_michael".to_string()));
    }

    #[test]
    fn test_voice_port_is_not_published_to_the_host() {
        let (_, c) = config(&None, &venv());
        assert!(c.labels.is_none(), "voice must not be traefik-fronted");
        let hc = c.host_config.unwrap();
        assert!(
            hc.port_bindings.unwrap().is_empty(),
            "voice must not publish a host port"
        );
        assert_eq!(hc.network_mode.unwrap(), "sphinx-swarm");
    }

    #[test]
    fn test_the_env_needs_the_livekit_keys_and_defaults_the_url() {
        let lookup = |vars: &[(&str, &str)]| {
            let m: HashMap<String, String> = vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            VoiceEnv::from_lookup(|k| m.get(k).cloned())
        };
        assert!(lookup(&[]).is_err());
        assert!(lookup(&[("LIVEKIT_API_KEY", "k")]).is_err());
        assert!(lookup(&[("LIVEKIT_API_SECRET", "s")]).is_err());

        let ok = lookup(&[("LIVEKIT_API_KEY", "k"), ("LIVEKIT_API_SECRET", "s")]).unwrap();
        assert_eq!(ok.livekit_url, LIVEKIT_URL);
        assert_eq!(ok.model, None);

        let other = lookup(&[
            ("LIVEKIT_API_KEY", "k"),
            ("LIVEKIT_API_SECRET", "s"),
            ("LIVEKIT_URL", "wss://livekit.example.com"),
        ])
        .unwrap();
        assert_eq!(other.livekit_url, "wss://livekit.example.com");
    }

    #[test]
    fn test_token_is_a_secret_made_once() {
        let a = img();
        let b = img();
        assert_eq!(a.token.len(), 64);
        assert!(a.token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a.token, b.token, "every node gets its own token");

        // persisted with the stack, so a restart keeps it
        let json = serde_json::to_string(&a).unwrap();
        let back: VoiceImage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.token, a.token);
    }

    #[test]
    fn test_a_node_written_without_a_token_gets_one() {
        let json = r#"{"name":"voice","version":"latest","port":"8080","links":[]}"#;
        let node: VoiceImage = serde_json::from_str(json).unwrap();
        assert_eq!(node.token.len(), 64);
    }

    #[test]
    fn test_url_has_no_secret_in_it() {
        assert_eq!(img().url(), "http://voice.sphinx:8080");
    }
}
