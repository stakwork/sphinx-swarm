use std::collections::HashMap;

use anyhow::{anyhow, Error};
use reqwest::Response;
use serde_json::Value;
use sphinx_swarm::cmd::{send_cmd_request, LoginInfo, SendCmdData, UpdateNode};
use sphinx_swarm::config::Stack;
use sphinx_swarm::utils::make_reqwest_client;

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
    let res = handle_get_child_swarm_config(&swarm_details.host, &token).await?;

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

pub async fn handle_get_child_swarm_config(host: &str, token: &str) -> Result<Response, Error> {
    let data = SendCmdData {
        cmd: "GetConfig".to_string(),
        content: None,
    };

    let url = get_child_base_route(host);
    let cmd_res = send_cmd_request(
        "Swarm".to_string(),
        data,
        "SWARM",
        &url,
        Some("x-jwt"),
        Some(&token),
    )
    .await?;

    Ok(cmd_res)
}

pub fn get_child_base_route(host: &str) -> String {
    let url = format!("https://app.{}/api", host);

    // return format!("http://{}/api", host);
}

pub async fn get_child_swarm_containers(
    swarm_details: &RemoteStack,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;
    let res = handle_get_child_swarm_containers(&swarm_details.host, &token).await?;

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

async fn handle_get_child_swarm_containers(host: &str, token: &str) -> Result<Response, Error> {
    let data = SendCmdData {
        cmd: "ListContainers".to_string(),
        content: None,
    };

    let url = get_child_base_route(host);
    let cmd_res = send_cmd_request(
        "Swarm".to_string(),
        data,
        "SWARM",
        &url,
        Some("x-jwt"),
        Some(&token),
    )
    .await?;

    Ok(cmd_res)
}

pub async fn access_child_swarm_containers(
    swarm_details: &RemoteStack,
    nodes: Vec<String>,
    cmd: &str,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;
    let mut errors: HashMap<String, String> = HashMap::new();

    for node in nodes {
        let data: Value;
        if cmd == "UpdateNode" {
            match serde_json::to_value(UpdateNode {
                id: node.clone(),
                version: "latest".to_string(),
            }) {
                Ok(value) => data = value,
                Err(err) => {
                    log::error!("Error trying to convert to value: {}", &err);
                    errors.insert(node, err.to_string());
                    continue;
                }
            }
        } else {
            match serde_json::to_value(node.clone()) {
                Ok(value) => data = value,
                Err(err) => {
                    log::error!("Error trying to convert to value: {}", &err);
                    errors.insert(node, err.to_string());
                    continue;
                }
            }
        }

        match handle_access_child_container(&swarm_details.host, &token, data, &cmd).await {
            Ok(res) => {
                if res.status().clone() != 200 {
                    errors.insert(
                        node.clone(),
                        format!(
                            "{} status error trying to {} {}",
                            res.status(),
                            &cmd,
                            node.clone()
                        ),
                    );
                }
            }
            Err(err) => {
                log::error!("Error trying to {}: {}", &cmd, &err);
                errors.insert(node, err.to_string());
            }
        }
    }

    if !errors.is_empty() {
        match serde_json::to_value(errors) {
            Ok(error_map) => {
                return Ok(SuperSwarmResponse {
                    success: false,
                    message: format!("Error occured trying to {}", cmd),
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
        message: format!("{} executed successfully", cmd),
        data: None,
    })
}

async fn handle_access_child_container(
    host: &str,
    token: &str,
    data: Value,
    cmd: &str,
) -> Result<Response, Error> {
    let data = SendCmdData {
        cmd: cmd.to_string(),
        content: Some(data),
    };

    let url = get_child_base_route(host);
    let cmd_res = send_cmd_request(
        "Swarm".to_string(),
        data,
        "SWARM",
        &url,
        Some("x-jwt"),
        Some(&token),
    )
    .await?;

    Ok(cmd_res)
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
