use crate::{
    cmd::SuperSwarmResponse, ec2::instance_with_swarm_name_exists,
    route53::domain_exists_in_route53, state, util::is_valid_domain,
};
use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidateDomainRes {
    pub exist: bool,
    pub domain_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidateDomainAndSwarmNameRes {
    pub domain_exists: bool,
    pub swarm_name_exist: bool,
}

pub async fn check_domain(domain: &str) -> SuperSwarmResponse {
    if domain.is_empty() {
        return SuperSwarmResponse {
            success: false,
            message: "please provide valid domain".to_string(),
            data: None,
        };
    }
    let normalize_domain = domain.to_lowercase();
    let state = state::STATE.lock().await;

    let parsed_domain = if normalize_domain.strip_suffix(".sphinx.chat").is_none() {
        format!("{}.sphinx.chat", normalize_domain.to_string())
    } else {
        normalize_domain
    };

    // check if we have it has a domain
    let domain_validate_res =
        match validate_and_check_domain(&parsed_domain, state.reserved_domains.clone()).await {
            Ok(data) => data,
            Err(err) => {
                return SuperSwarmResponse {
                    success: false,
                    message: err.to_string(),
                    data: None,
                }
            }
        };
    drop(state);

    let swarm_name_exist =
        match instance_with_swarm_name_exists(&domain_validate_res.domain_name).await {
            Ok(status) => status,
            Err(err) => {
                return SuperSwarmResponse {
                    success: false,
                    message: err.to_string(),
                    data: None,
                };
            }
        };
    let json_value = match serde_json::to_value(ValidateDomainAndSwarmNameRes {
        domain_exists: domain_validate_res.exist,
        swarm_name_exist,
    }) {
        Ok(data) => data,
        Err(err) => {
            return SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            };
        }
    };

    SuperSwarmResponse {
        success: true,
        message: "domain and swarm named checked successfully".to_string(),
        data: Some(json_value),
    }
}

async fn validate_and_check_domain(
    domain: &str,
    reserved_domains: Option<Vec<String>>,
) -> Result<ValidateDomainRes, Error> {
    if let Some(subdomain) = domain.strip_suffix(".sphinx.chat") {
        if subdomain.is_empty() {
            return Err(anyhow!("Provide a valid vanity address"));
        }

        let domain_status = is_valid_domain(subdomain.to_string());
        if !domain_status.is_empty() {
            return Err(anyhow!(domain_status));
        }
        let vanity_address_exist = domain_exists_in_route53(domain, reserved_domains).await?;
        return Ok(ValidateDomainRes {
            exist: vanity_address_exist,
            domain_name: subdomain.to_string(),
        });
    }
    return Err(anyhow!("An unexpected error occured, please contact admin"));
}
