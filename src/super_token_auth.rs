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
            let x_super_key = getenv("X_SUPER_KEY").unwrap_or("".to_string());

            if x_super_key.is_empty() {
                log::error!("X_SUPER_KEY is not set, please set ASAP");
                return Outcome::Success(VerifySuperToken { verified: false });
            }
            if x_super_key.as_str() != token {
                log::error!("Invalid super key passed");
                return Outcome::Success(VerifySuperToken { verified: false });
            }
            return Outcome::Success(VerifySuperToken { verified: true });
        } else {
            Outcome::Success(VerifySuperToken { verified: false })
        }
    }
}
