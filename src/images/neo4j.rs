// use super::traefik::{neo4j_labels, traefik_labels};
use super::*;
use crate::config::Node;
use crate::dock::upload_to_container;
use crate::secrets;
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
    pub password: String,
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
            password: secrets::random_word(32),
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
        if *self.version <= *"4.4.9" {
            let apoc_version_4 = "https://github.com/neo4j-contrib/neo4j-apoc-procedures/releases/download/4.4.0.11/apoc-4.4.0.11-all.jar";
            log::info!("=> download apoc version 4 plugin for neo4j...");
            let bytes = reqwest::get(apoc_version_4).await?.bytes().await?;
            upload_to_container(
                docker,
                &self.name,
                "/var/lib/neo4j/plugins",
                "apoc-4.4.0.11-all.jar",
                &bytes,
            )
            .await?;
        } else {
            let apoc_extended_url = "https://github.com/neo4j-contrib/neo4j-apoc-procedures/releases/download/5.19.0/apoc-5.19.0-extended.jar";
            log::info!("=> download apoc-extended plugin for neo4j...");
            let bytes = reqwest::get(apoc_extended_url).await?.bytes().await?;
            upload_to_container(
                docker,
                &self.name,
                "/var/lib/neo4j/plugins",
                "apoc-5.19.0-extended.jar",
                &bytes,
            )
            .await?;

            let apoc_url =
                "https://github.com/neo4j/apoc/releases/download/5.19.0/apoc-5.19.0-core.jar";
            log::info!("=> download apoc plugin for neo4j...");
            let bytes = reqwest::get(apoc_url).await?.bytes().await?;
            upload_to_container(
                docker,
                &self.name,
                "/var/lib/neo4j/plugins",
                "apoc-5.19.0-core.jar",
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
        }

        log::info!("=> download graph-data-science plugin for neo4j...");
        let graph_data_science = "https://github.com/neo4j/graph-data-science/releases/download/2.9.0/neo4j-graph-data-science-2.9.0.zip";
        let graph_data_science_bytes = reqwest::get(graph_data_science).await?.bytes().await?;
        upload_to_container(
            docker,
            &self.name,
            "/var/lib/neo4j/plugins",
            "neo4j-graph-data-science-2.9.0.jar",
            &graph_data_science_bytes,
        )
        .await?;

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
            registry: Registry::DockerHub,
            org: "library".to_string(),
            repo: "neo4j".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn neo4j(node: &Neo4jImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = node.image();
    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone(), node.bolt_port.clone()];

    let mut server_memory_heap_initial_size = "NEO4J_dbms_memory_heap_initial__size";
    let mut dbms_memory_heap_max_size = "NEO4J_dbms_memory_heap_max__size";
    let mut dbms_default_listen_address = "NEO4J_dbms_default__listen__address";
    let mut dbms_connector_bolt_listen_address = "NEO4J_dbms_connector_bolt_listen__address";
    let dbms_allow_upgrade = "NEO4J_dbms_allow__upgrade=true";
    let mut dbms_default_database = "NEO4J_dbms_default__database=neo4j";
    let mut dbms_security_procedures_unrestricted =
        "NEO4J_dbms_security_procedures_unrestricted=apoc.*,algo.*,gds.*";
    let mut dbms_security_procedures_whitelist = "NEO4J_dbms_security_procedures_whitelist=apoc.*";
    let mut dbms_security_auth_minimum_password_length =
        "NEO4J_dbms_security_auth__minimum__password__length=4";
    let mut dbms_memory_pagecache_size = "NEO4J_dbms_memory_pagecache_size";
    if *node.version > *"4.4.9" {
        server_memory_heap_initial_size = "NEO4J_server_memory_heap_initial__size";
        dbms_memory_heap_max_size = "NEO4J_server_memory_heap_max__size";
        dbms_default_listen_address = "NEO4J_server_default__listen__address";
        dbms_connector_bolt_listen_address = "NEO4J_server_bolt_listen__address";
        dbms_default_database = "NEO4J_initial_dbms_default__database=neo4j";
        dbms_security_auth_minimum_password_length =
            "NEO4J_dbms_security_auth__minimum__password__length=4";
        dbms_memory_pagecache_size = "NEO4J_server_memory_pagecache_size"
    }

    let c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None, node.mem_limit),
        env: Some(vec![
            // format!("NEO4J_URI=neo4j://neo4j:{}", &node.bolt_port),
            format!("NEO4J_AUTH=neo4j/{}", node.password),
            format!("NEO4J_apoc_export_file_enabled=true"),
            format!("NEO4J_apoc_import_file_enabled=true"),
            format!("{}=1g", server_memory_heap_initial_size),
            format!("{}=4g", dbms_memory_heap_max_size),
            format!("{}=1g", dbms_memory_pagecache_size),
            format!("NEO4J_apoc_uuid_enabled=true"),
            format!("{}=0.0.0.0", dbms_default_listen_address),
            format!(
                "{}=0.0.0.0:{}",
                dbms_connector_bolt_listen_address, &node.bolt_port
            ),
            format!("NEO4J_dbms.security.procedures.allowlist=gds.*"),
            format!("{}", dbms_allow_upgrade),
            format!("{}", dbms_default_database),
            format!("NEO4J_dbms_security_auth__minimum__password__length=4"),
            format!(
                "{}",
                "NEO4J_dbms_security_procedures_unrestricted=apoc.*,algo.*,gds.*"
            ),
            format!(
                "{}",
                "NEO4J_dbms_security_procedures_whitelist=apoc.*,gds.*,algo.*"
            ),
            format!("NEO4J_PLUGINS=[\"graph-data-science\"]"),
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
apoc.initializer.neo4j.3=CREATE FULLTEXT INDEX topic_full_index IF NOT EXISTS FOR (n:Topic) ON EACH [n.name]
apoc.initializer.neo4j.4=CREATE FULLTEXT INDEX text_full_index IF NOT EXISTS FOR (n:Content) ON EACH [n.namespace]
apoc.initializer.neo4j.5=CREATE FULLTEXT INDEX data_bank_full_index IF NOT EXISTS FOR (n:Data_Bank) ON EACH [n.Data_Bank] OPTIONS { indexConfig: { `fulltext.analyzer`: 'english' }}
apoc.initializer.neo4j.6=CREATE FULLTEXT INDEX aliasEntityIndex IF NOT EXISTS FOR (n:Alias) ON EACH [n.entity]
apoc.initializer.neo4j.7=CREATE TEXT INDEX entity_lower_string_exact_index IF NOT EXISTS FOR (a:Alias) ON (a.entity_lower)
apoc.initializer.neo4j.8=CREATE TEXT INDEX name_lower_string_exact_index IF NOT EXISTS FOR (t:Topic) ON (t.name_lower)
apoc.initializer.neo4j.9=CREATE INDEX match_entity_namespace_alias_index IF NOT EXISTS FOR (a:Alias) ON (a.entity, a.namespace)
apoc.initializer.neo4j.10=CREATE INDEX match_all_alias_index IF NOT EXISTS FOR (a:Alias) ON (a.entity, a.namespace, a.replacement, a.context)
apoc.initializer.neo4j.11=CREATE INDEX ON :Entity(entity)
apoc.initializer.neo4j.13=CREATE INDEX ON :Topic(name)
apoc.initializer.neo4j.15=CREATE INDEX ON :Replacement_Entity(replacement)
apoc.initializer.neo4j.16=CREATE FULLTEXT INDEX schema_full_index IF NOT EXISTS FOR (n:Schema) ON EACH [n.type]
apoc.initializer.neo4j.17=CREATE FULLTEXT INDEX query_full_index IF NOT EXISTS FOR (n:Query) ON EACH [n.query]
apoc.initializer.neo4j.20=CREATE VECTOR INDEX query_text_embeddings_vector_index IF NOT EXISTS FOR (n:Query) ON n.query_embeddings OPTIONS { indexConfig: { `vector.dimensions`: 384, `vector.similarity_function`: 'cosine' }}
"#;
