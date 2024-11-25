use crate::{
    cmd::{
        ChangeLightningBotLabel, LightningBotAccountRes, LightningBotBalanceRes, SuperSwarmResponse, CreateInvoiceLightningBotReq,LightningBotCreateInvoiceReq
    },
    state::{LightningBotsDetails, Super},
};
use anyhow::Error;
use reqwest::{Response, StatusCode};
use serde_json::Value;
use sphinx_swarm::utils::make_reqwest_client;

pub async fn get_lightning_bots_details(state: &Super) -> SuperSwarmResponse {
    let mut lightning_bots_details: Vec<LightningBotsDetails> = Vec::new();
    for bot in &state.lightning_bots {
        // get bot details
        let bot_details_res = match make_get_request_to_bot(&bot.url, &bot.token, "account").await {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error getting details: {}", err.to_string());
                lightning_bots_details.push(make_err_response(
                    format!(
                        "Error making get account details request: {}",
                        err.to_string()
                    ),
                    bot.label.clone(),
                    bot.url.clone(),
                ));
                continue;
            }
        };

        if bot_details_res.status() != 200 {
            let status_code = bot_details_res.status().clone();
            log::error!("Response body: {:?}", bot_details_res.text().await);
            lightning_bots_details.push(make_err_response(
                format!(
                    "Got this response code trying to get bot account details: {}",
                    status_code
                ),
                bot.label.clone(),
                bot.url.clone(),
            ));
            continue;
        }

        let bot_details: LightningBotAccountRes = match bot_details_res.json().await {
            Ok(value) => value,
            Err(err) => {
                log::error!("Error parsing response details: {}", err.to_string());
                lightning_bots_details.push(make_err_response(
                    format!(
                        "Error parsing response details from get account: {}",
                        err.to_string()
                    ),
                    bot.label.clone(),
                    bot.url.clone(),
                ));
                continue;
            }
        };

        // get bot balance
        let bot_balance_res = match make_get_request_to_bot(&bot.url, &bot.token, "balance").await {
            Ok(res) => res,
            Err(err) => {
                log::error!("Error getting balance: {}", err.to_string());
                lightning_bots_details.push(make_err_response(
                    format!("Error getting bot balance: {}", err.to_string()),
                    bot.label.clone(),
                    bot.url.clone(),
                ));
                continue;
            }
        };

        if bot_balance_res.status() != 200 {
            let status_code = bot_balance_res.status().clone();
            log::error!("Response body: {:?}", bot_balance_res.text().await);
            lightning_bots_details.push(make_err_response(
                format!(
                    "Got this response code trying to get bot balance: {}",
                    status_code
                ),
                bot.label.clone(),
                bot.url.clone(),
            ));
            continue;
        }

        let bot_balance: LightningBotBalanceRes = match bot_balance_res.json().await {
            Ok(value) => value,
            Err(err) => {
                log::error!("Error parsing response details: {}", err.to_string());
                lightning_bots_details.push(make_err_response(
                    format!(
                        "Error parsing response details from getting bot balance: {}",
                        err.to_string()
                    ),
                    bot.label.clone(),
                    bot.url.clone(),
                ));
                continue;
            }
        };

        lightning_bots_details.push(LightningBotsDetails {
            balance_in_msat: bot_balance.msat,
            contact_info: bot_details.contact_info,
            alias: bot_details.alias,
            error_message: "".to_string(),
            network: bot_details.network,
            id: bot.url.clone(),
            label: bot.label.clone(),
        })
    }

    let lightning_bots = match serde_json::to_value(&lightning_bots_details) {
        Ok(json) => json,
        Err(err) => {
            log::error!("Error converting vec to Value: {}", err.to_string());
            return SuperSwarmResponse {
                success: false,
                message: format!("Error converting vec to Value: {}", err.to_string()),
                data: None,
            };
        }
    };

    SuperSwarmResponse {
        success: true,
        message: "lightning details".to_string(),
        data: Some(lightning_bots),
    }
}

async fn make_get_request_to_bot(
    host: &str,
    token: &str,
    endpoint: &str,
) -> Result<Response, Error> {
    let client = make_reqwest_client();

    let res = client
        .get(format!("https://{}/{}", host, endpoint))
        .header("x-admin-token", token)
        .send()
        .await?;

    Ok(res)
}

fn make_err_response(err_msg: String, label: String, id: String) -> LightningBotsDetails {
    LightningBotsDetails {
        balance_in_msat: 0,
        contact_info: "".to_string(),
        alias: "".to_string(),
        error_message: err_msg,
        network: "".to_string(),
        id: id,
        label: label,
    }
}

pub async fn change_lightning_bot_label(
    state: &mut Super,
    must_save_stack: &mut bool,
    info: ChangeLightningBotLabel,
) -> SuperSwarmResponse {
    if info.new_label.is_empty() {
        return SuperSwarmResponse {
            success: false,
            message: "Label cannot be empty".to_string(),
            data: None,
        };
    }
    let bot_pos = &state
        .lightning_bots
        .iter()
        .position(|bot| bot.url == info.id);

    if bot_pos.is_none() {
        return SuperSwarmResponse {
            success: false,
            message: "Invalid bot".to_string(),
            data: None,
        };
    }

    let actual_bot_pos = bot_pos.unwrap();

    state.lightning_bots[actual_bot_pos].label = info.new_label;

    *must_save_stack = true;

    SuperSwarmResponse {
        success: true,
        message: "label updated successfully".to_string(),
        data: None,
    }
}

pub async fn create_invoice_lightning_bot(state: &Super,info: CreateInvoiceLightningBotReq)-> SuperSwarmResponse {
    // find bot
    let bot_option = state.lightning_bots.iter().find(|lightning_bot | lightning_bot.url == info.id);
    if bot_option.is_none() {
        return SuperSwarmResponse {
            success: false,
            message: "bot does not exist".to_string(),
            data: None
        }
    }

    let bot = bot_option.unwrap();

    // make request to bot server
    let invoice_res = match create_invoice_request(&bot.url, &bot.token, info.amt_msat).await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Error making request to create invoice: {}", err.to_string());
            return SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None
            }
        }
    };

    if invoice_res.status() != StatusCode::OK {
        log::error!("Did not get status created OK for creating invoice: {}", invoice_res.status());
        return SuperSwarmResponse {
            success: false,
            message:format!("Got unexpected status response {}", invoice_res.status()),
            data: None
        }
    }

    let invoice_details:Value =  match invoice_res.json().await {
        Ok(value) => value,
        Err(err) => {
            log::error!("Error converting response to value: {}", err.to_string());
            return SuperSwarmResponse {
                success: false,
                message: format!("Error converting response to value: {}", err.to_string()),
                data: None
            }
        }
    };

    // return response to frontend
    SuperSwarmResponse {
        success: true,
        message: "invoice created".to_string(),
        data: Some(invoice_details)
    }
}

async fn create_invoice_request(url: &str, token: &str, amt_msat: u64) -> Result<Response, Error> {
    let client = make_reqwest_client();

    let body = LightningBotCreateInvoiceReq {
        amt_msat: amt_msat
    };

    let res = client.post(format!("https://{}/invoice", url)).header("x-admin-token", token).json(&body).send().await?;

    Ok (res)
}
