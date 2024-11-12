use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Error};
use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_ec2::client::Waiters;
use aws_sdk_ec2::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_ec2::types::{
    AttributeBooleanValue, AttributeValue, BlockDeviceMapping, EbsBlockDevice, HttpTokensState,
    InstanceMetadataOptionsRequest, InstanceType, Tag, TagSpecification,
};
use aws_sdk_ec2::Client;
use aws_smithy_types::retry::RetryConfig;
use chrono::Local;
use reqwest::Response;
use serde_json::Value;
use sphinx_swarm::cmd::{send_cmd_request, Cmd, LoginInfo, SwarmCmd, UpdateNode};
use sphinx_swarm::config::Stack;
use sphinx_swarm::utils::{getenv, make_reqwest_client};

use crate::aws_util::make_aws_client;
use crate::cmd::{
    AccessNodesInfo, AddSwarmResponse, CreateEc2InstanceInfo, GetInstanceTypeByInstanceId,
    GetInstanceTypeRes, LoginResponse, SuperSwarmResponse, UpdateInstanceDetails,
};
use crate::ec2::get_swarms_by_tag;
use crate::route53::add_domain_name_to_route53;
use crate::state::{AwsInstanceType, InstanceFromAws, RemoteStack, Super};
use aws_config::timeout::TimeoutConfig;
use aws_sdk_ec2::types::IamInstanceProfileSpecification;
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

pub fn add_new_swarm_from_child_swarm(
    state: &mut Super,
    swarm_details: RemoteStack,
    must_save_stack: &mut bool,
) -> AddSwarmResponse {
    match state
        .stacks
        .iter()
        .position(|swarm| swarm.default_host == swarm_details.default_host)
    {
        Some(swarm_pos) => {
            if let Some(password) = &state.stacks[swarm_pos].pass {
                if !password.is_empty() {
                    return AddSwarmResponse {
                        success: false,
                        message: "swarm already exist".to_string(),
                    };
                }
            }

            state.stacks[swarm_pos].host = swarm_details.host;
            state.stacks[swarm_pos].pass = swarm_details.pass;
            state.stacks[swarm_pos].user = swarm_details.user;

            *must_save_stack = true;
            return AddSwarmResponse {
                success: true,
                message: "Swarm added successfully".to_string(),
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

async fn swarm_cmd(cmd: Cmd, host: String, token: &str) -> Result<Response, Error> {
    let url = get_child_base_route(host)?;
    let cmd_res = send_cmd_request(cmd, "SWARM", &url, Some("x-jwt"), Some(&token)).await?;
    Ok(cmd_res)
}

pub fn get_child_base_route(host: String) -> Result<String, Error> {
    if host.is_empty() {
        return Err(anyhow!("child swarm default host not provided"));
    };

    return Ok(format!("https://app.{}/api", host));

    // return Ok(format!("http://{}/api", host));
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

pub async fn get_child_swarm_image_versions(
    swarm_details: &RemoteStack,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;
    let cmd = Cmd::Swarm(SwarmCmd::GetAllImageActualVersion);
    let res = swarm_cmd(cmd, swarm_details.default_host.clone(), &token).await?;

    if res.status().clone() != 200 {
        return Err(anyhow!(format!(
            "{} status code gotten from get child swarm container",
            res.status()
        )));
    }

    let image_version: Value = res.json().await?;

    Ok(SuperSwarmResponse {
        success: true,
        message: "child swarm image versions successfully retrieved".to_string(),
        data: Some(image_version),
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

fn instance_types() -> Vec<AwsInstanceType> {
    return vec![
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
}

pub fn get_aws_instance_types() -> SuperSwarmResponse {
    let instance_types = instance_types();

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

pub fn get_descriptive_instance_type(instance_value: Option<String>) -> String {
    if let None = &instance_value {
        return "".to_string();
    }

    let instance_types = instance_types();

    match instance_types
        .iter()
        .position(|instance| instance.value == instance_value.clone().unwrap())
    {
        Some(instance_pos) => {
            let instance = &instance_types[instance_pos];
            format!("{} ({})", instance.name, instance.value)
        }
        None => "".to_string(),
    }
}

async fn create_ec2_instance(
    swarm_name: String,
    vanity_address: Option<String>,
    instance_type_name: String,
) -> Result<(String, i32), Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));

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

    let aws_role = getenv("AWS_USER_ROLE")?;

    let swarm_updater_password = getenv("SWARM_UPDATER_PASSWORD")?;

    let aws_s3_bucket_name = getenv("AWS_S3_BUCKET_NAME")?;

    let custom_domain = vanity_address.unwrap_or_else(|| String::from(""));

    let key = getenv("SWARM_TAG_KEY")?;

    let value = getenv("SWARM_TAG_VALUE")?;

    let timeout_config = TimeoutConfig::builder()
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(60))
        .build();

    // Load the AWS configuration
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .timeout_config(timeout_config)
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
          echo "AWS_REGION=us-east-1" >> .env && \
          echo "AWS_S3_BUCKET_NAME={aws_s3_bucket_name}" >> .env && \
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
          pm2 save && \
          startup_command=$(pm2 startup | grep "sudo" | tail -n 1) && \
          eval $startup_command && \
          pm2 save && \
          ./restart-second-brain.sh
      '
      "#
    );

    let tags = vec![
        Tag::builder().key("Name").value(swarm_name).build(),
        Tag::builder().key(key).value(value).build(),
    ];

    // Define the TagSpecification to apply the tags when the instance is created
    let tag_specification = TagSpecification::builder()
        .resource_type("instance".into())
        .set_tags(Some(tags))
        .build();

    let block_device = BlockDeviceMapping::builder()
        .device_name(device_name) // Valid for Debian
        .ebs(EbsBlockDevice::builder().volume_size(100).build())
        .build();

    let instance_type = InstanceType::from_str(&instance_type_name).map_err(|err| {
        log::error!("Invalid instance type: {}", err);
        anyhow!(err.to_string())
    })?;

    let instance_profile_spec = IamInstanceProfileSpecification::builder()
        .name(aws_role)
        .build();

    let metadata_options = InstanceMetadataOptionsRequest::builder()
        .http_tokens(HttpTokensState::Required)
        .http_endpoint("enabled".into())
        .http_put_response_hop_limit(2)
        .build();

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
        .disable_api_termination(true)
        .iam_instance_profile(instance_profile_spec)
        .metadata_options(metadata_options)
        .send()
        // .map_err(|err| {
        //     log::error!("Error Creating instance instance: {}", err);
        //     anyhow!(err.to_string())
        // })
        .await;

    match result {
        Ok(response) => {
            if response.instances().is_empty() {
                return Err(anyhow!("Failed to create instance"));
            }
            let instance_id: String = response.instances()[0].instance_id().unwrap().to_string();
            log::info!("Created instance with ID: {}", instance_id);

            client
                .modify_instance_attribute()
                .instance_id(instance_id.clone())
                .disable_api_termination(
                    AttributeBooleanValue::builder()
                        .set_value(Some(true))
                        .build(),
                )
                .send()
                .await
                .map_err(|err| {
                    log::error!("Error enabling termination protection: {}", err);
                    anyhow::anyhow!(err.to_string())
                })?;

            log::info!(
                "Instance {} created and termination protection enabled.",
                instance_id
            );

            return Ok((instance_id, swarm_number));
        }
        Err(SdkError::ServiceError(service_error)) => {
            let err = service_error
                .err()
                .message()
                .unwrap_or("Unknown error")
                .to_string();
            log::error!("Service error: {}", err);
            return Err(anyhow!(err));
        }
        Err(SdkError::TimeoutError(_)) => {
            let err_msg = "Request timed out.";
            log::error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        }
        Err(SdkError::DispatchFailure(err)) => {
            log::error!("Network error: {:?}", err);
            return Err(anyhow!("Network error"));
        }
        Err(e) => {
            log::error!("Unexpected error: {:?}", e);
            return Err(anyhow!("Unexpected error"));
        }
    }
}

async fn get_instance_ip(instance_id: &str) -> Result<String, Error> {
    let region = getenv("AWS_REGION")?;
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region)));
    let config = aws_config::from_env()
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .load()
        .await;

    let client = Client::new(&config);

    log::info!("About to get instance ip address");

    let result = client
        .describe_instances()
        .instance_ids(instance_id)
        .send()
        // .map_err(|err| {
        //     log::error!("Error describing instance: {}", err);
        //     anyhow!(err.to_string())
        // })
        .await;

    match result {
        Ok(response) => {
            if response.reservations().is_empty() {
                return Err(anyhow!("Failed to create instance"));
            }

            if response.reservations()[0].instances().is_empty() {
                return Err(anyhow!("Could not get ec2 instance"));
            }

            if response.reservations()[0].instances()[0]
                .public_ip_address()
                .is_none()
            {
                return Err(anyhow!("No public ip address for the new instance"));
            }

            let public_ip_address = response.reservations()[0].instances()[0]
                .public_ip_address()
                .unwrap();

            log::info!("Instance Public IP Address: {}", public_ip_address);

            return Ok(public_ip_address.to_string());
        }
        Err(SdkError::ServiceError(service_error)) => {
            let err = service_error
                .err()
                .message()
                .unwrap_or("Unknown error")
                .to_string();
            log::error!("Service error: {}", err);
            return Err(anyhow!(err));
        }
        Err(SdkError::TimeoutError(_)) => {
            let err_msg = "Request timed out.";
            log::error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        }
        Err(SdkError::DispatchFailure(err)) => {
            log::error!("Network error: {:?}", err);
            return Err(anyhow!("Network error"));
        }
        Err(e) => {
            log::error!("Unexpected error: {:?}", e);
            return Err(anyhow!("Unexpected error"));
        }
    }
}

//Sample execution function
pub async fn create_swarm_ec2(
    info: &CreateEc2InstanceInfo,
    state: &mut Super,
) -> Result<(), Error> {
    let daily_limit = getenv("EC2_DAILY_LIMIT")
        .unwrap_or("5".to_string())
        .parse()
        .unwrap_or(5);

    let today_date = get_today_dash_date();
    if today_date == state.ec2_limit.date {
        if &state.ec2_limit.count < &daily_limit {
            state.ec2_limit.count = state.ec2_limit.count + 1;
        } else {
            return Err(anyhow!("Daily limit for creating Ec2 Instance exceeded"));
        }
    } else {
        state.ec2_limit.date = today_date;
        state.ec2_limit.count = 1;
    }
    let mut actual_vanity_address: Option<String> = None;

    let instance_type = get_instance(&info.instance_type);

    if instance_type.is_none() {
        return Err(anyhow!("Invalid instance type"));
    }

    if let Some(vanity_address) = &info.vanity_address {
        if !vanity_address.is_empty() {
            if let Some(subdomain) = vanity_address.strip_suffix(".sphinx.chat") {
                if subdomain.is_empty() {
                    return Err(anyhow!("Provide a valid vanity address"));
                }

                let domain_status = is_valid_domain(subdomain.to_string());
                if !domain_status.is_empty() {
                    return Err(anyhow!(domain_status));
                }
                actual_vanity_address = Some(vanity_address.to_string());
            } else {
                return Err(anyhow!("Vanity Address doesn't match the expected format."));
            }
        }
    }

    let ec2_intance = create_ec2_instance(
        info.name.clone(),
        actual_vanity_address.clone(),
        info.instance_type.clone(),
    )
    .await?;

    sleep(Duration::from_secs(40)).await;

    let default_host = format!("swarm{}.sphinx.chat", &ec2_intance.1);

    let ec2_ip_address = get_instance_ip(&ec2_intance.0).await?;
    let default_domain = format!("*.{}", default_host);
    let mut domain_names = vec![default_domain.as_str()];

    let mut host = default_host.clone();

    if let Some(custom_domain) = &actual_vanity_address {
        log::info!("vanity address is being set");
        if !custom_domain.is_empty() {
            host = custom_domain.clone();
            domain_names.push(custom_domain.as_str());
        }
    }

    let _ = add_domain_name_to_route53(domain_names, &ec2_ip_address).await?;

    log::info!("Public_IP: {}", ec2_ip_address);

    // add new ec2 to list of swarms
    let new_swarm = RemoteStack {
        host: host,
        ec2: Some(info.instance_type.clone()),
        default_host: default_host,
        note: Some("".to_string()),
        user: Some("".to_string()),
        pass: Some("".to_string()),
        ec2_instance_id: ec2_intance.0,
    };

    state.add_remote_stack(new_swarm);

    log::info!("New Swarm added to stack");
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

pub async fn update_aws_instance_type(
    details: UpdateInstanceDetails,
    state: &mut Super,
) -> Result<(), Error> {
    if details.instance_id.is_empty() {
        return Err(anyhow!("Please provide a valid instance id"));
    }

    if details.instance_type.is_empty() {
        return Err(anyhow!("Please provide a instance type"));
    }

    // find instance type
    let instance_types = instance_types();
    if let None = instance_types
        .iter()
        .position(|instance_type| instance_type.value == details.instance_type)
    {
        return Err(anyhow!("Invalid instance type"));
    }

    let swarm_pos = state
        .stacks
        .iter()
        .position(|swarm| swarm.ec2_instance_id == details.instance_id);

    if let None = swarm_pos {
        return Err(anyhow!("Instance does not exist"));
    }
    let unwrapped_swarm_pos = swarm_pos.unwrap();

    if let Some(current_instance) = &state.stacks[unwrapped_swarm_pos].ec2 {
        if details.instance_type == current_instance.to_string() {
            return Err(anyhow!("Please select a different instance type"));
        }
    }

    let ec2_instance_id = state.stacks[unwrapped_swarm_pos].ec2_instance_id.clone();

    let client = make_aws_client().await?;

    //update ec2 instance type
    update_ec2_instance_type(&client, &ec2_instance_id, &details.instance_type).await?;

    // get ec2 instance ip
    let new_ec2_ip_address = get_instance_ip(&details.instance_id).await?;

    let current_swarm: &RemoteStack = &state.stacks[unwrapped_swarm_pos];

    let defailt_domain = format!("*.{}", current_swarm.default_host.clone());

    let mut domain_names = vec![defailt_domain.as_str()];

    if current_swarm.default_host.clone() != current_swarm.host {
        domain_names.push(&current_swarm.host)
    }

    //update route53 record for both host and default_host
    let _ = add_domain_name_to_route53(domain_names, &new_ec2_ip_address).await?;

    // update stack with current instance type locally
    state.stacks[unwrapped_swarm_pos].ec2 = Some(details.instance_type);
    Ok(())
}

pub async fn stop_ec2_instance(client: &Client, instance_id: &str) -> Result<(), Error> {
    log::info!("Stopping instance: {}", instance_id);

    let result = client
        .stop_instances()
        .instance_ids(instance_id)
        .send()
        .await;

    match result {
        Ok(_response) => {
            log::info!("Waiting for instance to stop...");

            client
                .wait_until_instance_stopped()
                .instance_ids(instance_id)
                .wait(Duration::from_secs(120))
                .await?;

            log::info!("Instance Stopped...");
            Ok(())
        }
        Err(SdkError::ServiceError(service_error)) => {
            let err = service_error
                .err()
                .message()
                .unwrap_or("Unknown error")
                .to_string();
            log::error!("Service error: {}", err);
            return Err(anyhow!(err));
        }
        Err(SdkError::TimeoutError(_)) => {
            let err_msg = "Request timed out.";
            log::error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        }
        Err(SdkError::DispatchFailure(err)) => {
            log::error!("Network error: {:?}", err);
            return Err(anyhow!("Network error"));
        }
        Err(e) => {
            log::error!("Unexpected error: {:?}", e);
            return Err(anyhow!("Unexpected error"));
        }
    }
}

pub async fn start_ec2_instance(client: &Client, instance_id: &str) -> Result<(), Error> {
    let result = client
        .start_instances()
        .instance_ids(instance_id)
        .send()
        .await;

    match result {
        Ok(_response) => {
            log::info!("Waiting for instance to be running");

            client
                .wait_until_instance_running()
                .instance_ids(instance_id)
                .wait(Duration::from_secs(120))
                .await?;

            log::info!("Started instance successfully");
            return Ok(());
        }
        Err(SdkError::ServiceError(service_error)) => {
            let err = service_error
                .err()
                .message()
                .unwrap_or("Unknown error")
                .to_string();
            log::error!("Service error: {}", err);
            return Err(anyhow!(err));
        }
        Err(SdkError::TimeoutError(_)) => {
            let err_msg = "Request timed out.";
            log::error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        }
        Err(SdkError::DispatchFailure(err)) => {
            log::error!("Network error: {:?}", err);
            return Err(anyhow!("Network error"));
        }
        Err(e) => {
            log::error!("Unexpected error: {:?}", e);
            return Err(anyhow!("Unexpected error"));
        }
    }
}

pub async fn update_ec2_instance_type(
    client: &Client,
    instance_id: &str,
    instance_type: &str,
) -> Result<(), Error> {
    // stop ec2 instance
    stop_ec2_instance(client, &instance_id).await?;

    log::info!("Modifying Ec2 Instance...");
    // update ec2 instance
    let result = client
        .modify_instance_attribute()
        .instance_id(instance_id)
        .instance_type(
            AttributeValue::builder()
                .set_value(Some(instance_type.to_string()))
                .build(),
        )
        .send()
        .await;

    match result {
        Ok(_response) => {
            // state ec2 instance
            start_ec2_instance(&client, instance_id).await?;
            return Ok(());
        }
        Err(SdkError::ServiceError(service_error)) => {
            let err = service_error
                .err()
                .message()
                .unwrap_or("Unknown error")
                .to_string();
            log::error!("Service error: {}", err);
            return Err(anyhow!(err));
        }
        Err(SdkError::TimeoutError(_)) => {
            let err_msg = "Request timed out.";
            log::error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        }
        Err(SdkError::DispatchFailure(err)) => {
            log::error!("Network error: {:?}", err);
            return Err(anyhow!("Network error"));
        }
        Err(e) => {
            log::error!("Unexpected error: {:?}", e);
            return Err(anyhow!("Unexpected error"));
        }
    }
}

pub fn get_swarm_instance_type(
    info: GetInstanceTypeByInstanceId,
    state: &Super,
) -> Result<SuperSwarmResponse, Error> {
    if info.instance_id.is_empty() {
        return Err(anyhow!("Please provide a valid instance id"));
    }

    let swarm_pos = state
        .stacks
        .iter()
        .position(|swarm| swarm.ec2_instance_id == info.instance_id);

    if swarm_pos.is_none() {
        return Err(anyhow!("Swarm does not exist"));
    };

    let instance_res = GetInstanceTypeRes {
        instance_type: state.stacks[swarm_pos.unwrap()].ec2.clone(),
    };

    let value = serde_json::to_value(instance_res)?;

    return Ok(SuperSwarmResponse {
        success: true,
        message: "instance type".to_string(),
        data: Some(value),
    });
}

fn get_instance(instance_type: &str) -> Option<AwsInstanceType> {
    let instance_types = instance_types();
    let postion = instance_types
        .iter()
        .position(|instance| instance.value == instance_type);

    if let None = postion {
        return None;
    }

    return Some(instance_types[postion.unwrap()].clone());
}

pub async fn get_config(state: &mut Super) -> Result<Super, Error> {
    let key = getenv("SWARM_TAG_KEY")?;
    let value = getenv("SWARM_TAG_VALUE")?;
    let aws_instances = get_swarms_by_tag(&key, &value).await?;

    let mut aws_instances_hashmap: HashMap<String, InstanceFromAws> = HashMap::new();

    for aws_instance in aws_instances {
        aws_instances_hashmap.insert(aws_instance.instacne_id.clone(), aws_instance.clone());
    }

    for stack in state.stacks.iter_mut() {
        if aws_instances_hashmap.contains_key(&stack.ec2_instance_id) {
            let aws_instance_hashmap = aws_instances_hashmap.get(&stack.ec2_instance_id).unwrap();
            if stack.ec2.is_none() {
                stack.ec2 = Some(aws_instance_hashmap.intance_type.clone());
            } else {
                if aws_instance_hashmap.intance_type != stack.ec2.clone().unwrap() {
                    stack.ec2 = Some(aws_instance_hashmap.intance_type.clone())
                }
            }
        }
    }
    let res = state.remove_tokens();
    Ok(res)
}

pub fn get_today_dash_date() -> String {
    Local::now().format("%d-%m-%Y").to_string()
}
