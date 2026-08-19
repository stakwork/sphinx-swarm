//! Hermes Agent — the *subscription proxy*, which turns an OAuth-backed
//! provider session (SuperGrok / X Premium+ via xAI OAuth) into a local
//! OpenAI-compatible endpoint. Not the messaging gateway (`gateway run`,
//! port 8642) — that's a separate service with Telegram/Discord/etc. attached.
//!
//! # Why auth goes through swarm
//!
//! Logging in means running `hermes auth add xai-oauth` *inside* this
//! container. Neither repo2graph nor an external UI can do that themselves:
//!
//!   - `docker compose exec` cannot work at all. Swarm creates containers with
//!     bollard directly (see `builder.rs`), not compose, so there is no compose
//!     project to resolve against. The host-side equivalent would be
//!     `docker exec -it hermes.sphinx ...`.
//!   - Doing it from repo2graph would mean bind-mounting /var/run/docker.sock
//!     into it plus shipping a docker CLI in the stakgraph-mcp image. The
//!     socket is root-equivalent on the host and repo2graph runs LLM-directed
//!     code over arbitrary cloned repos, so that trade isn't worth making.
//!
//! Swarm already holds the socket, so it brokers the exec instead. See
//! `crate::hermes_auth` for the implementation.
//!
//! # Driving it over the API
//!
//! Four `SwarmCmd`s, all via the normal admin-JWT endpoint. There is no UI.
//!
//! Get a JWT (returns `{"token":"..."}`; port is 8000 in docker-compose.yml,
//! 8800 in second-brain-2.yml):
//!
//! ```text
//! JWT=$(curl -s -X POST http://localhost:8000/api/login \
//!   -H 'Content-Type: application/json' \
//!   -d '{"username":"admin","password":"password"}' | jq -r .token)
//! ```
//!
//! Start a login. Returns `{session_id, provider, output, done, exit_code}`.
//! `provider` is optional and defaults to `xai-oauth`:
//!
//! ```text
//! curl -H "x-jwt: $JWT" --get http://localhost:8000/api/cmd \
//!   --data-urlencode 'tag=SWARM' \
//!   --data-urlencode 'txt={"type":"Swarm","data":{"cmd":"HermesAuthStart","content":{"provider":"xai-oauth"}}}'
//! ```
//!
//! `auth add` is a *device-code* flow: it prints a verification URL and user
//! code within about a second, then polls xAI for minutes until a human
//! approves in a browser. No stdin and no TTY are involved, but it's far too
//! slow to hold an HTTP request open — so `HermesAuthStart` blocks only until
//! the URL is on the wire (8s cap) and backgrounds the rest. The URL and code
//! are in the `output` field of the response.
//!
//! Poll until `done` is true. Note `content` here is a bare string, not an
//! object:
//!
//! ```text
//! curl -H "x-jwt: $JWT" --get http://localhost:8000/api/cmd \
//!   --data-urlencode 'tag=SWARM' \
//!   --data-urlencode 'txt={"type":"Swarm","data":{"cmd":"HermesAuthStatus","content":"<session_id>"}}'
//! ```
//!
//! `HermesAuthList` and `HermesAuthLogout` take the same `{"provider":...}`
//! content as `HermesAuthStart`, run synchronously, and return the CLI output
//! as a string. Logout drops every stored credential for that provider.
//!
//! Sessions live in memory only — a swarm restart forgets them. That costs
//! nothing, since a login that was still pending is dead anyway; the stored
//! credentials themselves are on the volume (see HERMES_HOME below).
//!
//! # There is no token to hand out
//!
//! A successful login does not yield a bearer token you can pass around. The
//! OAuth credentials stay in auth.json on the volume and rotate transparently;
//! `auth list` prints only metadata (provider, pool index, active/cooldown,
//! expiry), never the raw access or refresh token. So `HermesAuthList` cannot
//! leak a credential even though it returns CLI output verbatim.
//!
//! The proxy *is* the credential. A harness consumes it by using
//! `http://hermes.sphinx:8645/v1` as an OpenAI-compatible base URL — clients
//! send **any** bearer token, and the proxy attaches the real OAuth credential
//! on the way upstream.
//!
//! That "any bearer token" is why this container publishes no host port and
//! carries no traefik labels: anything that can reach the port can spend the
//! subscription. Keep it on the sphinx-swarm network. If you ever do need it
//! from outside, put real auth in front of it rather than publishing it.
//!
//! # Gotchas
//!
//!   - `hermes` is **not on PATH** for `docker exec`; the CLI lives in a venv.
//!     Hence `HERMES_BIN` below.
//!   - `HERMES_HOME` must point at the named volume or credentials are lost on
//!     every image pull by the auto-updater. See the test at the bottom.
//!   - `proxy start` defaults to `--provider nous`. We pass `--provider xai`
//!     explicitly, or an xai-oauth login would authenticate fine and then
//!     serve the wrong upstream. Note the auth namespace differs from the
//!     proxy one: `xai-oauth` for `auth add`, `xai` for `proxy start`.
//!   - Neither host-published nor traefik-fronted, deliberately — see above.
//!     repo2graph reaches it at `HERMES_URL=http://hermes.sphinx:8645`
//!     (see `repo2graph.rs`).
//!   - Not in any preset's `auto_update` list. Those are all Sphinx/stakwork
//!     images; auto-recreating a third-party `:latest` on every upstream push
//!     is an opt-in decision, not a default.
//!
//! Upstream docs: <https://hermes-agent.nousresearch.com/docs/guides/xai-grok-oauth>

use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

/// Absolute path to the CLI inside the official image. `hermes` is not on
/// PATH for `docker exec`, so every exec we issue has to spell it out.
pub const HERMES_BIN: &str = "/opt/hermes/.venv/bin/hermes";

/// Mutable state dir. The image keeps its immutable app tree under
/// /opt/hermes and expects writable state at /opt/data, which is where our
/// named volume lands. HERMES_HOME points the CLI at it so the OAuth
/// credentials (auth.json) survive container recreation — without it they'd
/// go to ~/.hermes inside the container layer and be lost on every image
/// pull by the auto-updater.
const HERMES_HOME: &str = "/opt/data";

/// The subscription proxy: turns OAuth-backed provider sessions (SuperGrok /
/// X Premium+ via `auth add xai-oauth`) into an OpenAI-compatible endpoint.
/// Not the messaging gateway (`gateway run`, port 8642) — that's a different
/// service.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct HermesImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    /// Upstream the proxy forwards to. `proxy start` takes `<nous|xai>` and
    /// **defaults to nous**, which is not what we want — a stack that logged
    /// in with `auth add xai-oauth` but proxied to nous would authenticate
    /// fine and then serve the wrong upstream. Note this namespace differs
    /// from the auth provider's ("xai" here, "xai-oauth" there).
    #[serde(default = "default_proxy_provider")]
    pub provider: String,
}

fn default_proxy_provider() -> String {
    "xai".to_string()
}

impl HermesImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            provider: default_proxy_provider(),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
}

#[async_trait]
impl DockerConfig for HermesImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(hermes(self))
    }
}

impl DockerHubImage for HermesImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "nousresearch".to_string(),
            repo: "hermes-agent".to_string(),
            root_volume: HERMES_HOME.to_string(),
        }
    }
}

fn hermes(node: &HermesImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![node.port.clone()];

    let env = vec![format!("HERMES_HOME={}", HERMES_HOME)];

    // The proxy accepts ANY bearer token — it attaches the real OAuth
    // credential upstream itself, so anything that can reach this port can
    // spend the subscription. Hence both: no traefik labels, and no host port
    // publishing (the empty `ports` passed to host_config below). Reachable
    // only from inside the sphinx-swarm network, at http://hermes.sphinx:PORT.
    Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports),
        cmd: Some(vec![
            "proxy".to_string(),
            "start".to_string(),
            "--provider".to_string(),
            node.provider.clone(),
            "--port".to_string(),
            node.port.clone(),
            "--host".to_string(),
            "0.0.0.0".to_string(),
        ]),
        env: Some(env),
        // Mirrors stdin_open/tty in the upstream compose example.
        open_stdin: Some(true),
        tty: Some(true),
        host_config: host_config(&name, vec![], root_vol, None, None),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermes_runs_the_proxy_against_the_xai_upstream() {
        let img = HermesImage::new("hermes", "latest", "8645");
        let c = hermes(&img);

        // --provider matters: `proxy start` defaults to nous, so without it
        // an xai-oauth login would serve the wrong upstream.
        assert_eq!(
            c.cmd.unwrap(),
            vec![
                "proxy", "start", "--provider", "xai", "--port", "8645", "--host", "0.0.0.0"
            ]
        );
        assert_eq!(c.image.unwrap(), "nousresearch/hermes-agent:latest");
        assert_eq!(c.hostname.unwrap(), "hermes.sphinx");
    }

    #[test]
    fn test_hermes_port_is_not_published_to_the_host() {
        let img = HermesImage::new("hermes", "latest", "8645");
        let c = hermes(&img);

        // The proxy takes any bearer token, so publishing it on the host would
        // put a spendable xAI subscription on a public interface.
        let bindings = c.host_config.unwrap().port_bindings.unwrap();
        assert!(
            bindings.is_empty(),
            "hermes must not publish a host port, got: {:?}",
            bindings
        );
    }

    #[test]
    fn test_hermes_home_is_on_the_named_volume() {
        let img = HermesImage::new("hermes", "latest", "8645");
        let c = hermes(&img);

        assert!(c
            .env
            .unwrap()
            .contains(&"HERMES_HOME=/opt/data".to_string()));

        // Credentials written under HERMES_HOME must land on the volume, or
        // they're lost every time the auto-updater recreates the container.
        let binds = c.host_config.unwrap().binds.unwrap();
        assert!(
            binds.iter().any(|b| b == "hermes.sphinx:/opt/data:rw"),
            "binds should mount the named volume at /opt/data, got: {:?}",
            binds
        );
    }
}
