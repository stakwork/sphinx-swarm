use anyhow::{anyhow, Error};
use sphinx_swarm::cmd::LoginInfo;
use sphinx_swarm::utils::make_reqwest_client;

use crate::cmd::{AddSwarmResponse, LoginResponse, SuperSwarmResponse};
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

    // let route = format!("https://app.{}/api/login", swarm_details.host);
    let route = format!("http://{}/api/login", swarm_details.host);

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

pub async fn get_child_swarm_config(swarm_details: &RemoteStack) -> SuperSwarmResponse {
    return match login_to_child_swarm(swarm_details).await {
        Ok(details) => {
            log::info!("{}", details);
            SuperSwarmResponse {
                success: true,
                message: "tobi success".to_string(),
                data: None,
            }
        }
        Err(err) => {
            log::error!("{}", err);
            SuperSwarmResponse {
                success: false,
                message: "error occured while trying to login".to_string(),
                data: None,
            }
        }
    };
}
