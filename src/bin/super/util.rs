use std::collections::HashMap;

use anyhow::{anyhow, Error};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_ec2::types::{BlockDeviceMapping, EbsBlockDevice, InstanceType, Tag, TagSpecification};
use aws_sdk_ec2::Client;
use aws_smithy_types::retry::RetryConfig;
use futures_util::TryFutureExt;
use reqwest::Response;
use serde_json::Value;
use sphinx_swarm::cmd::{send_cmd_request, Cmd, LoginInfo, SwarmCmd, UpdateNode};
use sphinx_swarm::config::Stack;
use sphinx_swarm::utils::{getenv, make_reqwest_client};

use crate::cmd::{AccessNodesInfo, AddSwarmResponse, LoginResponse, SuperSwarmResponse};
use crate::state::{RemoteStack, Super};

pub fn add_new_swarm_details(
    state: &mut Super,
    swarm_details: RemoteStack,
    must_save_stack: &mut bool,
) -> AddSwarmResponse {
    match state.find_swarm_by_host(&swarm_details.host) {
        Some(_swarm) => {
            return AddSwarmResponse {
                success: false,
                message: "swarm already exist".to_string(),
            };
        }
        None => {
            state.add_remote_stack(swarm_details);
            *must_save_stack = true;
            return AddSwarmResponse {
                success: true,
                message: "Swarm added successfully".to_string(),
            };
        }
    }
}

pub async fn login_to_child_swarm(swarm_details: &RemoteStack) -> Result<String, Error> {
    let client = make_reqwest_client();

    let base_route = get_child_base_route(&swarm_details.host);
    let route = format!("{}/login", base_route);

    if let None = &swarm_details.user {
        return Err(anyhow!("Swarm Username is missing"));
    }

    if let None = &swarm_details.pass {
        return Err(anyhow!("Swarm Password is missing"));
    }

    let body = LoginInfo {
        username: swarm_details.user.clone().unwrap(),
        password: swarm_details.pass.clone().unwrap(),
    };

    return match client.post(route.as_str()).json(&body).send().await {
        Ok(res) => {
            if res.status().clone() != 200 {
                return Err(anyhow!(
                    "{} Status code from login into child swarm",
                    res.status().clone()
                ));
            }
            let login_json: LoginResponse = res.json().await?;

            Ok(login_json.token)
        }
        Err(err) => {
            log::error!("Error trying to login: {:?}", err);
            Err(anyhow!("error trying to login"))
        }
    };
}

pub async fn get_child_swarm_config(
    swarm_details: &RemoteStack,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;
    // let res = handle_get_child_swarm_config(&swarm_details.host, &token).await?;
    let cmd = Cmd::Swarm(SwarmCmd::GetConfig);
    let res = swarm_cmd(cmd, &swarm_details.host, &token).await?;

    if res.status().clone() != 200 {
        return Err(anyhow!(format!(
            "{} status code gotten from get child swarm config",
            res.status()
        )));
    };

    let stack: Stack = res.json().await?;

    let nodes = serde_json::to_value(stack.nodes)?;

    Ok(SuperSwarmResponse {
        success: true,
        message: "child swarm config successfully retrieved".to_string(),
        data: Some(nodes),
    })
}

async fn swarm_cmd(cmd: Cmd, host: &str, token: &str) -> Result<Response, Error> {
    let url = get_child_base_route(host);
    let cmd_res = send_cmd_request(cmd, "SWARM", &url, Some("x-jwt"), Some(&token)).await?;
    Ok(cmd_res)
}

pub fn get_child_base_route(host: &str) -> String {
    return format!("https://app.{}/api", host);

    // return format!("http://{}/api", host);
}

pub async fn get_child_swarm_containers(
    swarm_details: &RemoteStack,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;
    let cmd = Cmd::Swarm(SwarmCmd::ListContainers);
    let res = swarm_cmd(cmd, &swarm_details.host, &token).await?;

    if res.status().clone() != 200 {
        return Err(anyhow!(format!(
            "{} status code gotten from get child swarm container",
            res.status()
        )));
    }

    let containers: Value = res.json().await?;

    Ok(SuperSwarmResponse {
        success: true,
        message: "child swarm containers successfully retrieved".to_string(),
        data: Some(containers),
    })
}

pub async fn access_child_swarm_containers(
    swarm_details: &RemoteStack,
    nodes: Vec<String>,
    cmd_text: &str,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;
    let mut errors: HashMap<String, String> = HashMap::new();

    for node in nodes {
        let cmd: Cmd;
        if cmd_text == "UpdateNode" {
            cmd = Cmd::Swarm(SwarmCmd::UpdateNode(UpdateNode {
                id: node.clone(),
                version: "latest".to_string(),
            }));
        } else if cmd_text == "StartContainer" {
            cmd = Cmd::Swarm(SwarmCmd::StartContainer(node.clone()))
        } else {
            cmd = Cmd::Swarm(SwarmCmd::StopContainer(node.clone()))
        }

        match swarm_cmd(cmd, &swarm_details.host, &token).await {
            Ok(res) => {
                if res.status().clone() != 200 {
                    errors.insert(
                        node.clone(),
                        format!(
                            "{} status error trying to {} {}",
                            res.status(),
                            &cmd_text,
                            node.clone()
                        ),
                    );
                }
            }
            Err(err) => {
                log::error!("Error trying to {}: {}", &cmd_text, &err);
                errors.insert(node, err.to_string());
            }
        }
    }

    if !errors.is_empty() {
        match serde_json::to_value(errors) {
            Ok(error_map) => {
                return Ok(SuperSwarmResponse {
                    success: false,
                    message: format!("Error occured trying to {}", cmd_text),
                    data: Some(error_map),
                });
            }
            Err(err) => {
                return Err(anyhow!("Error parsing error: {}", err.to_string()));
            }
        };
    }
    Ok(SuperSwarmResponse {
        success: true,
        message: format!("{} executed successfully", cmd_text),
        data: None,
    })
}

pub async fn accessing_child_container_controller(
    state: &Super,
    info: AccessNodesInfo,
    cmd: &str,
) -> SuperSwarmResponse {
    let res: SuperSwarmResponse;
    match state.find_swarm_by_host(&info.host) {
        Some(swarm) => match access_child_swarm_containers(&swarm, info.nodes, cmd).await {
            Ok(result) => res = result,
            Err(err) => {
                res = SuperSwarmResponse {
                    success: false,
                    message: err.to_string(),
                    data: None,
                }
            }
        },
        None => {
            res = SuperSwarmResponse {
                success: false,
                message: "Swarm does not exist".to_string(),
                data: None,
            }
        }
    }
    res
}

async fn create_ec2_instance() -> Result<String, Error> {
    let region = getenv("AWS_S3_REGION_NAME")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

    let aws_access_key_id = getenv("AWS_ACCESS_KEY_ID")?;

    let aws_access_token = getenv("AWS_SECRET_ACCESS_KEY")?;

    let stakwork_token = getenv("STAKWORK_ADD_NODE_TOKEN")?;

    let lnd_macaroon = getenv("EXTERNAL_LND_MACAROON")?;

    let lnd_address = getenv("EXTERNAL_LND_ADDRESS")?;

    let lnd_cert = getenv("EXTERNAL_LND_CERT")?;

    let youtube_token = getenv("YOUTUBE_API_TOKEN")?;

    let twitter_token = getenv("TWITTER_BEARER")?;

    let super_url = getenv("SUPER_URL")?;

    let super_token = getenv("SUPER_TOKEN")?;

    let swarm_name = "swarm46";

    // Load the AWS configuration
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    let client = Client::new(&config);

    let user_data_script = format!(
        r#"#!/bin/bash
        cd /home/admin &&
        pwd &&
        echo "INSTALLING DEPENDENCIES..." && \
        curl -fsSL https://get.docker.com/ -o get-docker.sh && \
        sh get-docker.sh && \
        sudo usermod -aG docker $(whoami) && \
        sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) -o /usr/local/bin/docker-compose && \
        sudo chmod +x /usr/local/bin/docker-compose && \
        docker-compose version && \
        sudo apt update && \
        sudo apt install git && \
        sleep 10 && \
        pwd && \
        git clone https://github.com/stakwork/sphinx-swarm.git && \
        cd sphinx-swarm && \
        docker network create sphinx-swarm && \
        touch .env && \

        echo "HOST={}.sphinx.chat" >> .env && \
    echo 'NETWORK=bitcoin' >> .env && \
    echo 'AWS_ACCESS_KEY_ID={}' >> .env && \
    echo 'AWS_SECRET_ACCESS_KEY={}' >> .env && \
    echo 'AWS_REGION=us-east-1a' >> .env && \
    echo 'AWS_S3_REGION_NAME=us-east-1' >> .env && \
    echo 'STAKWORK_ADD_NODE_TOKEN={}' >> .env && \
    echo 'STAKWORK_RADAR_REQUEST_TOKEN={}' >> .env && \
    echo 'NO_REMOTE_SIGNER=true' >> .env && \
    echo 'EXTERNAL_LND_MACAROON={}' >> .env && \
    echo 'EXTERNAL_LND_ADDRESS={}' >> .env && \
    echo 'EXTERNAL_LND_CERT={}' >> .env && \
    echo 'YOUTUBE_API_TOKEN={}' >> .env && \
    echo 'SWARM_UPDATER_PASSWORD=-' >> .env && \
    echo 'JARVIS_FEATURE_FLAG_SCHEMA=true' >> .env && \
    echo 'BACKUP_KEY=' >> .env && \
    echo 'FEATURE_FLAG_TEXT_EMBEDDINGS=true' >> .env && \
    echo 'TWITTER_BEARER={}' >> .env && \
    echo 'SUPER_TOKEN={}' >> .env && \
    echo 'SUPER_URL={}' >> .env && \

    sleep 30 && \
    ./restart-second-brain.sh
        "#,
        swarm_name,
        aws_access_key_id,
        aws_access_token,
        stakwork_token,
        stakwork_token,
        lnd_macaroon,
        lnd_address,
        lnd_cert,
        youtube_token,
        twitter_token,
        super_token,
        super_url
    );

    let tag = Tag::builder()
        .key("Name")
        .value(swarm_name) // Replace with the desired instance name
        .build();

    // Define the TagSpecification to apply the tags when the instance is created
    let tag_specification = TagSpecification::builder()
        .resource_type("instance".into()) // Tag the instance
        .tags(tag)
        .build();

    let block_device = BlockDeviceMapping::builder()
        .device_name("/dev/xvda") // Valid for Debian
        .ebs(EbsBlockDevice::builder().volume_size(100).build())
        .build();

    let result = client
        .run_instances()
        .image_id("ami-064519b8c76274859")
        .instance_type(InstanceType::T3Medium)
        .security_group_ids("sg-0968c683977f8323e")
        .key_name("sphinx-instances".to_string())
        .min_count(1)
        .max_count(1)
        .user_data(base64::encode(user_data_script))
        .block_device_mappings(block_device)
        .tag_specifications(tag_specification)
        .send()
        .await?;

    log::info!("Result from creating instance is back");

    if result.instances().is_empty() {
        return Err(anyhow!("Failed to create instance"));
    }

    let instance_id: String = result.instances()[0].instance_id().unwrap().to_string();
    println!("Created instance with ID: {}", instance_id);

    Ok(instance_id)
}

async fn get_instance_ip(instance_id: &str) -> Result<String, Error> {
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let result = client
        .describe_instances()
        .instance_ids(instance_id)
        .send()
        .map_err(|err| anyhow!(err.to_string()))
        .await?;

    if result.reservations().is_empty() {
        return Err(anyhow!("Failed to create instance"));
    }

    log::info!("Result from IP: {:?}", result.reservations()[0].instances());

    // Ok(public_ip.to_string())
    Ok("".to_string())
}

pub async fn create_swarm_ec2() -> Result<(), Error> {
    log::info!("About to get into the creating ec2 instance");
    let ec2_intance_id = create_ec2_instance().await?;
    let ec2_ip_address = get_instance_ip(&ec2_intance_id).await?;
    log::info!("{}", ec2_ip_address);
    Ok(())
}
