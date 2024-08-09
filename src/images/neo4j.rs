// use super::traefik::{neo4j_labels, traefik_labels};
use super::*;
use crate::config::Node;
use crate::dock::upload_to_container;
use crate::dock::exec;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Neo4jImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub bolt_port: String,
    pub links: Links,
    pub host: Option<String>,
    pub mem_limit: Option<i64>,
}

impl Neo4jImage {
    pub fn new(name: &str, version: &str) -> Self {
        // ports are hardcoded
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "7474".to_string(),
            // bolt_port: "7687".to_string(),
            bolt_port: "7687".to_string(),
            links: vec![],
            host: None,
            mem_limit: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("neo4j.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub async fn pre_startup(&self, docker: &Docker) -> Result<()> {
        let apoc_url = "https://github.com/neo4j-contrib/neo4j-apoc-procedures/releases/download/5.19.0/apoc-5.19.0-extended.jar";
        log::info!("=> download apoc plugin for neo4j...");
        let bytes = reqwest::get(apoc_url).await?.bytes().await?;
        upload_to_container(
            docker,
            &self.name,
            "/var/lib/neo4j/plugins",
            "apoc-4.4.0.11-all.jar",
            &bytes,
        )
        .await?;
        log::info!("=> copy apoc.conf into container...");
        upload_to_container(
            docker,
            &self.name,
            "/var/lib/neo4j/conf",
            "apoc.conf",
            APOC_CONF.as_bytes(),
        )
        .await?;

        log::info!("=> download graph-data-science plugin for neo4j...");
        let graph_data_science = "https://github.com/neo4j/graph-data-science/releases/download/2.6.8/neo4j-graph-data-science-2.6.8.jar";
        let graph_data_science_bytes = reqwest::get(graph_data_science).await?.bytes().await?;
        upload_to_container(
            docker,
            &self.name,
            "/var/lib/neo4j/plugins",
            "neo4j-graph-data-science-2.6.8.jar",
            &graph_data_science_bytes,
        )
        .await?;

        Ok(())
    }
    pub async fn post_startup(&self, _proj: &str, docker: &Docker) -> Result<()> {
        //Migrate Neo4j to 5.19.0
        log::info!("=> Checking neo4j version and migrating to 5.19.0...");
        //let command = "/var/lib/neo4j/bin/neo4j-admin --version";
        let command = "/bin/echo 4.0.0";
        log::info!("=> getting neo4j version...");
        let neo4j_admin_version = exec(docker, &domain(&self.name), command).await?;

        log::info!("=> got neo4j version...");

        let version_output = String::from_utf8_lossy(&neo4j_admin_version.as_bytes()).trim().to_string();
        log::info!("=> got neo4j version string...");
        let required_version = "5.0.0".to_string();
        log::info!("{}", version_output);
        log::info!("{}", required_version);
        log::info!("{}", version_output >= required_version);
        if version_output <= required_version {
            log::info!("=> migrating to 5.19.0...");

            let command = "/var/lib/neo4j/bin/neo4j-admin database migrate system --force-btree-indexes-to-range";
            exec(docker, &domain(&self.name), command).await?;

            let command = "/var/lib/neo4j/bin/neo4j-admin database migrate neo4j --force-btree-indexes-to-range";
            exec(docker, &domain(&self.name), command).await?;
            log::info!("=> finished migrating to 5.19.0...");
        }
        Ok(())
    }

}

#[async_trait]
impl DockerConfig for Neo4jImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(neo4j(self))
    }
}

impl DockerHubImage for Neo4jImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "library".to_string(),
            repo: "neo4j".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn neo4j(node: &Neo4jImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}", repo.repo);
    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone(), node.bolt_port.clone()];

    let c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None, node.mem_limit),
        env: Some(vec![
            // format!("NEO4J_URI=neo4j://neo4j:{}", &node.bolt_port),
            format!("NEO4J_AUTH=neo4j/test"),
            format!("NEO4J_apoc_export_file_enabled=true"),
            format!("NEO4J_apoc_import_file_enabled=true"),
            format!("NEO4J_dbms_security_procedures_unrestricted=apoc.*,algo.*"),
            format!("NEO4J_server_memory_heap_initial__size=64m"),
            format!("NEO4J_server_memory_heap_max__size=512m"),
            format!("NEO4J_apoc_uuid_enabled=true"),
            format!("NEO4J_server_default__listen__address=0.0.0.0"),
            format!(
                "NEO4J_server_bolt_listen__address=0.0.0.0:{}",
                &node.bolt_port
            ),
            format!("NEO4J_initial_dbms_default__database=neo4j"),
            format!("NEO4J_dbms_security_auth__minimum__password__length=4"),
        ]),
        ..Default::default()
    };
    if let Some(_host) = node.host.clone() {
        // production tls extra domain
        // c.labels = Some(traefik_labels(&node.name, &host, &node.http_port, true));
        // c.labels = Some(neo4j_labels(
        //     &node.name,
        //     &host,
        //     &node.http_port,
        //     &node.bolt_port,
        // ));
    }
    c
}

const APOC_CONF: &str = r#"
apoc.import.file.use_neo4j_config=true
apoc.import.file.enabled=true
apoc.export.file.enabled=true
apoc.initializer.neo4j.1=CREATE FULLTEXT INDEX titles_full_index IF NOT EXISTS FOR (n:Content) ON EACH [n.show_title, n.episode_title]
apoc.initializer.neo4j.2=CREATE FULLTEXT INDEX person_full_index IF NOT EXISTS FOR (n:Person) ON EACH [n.name]
apoc.initializer.neo4j.3=CREATE FULLTEXT INDEX topic_full_index IF NOT EXISTS FOR (n:Topic) ON EACH [n.topic]
apoc.initializer.neo4j.4=CREATE FULLTEXT INDEX text_full_index IF NOT EXISTS FOR (n:Content) ON EACH [n.text]
apoc.initializer.neo4j.5=CREATE FULLTEXT INDEX data_bank_full_index IF NOT EXISTS FOR (n:Data_Bank) ON EACH [n.Data_Bank]
apoc.initializer.neo4j.6=MATCH (n) WHERE NOT EXISTS(n.namespace) OR n.namespace = '' SET n.namespace = 'default'
"#;
