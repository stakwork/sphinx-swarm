use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::boltwall::BoltwallImage;
use crate::images::hermes::HermesImage;
use crate::images::jarvis::JarvisImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::redis::RedisImage;
use crate::images::traefik::shared_host;
use crate::utils::{domain, exposed_ports, getenv, host_config, volume_string};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Repo2GraphImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>, // openai by default
}

impl Repo2GraphImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            host: None,
            llm_provider: None,
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(shared_host) = shared_host() {
            self.host = Some(format!("{}.{}", self.name, shared_host))
        } else {
            if let Some(h) = eh {
                self.host = Some(format!("{}.{}", self.name, h));
            }
        }
    }
}

// with ndeo4j
#[async_trait]
impl DockerConfig for Repo2GraphImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let neo4j = li.find_neo4j().context("Repo2Graph: No Neo4j")?;
        let boltwall = li.find_boltwall();
        let jarvis = li.find_jarvis();
        let hermes = li.find_hermes();
        let redis = li.find_redis();
        Ok(repo2graph(self, &neo4j, &boltwall, &jarvis, &hermes, &redis)?)
    }
}

impl DockerHubImage for Repo2GraphImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::Ghcr,
            org: "stakwork".to_string(),
            repo: "stakgraph-mcp".to_string(),
            root_volume: "/root".to_string(),
        }
    }
}

fn repo2graph(
    img: &Repo2GraphImage,
    neo4j: &Neo4jImage,
    boltwall: &Option<BoltwallImage>,
    jarvis: &Option<JarvisImage>,
    hermes: &Option<HermesImage>,
    redis: &Option<RedisImage>,
) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("PORT={}", img.port),
        format!("NEO4J_HOST={}:{}", domain(&neo4j.name), neo4j.bolt_port),
        format!("NEO4J_PASSWORD={}", neo4j.password),
        format!("SAGE_CONFIG_PATH={}/sage_config.json", root_vol),
        format!("USE_STAGEHAND=1"),
    ];
    if let Some(llm_provider) = &img.llm_provider {
        env.push(format!("LLM_PROVIDER={}", llm_provider));
    }
    if let Ok(github_request_token) = getenv("GITHUB_REQUEST_TOKEN") {
        env.push(format!("PAT={}", github_request_token))
    }
    if let Some(boltwall) = boltwall {
        if let Some(api_token) = &boltwall.stakwork_secret {
            env.push(format!("API_TOKEN={}", api_token));
        }
    }
    if let Some(j) = jarvis {
        env.push(format!("JARVIS_URL=http://{}:{}", domain(&j.name), j.port));
    }
    if let Some(h) = hermes {
        env.push(format!("HERMES_URL=http://{}:{}", domain(&h.name), h.port));
    }
    if let Some(r) = redis {
        env.push(format!("REDIS_URL=redis://{}:{}", domain(&r.name), r.http_port));
    }

    if let Ok(openai_api_key) = getenv("OPENAI_API_KEY") {
        env.push(format!("OPENAI_API_KEY={}", openai_api_key));
    }
    if let Ok(anthropic_api_key) = getenv("ANTHROPIC_API_KEY") {
        env.push(format!("ANTHROPIC_API_KEY={}", anthropic_api_key));
    }
    if let Ok(openrouter_api_key) = getenv("OPENROUTER_API_KEY") {
        env.push(format!("OPENROUTER_API_KEY={}", openrouter_api_key));
    }

    let sessions_dir = "/usr/src/app/sessions";
    env.push(format!("SESSIONS_DIR={}", sessions_dir));

    // Durable dir for agent-written output files (reports, answer.json, etc.).
    // Backed by a named volume so artifacts survive container restarts; the mcp
    // prunes entries older than 7 days on startup and every 6h.
    let artifacts_dir = "/usr/src/app/artifacts";
    env.push(format!("AGENT_ARTIFACTS_DIR={}", artifacts_dir));

    // Durable dir for MCP request-tracking store. Backed by a named volume so
    // in-flight and historical request records survive container recreation on
    // auto-updater image pulls. sweepOrphanedReqs converts surviving pending
    // records to failed (retryable) on boot — durability, not continuation.
    let reqs_dir = "/usr/src/app/reqs";
    env.push(format!("REQS_DIR={}", reqs_dir));

    let tests_vol = volume_string(
        &format!("{}-tests", img.name),
        "/usr/src/app/tests/generated_tests",
    );
    let sessions_vol = volume_string(&format!("{}-sessions", img.name), sessions_dir);
    let artifacts_vol = volume_string(&format!("{}-artifacts", img.name), artifacts_dir);
    let reqs_vol = volume_string(&format!("{}-reqs", img.name), reqs_dir);
    let extra_vols = vec![tests_vol, sessions_vol, artifacts_vol, reqs_vol];
    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, Some(extra_vols), None),
        env: Some(env),
        ..Default::default()
    };
    if let Some(host) = &img.host {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false))
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_repo2graph_image() -> Repo2GraphImage {
        Repo2GraphImage::new("repo2graph", "latest", "8888")
    }

    fn test_neo4j_image() -> Neo4jImage {
        Neo4jImage::new("neo4j", "5.19.0")
    }

    #[test]
    fn test_reqs_dir_env_var_is_emitted() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("GITHUB_REQUEST_TOKEN");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");

        let img = test_repo2graph_image();
        let neo4j = test_neo4j_image();
        let config = repo2graph(&img, &neo4j, &None, &None, &None, &None).unwrap();
        let env = config.env.unwrap();

        assert!(
            env.contains(&"REQS_DIR=/usr/src/app/reqs".to_string()),
            "env should contain REQS_DIR=/usr/src/app/reqs, got: {:?}",
            env
        );
    }

    #[test]
    fn test_reqs_vol_is_in_binds() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("GITHUB_REQUEST_TOKEN");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");

        let img = test_repo2graph_image();
        let neo4j = test_neo4j_image();
        let config = repo2graph(&img, &neo4j, &None, &None, &None, &None).unwrap();

        let binds = config
            .host_config
            .expect("host_config must be set")
            .binds
            .expect("binds must be set");

        assert!(
            binds
                .iter()
                .any(|b| b == "repo2graph-reqs.sphinx:/usr/src/app/reqs:rw"),
            "binds should contain repo2graph-reqs.sphinx:/usr/src/app/reqs:rw, got: {:?}",
            binds
        );
    }

    #[test]
    fn test_hermes_url_is_emitted_only_when_linked() {
        // Deliberately does not take ENV_LOCK: nothing here reads or writes
        // env vars, and the assertions only look at HERMES_URL.
        let img = test_repo2graph_image();
        let neo4j = test_neo4j_image();

        let without = repo2graph(&img, &neo4j, &None, &None, &None, &None).unwrap();
        assert!(
            !without
                .env
                .unwrap()
                .iter()
                .any(|e| e.starts_with("HERMES_URL=")),
            "HERMES_URL should be absent when hermes isn't linked"
        );

        let hermes = HermesImage::new("hermes", "latest", "8645");
        let with = repo2graph(&img, &neo4j, &None, &None, &Some(hermes), &None).unwrap();
        assert!(
            with.env
                .unwrap()
                .contains(&"HERMES_URL=http://hermes.sphinx:8645".to_string()),
            "HERMES_URL should point at the hermes container"
        );
    }

    #[test]
    fn test_redis_url_is_emitted_only_when_linked() {
        // Deliberately does not take ENV_LOCK: nothing here reads or writes
        // env vars, and the assertions only look at REDIS_URL.
        let img = test_repo2graph_image();
        let neo4j = test_neo4j_image();

        let without = repo2graph(&img, &neo4j, &None, &None, &None, &None).unwrap();
        assert!(
            !without
                .env
                .unwrap()
                .iter()
                .any(|e| e.starts_with("REDIS_URL=")),
            "REDIS_URL should be absent when redis isn't linked"
        );

        let redis = RedisImage::new("redis", "latest");
        let with = repo2graph(&img, &neo4j, &None, &None, &None, &Some(redis)).unwrap();
        assert!(
            with.env
                .unwrap()
                .contains(&"REDIS_URL=redis://redis.sphinx:6379".to_string()),
            "REDIS_URL should point at the redis container"
        );
    }

    #[test]
    fn test_sessions_and_artifacts_vols_still_present() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("GITHUB_REQUEST_TOKEN");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");

        let img = test_repo2graph_image();
        let neo4j = test_neo4j_image();
        let config = repo2graph(&img, &neo4j, &None, &None, &None, &None).unwrap();

        let env = config.env.as_ref().unwrap();
        assert!(env.contains(&"SESSIONS_DIR=/usr/src/app/sessions".to_string()));
        assert!(env.contains(&"AGENT_ARTIFACTS_DIR=/usr/src/app/artifacts".to_string()));

        let binds = config
            .host_config
            .expect("host_config must be set")
            .binds
            .expect("binds must be set");

        assert!(
            binds
                .iter()
                .any(|b| b == "repo2graph-sessions.sphinx:/usr/src/app/sessions:rw"),
            "sessions vol missing from binds: {:?}",
            binds
        );
        assert!(
            binds
                .iter()
                .any(|b| b == "repo2graph-artifacts.sphinx:/usr/src/app/artifacts:rw"),
            "artifacts vol missing from binds: {:?}",
            binds
        );
    }
}
