use fairing::{Fairing, Info, Kind};
use rocket::*;
use tokio::sync::oneshot;

pub type Result<T> = std::result::Result<T, Error>;

/// Responses are received on the oneshot sender
#[derive(Debug)]
pub struct CmdRequest {
    pub tag: String,
    pub message: String,
    pub reply_tx: oneshot::Sender<String>,
}
impl CmdRequest {
    pub fn new(tag: &str, message: &str) -> (Self, oneshot::Receiver<String>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = CmdRequest {
            tag: tag.to_string(),
            message: message.to_string(),
            reply_tx,
        };
        (cr, reply_rx)
    }
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(http::Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS, PUT",
        ));
        response.set_header(http::Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(http::Header::new(
            "Access-Control-Allow-Credentials",
            "true",
        ));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed")]
    Fail,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hex error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("rocket error: {0}")]
    Rocket(#[from] rocket::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("BcryptError error: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),
    #[error("unauthorized")]
    Unauthorized,
}

use rocket::http::Status;
use rocket::response::{self, Responder};
impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'o> {
        match self {
            Error::Unauthorized => Status::Unauthorized.respond_to(req),
            _ => Status::InternalServerError.respond_to(req),
        }
    }
}
