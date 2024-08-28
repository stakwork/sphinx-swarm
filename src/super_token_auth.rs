use crate::utils::getenv;
use rocket::request::{FromRequest, Outcome, Request};

#[derive(Debug)]
pub enum SuperAuthError {
    Unauthorized,
}

#[derive(Clone)]
pub struct VerifySuperToken {
    pub verified: bool,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for VerifySuperToken {
    type Error = SuperAuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(token) = request.headers().get_one("x-super-token") {
            let super_token = getenv("SUPER_TOKEN").unwrap_or("".to_string());

            if super_token.is_empty() {
                log::error!("SUPER_TOKEN is not set, please set ASAP");
                return Outcome::Success(VerifySuperToken { verified: false });
            }
            if super_token.as_str() != token {
                log::error!("Invalid super key passed");
                return Outcome::Success(VerifySuperToken { verified: false });
            }
            return Outcome::Success(VerifySuperToken { verified: true });
        } else {
            Outcome::Success(VerifySuperToken { verified: false })
        }
    }
}
