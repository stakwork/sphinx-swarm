use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Error};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_ec2::types::{BlockDeviceMapping, EbsBlockDevice, InstanceType, Tag, TagSpecification};
use aws_sdk_ec2::Client;
use aws_sdk_route53::types::{
    Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet,
};
use aws_sdk_route53::Client as Route53Client;
use aws_smithy_types::retry::RetryConfig;
use futures_util::TryFutureExt;
use reqwest::Response;
use serde_json::Value;
use sphinx_swarm::cmd::{send_cmd_request, Cmd, LoginInfo, SwarmCmd, UpdateNode};
use sphinx_swarm::config::Stack;
use sphinx_swarm::utils::{getenv, make_reqwest_client};

use crate::cmd::{
    AccessNodesInfo, AddSwarmResponse, CreateEc2InstanceInfo, LoginResponse, SuperSwarmResponse,
};
use crate::state::{AwsInstanceType, RemoteStack, Super};
use rand::Rng;
use tokio::time::{sleep, Duration};

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

    let base_route = get_child_base_route(swarm_details.default_host.clone())?;
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
    let res = swarm_cmd(cmd, swarm_details.default_host.clone(), &token).await?;

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

async fn swarm_cmd(cmd: Cmd, host: Option<String>, token: &str) -> Result<Response, Error> {
    let url = get_child_base_route(host)?;
    let cmd_res = send_cmd_request(cmd, "SWARM", &url, Some("x-jwt"), Some(&token)).await?;
    Ok(cmd_res)
}

pub fn get_child_base_route(host: Option<String>) -> Result<String, Error> {
    if host.is_none() {
        return Err(anyhow!("child swarm default host not provided"));
    };

    return Ok(format!("https://app.{}/api", host.unwrap()));

    // return Ok(format!("http://{}/api", host.unwrap()));
}

pub async fn get_child_swarm_containers(
    swarm_details: &RemoteStack,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;
    let cmd = Cmd::Swarm(SwarmCmd::ListContainers);
    let res = swarm_cmd(cmd, swarm_details.default_host.clone(), &token).await?;

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
        } else if cmd_text == "RestartContainer" {
            cmd = Cmd::Swarm(SwarmCmd::RestartContainer(node.clone()))
        } else {
            cmd = Cmd::Swarm(SwarmCmd::StopContainer(node.clone()))
        }

        match swarm_cmd(cmd, swarm_details.default_host.clone(), &token).await {
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

pub fn get_aws_instance_types() -> SuperSwarmResponse {
    let instance_types: Vec<AwsInstanceType> = vec![
        AwsInstanceType {
            name: "Large".to_string(),
            value: "m5.large".to_string(),
        },
        AwsInstanceType {
            name: "Extra Large".to_string(),
            value: "m5.xlarge".to_string(),
        },
        AwsInstanceType {
            name: "Extra Large GPU".to_string(),
            value: "g4dn.2xlarge".to_string(),
        },
    ];

    match serde_json::to_value(instance_types) {
        Ok(instance_value) => SuperSwarmResponse {
            success: true,
            message: "Aws Instance types loaded successfully".to_string(),
            data: Some(instance_value),
        },
        Err(err) => SuperSwarmResponse {
            success: false,
            message: err.to_string(),
            data: None,
        },
    }
}

async fn create_ec2_instance(
    swarm_name: String,
    vanity_address: Option<String>,
    instance_type_name: String,
) -> Result<(String, i32), Error> {
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

    let swarm_name = format!("{}", swarm_name);

    let swarm_number = rand::thread_rng().gen_range(100000..1000000);

    let device_name = getenv("AWS_DEVICE_NAME")?;

    let image_id = getenv("AWS_IMAGE_ID")?;

    let security_group_id = getenv("AWS_SECURITY_GROUP_ID")?;

    let subnet_id = getenv("AWS_SUBNET_ID")?;

    let key_name = getenv("AWS_KEY_NAME")?;

    let swarm_updater_password = getenv("SWARM_UPDATER_PASSWORD")?;

    let custom_domain = vanity_address.unwrap_or_else(|| String::from(""));

    // Load the AWS configuration
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    let client = Client::new(&config);

    let user_data_script = format!(
        r#"#!/bin/bash
      su - admin -c '
          cd /home/admin &&
          pwd &&
          echo "INSTALLING DEPENDENCIES..." && \
          curl -fsSL https://get.docker.com/ -o get-docker.sh && \
          sh get-docker.sh && \
          sudo usermod -aG docker $USER && \
          sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) -o /usr/local/bin/docker-compose && \
          sudo chmod +x /usr/local/bin/docker-compose && \
          docker-compose version && \
          sudo apt update && \
          sudo apt install -y git && \
          
          # Create Docker network
          echo "Creating Docker network..." && \
          newgrp docker <<EOF
  docker network create sphinx-swarm
  EOF
          
          sleep 10 && \
          pwd && \
          cd /home/admin && \
          git clone https://github.com/stakwork/sphinx-swarm.git && \
          cd sphinx-swarm && \
          pwd && \
          touch .env && \
          
          # Populate the .env file
          echo "HOST=swarm{swarm_number}.sphinx.chat" >> .env && \
          echo "NETWORK=bitcoin" >> .env && \
          echo "AWS_ACCESS_KEY_ID={aws_access_key_id}" >> .env && \
          echo "AWS_SECRET_ACCESS_KEY={aws_access_token}" >> .env && \
          echo "AWS_REGION=us-east-1a" >> .env && \
          echo "AWS_S3_REGION_NAME=us-east-1" >> .env && \
          echo "STAKWORK_ADD_NODE_TOKEN={stakwork_token}" >> .env && \
          echo "STAKWORK_RADAR_REQUEST_TOKEN={stakwork_token}" >> .env && \
          echo "NO_REMOTE_SIGNER=true" >> .env && \
          echo "EXTERNAL_LND_MACAROON={lnd_macaroon}" >> .env && \
          echo "EXTERNAL_LND_ADDRESS={lnd_address}" >> .env && \
          echo "EXTERNAL_LND_CERT={lnd_cert}" >> .env && \
          echo "YOUTUBE_API_TOKEN={youtube_token}" >> .env && \
          echo "SWARM_UPDATER_PASSWORD={swarm_updater_password}" >> .env && \
          echo "JARVIS_FEATURE_FLAG_SCHEMA=true" >> .env && \
          echo "BACKUP_KEY=" >> .env && \
          echo "FEATURE_FLAG_TEXT_EMBEDDINGS=true" >> .env && \
          echo "TWITTER_BEARER={twitter_token}" >> .env && \
          echo "SUPER_TOKEN={super_token}" >> .env && \
          echo "SUPER_URL={super_url}" >> .env && \
          echo "NAV_BOLTWALL_SHARED_HOST={custom_domain}" >> .env && \
          echo "SECOND_BRAIN_ONLY=true" >> .env && \
          
          sleep 60 && \
          
          echo "Setting up Restarter server..." && \
          sudo apt install -y nodejs npm && \
          sudo npm install pm2 -g
        '
          
          echo 'module.exports = {{
            apps: [
              {{
                name: "restarter",
                script: "./restarter.js",
                env: {{
                  SECOND_BRAIN: "true",
                  PASSWORD: "{swarm_updater_password}",
                }},
              }},
            ],
          }};' > /home/admin/sphinx-swarm/ecosystem.config.js
          
        su - admin -c '
        cd /home/admin && \
        wget https://s3.amazonaws.com/ec2-downloads-windows/SSMAgent/latest/debian_amd64/amazon-ssm-agent.deb && \
        sudo dpkg -i amazon-ssm-agent.deb
          cd /home/admin/sphinx-swarm && \
          pm2 start ecosystem.config.js && \
          ./restart-second-brain.sh
      '
      "#
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
        .device_name(device_name) // Valid for Debian
        .ebs(EbsBlockDevice::builder().volume_size(100).build())
        .build();

    let instance_type = InstanceType::from_str(&instance_type_name).map_err(|err| {
        log::error!("Invalid instance type: {}", err);
        anyhow!(err.to_string())
    })?;

    let result = client
        .run_instances()
        .image_id(image_id)
        .instance_type(instance_type)
        .security_group_ids(security_group_id)
        .key_name(key_name)
        .min_count(1)
        .max_count(1)
        .user_data(base64::encode(user_data_script))
        .block_device_mappings(block_device)
        .tag_specifications(tag_specification)
        .subnet_id(subnet_id)
        .send()
        .map_err(|err| {
            log::error!("Error Creating instance instance: {}", err);
            anyhow!(err.to_string())
        })
        .await?;

    if result.instances().is_empty() {
        return Err(anyhow!("Failed to create instance"));
    }

    let instance_id: String = result.instances()[0].instance_id().unwrap().to_string();
    println!("Created instance with ID: {}", instance_id);

    Ok((instance_id, swarm_number))
}

async fn get_instance_ip(instance_id: &str) -> Result<String, Error> {
    let region = getenv("AWS_S3_REGION_NAME")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;

    let client = Client::new(&config);

    let result = client
        .describe_instances()
        .instance_ids(instance_id)
        .send()
        .map_err(|err| {
            log::error!("Error describing instance: {}", err);
            anyhow!(err.to_string())
        })
        .await?;

    if result.reservations().is_empty() {
        return Err(anyhow!("Failed to create instance"));
    }

    if result.reservations()[0].instances().is_empty() {
        return Err(anyhow!("Could not get ec2 instance"));
    }

    if result.reservations()[0].instances()[0]
        .public_ip_address()
        .is_none()
    {
        return Err(anyhow!("No public ip address for the new instance"));
    }

    let public_ip_address = result.reservations()[0].instances()[0]
        .public_ip_address()
        .unwrap();

    log::info!("Instance Public IP Address: {}", public_ip_address);

    Ok(public_ip_address.to_string())
}

async fn add_domain_name_to_route53(domain_name: &str, public_ip: &str) -> Result<(), Error> {
    let region = getenv("AWS_S3_REGION_NAME")?;
    let hosted_zone_id = getenv("ROUTE53_ZONE_ID")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;
    let route53_client = Route53Client::new(&config);

    let resource_record = ResourceRecord::builder().value(public_ip).build()?;

    let resource_record_set = ResourceRecordSet::builder()
        .name(domain_name)
        .r#type("A".into()) // A record for IPv4
        .ttl(300) // Time-to-live (in seconds)
        .resource_records(resource_record)
        .build()
        .map_err(|err| anyhow!(err.to_string()))?;

    // Create a change request to upsert (create or update) the A record
    let change = Change::builder()
        .action(ChangeAction::Upsert)
        .resource_record_set(resource_record_set)
        .build()
        .map_err(|err| anyhow!(err.to_string()))?;

    let change_batch = ChangeBatch::builder()
        .changes(change)
        .build()
        .map_err(|err| anyhow!(err.to_string()))?;

    let response = route53_client
        .change_resource_record_sets()
        .hosted_zone_id(hosted_zone_id)
        .change_batch(change_batch)
        .send()
        .await?;

    log::info!(
        "Route 53 change status for {}: {:?}",
        domain_name,
        response.change_info()
    );

    Ok(())
}

//Sample execution function
pub async fn create_swarm_ec2(info: &CreateEc2InstanceInfo) -> Result<(), Error> {
    if let Some(vanity_address) = &info.vanity_address {
        if !vanity_address.is_empty() {
            if let Some(subdomain) = vanity_address.strip_suffix(".sphinx.chat") {
                let domain_status = is_valid_domain(subdomain.to_string());
                if !domain_status.is_empty() {
                    return Err(anyhow!(domain_status));
                }
            } else {
                return Err(anyhow!("Vanity Address doesn't match the expected format."));
            }
        }
    }
    let ec2_intance_id = create_ec2_instance(
        info.name.clone(),
        info.vanity_address.clone(),
        info.instance_type.clone(),
    )
    .await?;

    sleep(Duration::from_secs(40)).await;

    let ec2_ip_address = get_instance_ip(&ec2_intance_id.0).await?;
    let _ = add_domain_name_to_route53(
        &format!("*.swarm{}.sphinx.chat", &ec2_intance_id.1),
        &ec2_ip_address,
    )
    .await?;
    if let Some(custom_domain) = &info.vanity_address {
        log::info!("vanity address is being set");
        let _custom_domain_result =
            add_domain_name_to_route53(custom_domain, &ec2_ip_address).await?;
    }
    log::info!("Public_IP: {}", ec2_ip_address);
    Ok(())
}

fn is_valid_domain(domain: String) -> String {
    let valid_chars = |c: char| c.is_ascii_alphanumeric() || c == '-';

    if domain.starts_with('-') || domain.ends_with('-') {
        return "Hyphen cannot be the first or last character.".to_string();
    }

    let mut previous_char: Option<char> = None;
    for c in domain.chars() {
        if !valid_chars(c) {
            return "Domain can only contain letters, numbers, and hyphens.".to_string();
        }

        if let Some(prev) = previous_char {
            if prev == '-' && c == '-' {
                return "Hyphens cannot appear consecutively.".to_string();
            }
        }

        previous_char = Some(c);
    }

    "".to_string()
}
