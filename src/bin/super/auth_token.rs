use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};

#[derive(Debug)]
pub enum SuperAuthError {
    MissingToken,
    InvalidToken,
}

#[derive(Clone)]
pub struct VerifySuperToken {
    pub token: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for VerifySuperToken {
    type Error = SuperAuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // If SUPER_TOKEN env is defined, the header MUST be present and match it.
        match crate::getenv("SUPER_TOKEN") {
            Ok(super_token) => {
                // Env defined: require header and exact match
                if let Some(token) = request.headers().get_one("x-super-token") {
                    if super_token == token {
                        return Outcome::Success(VerifySuperToken {
                            token: Some(token.to_string()),
                        });
                    } else {
                        return Outcome::Error((
                            Status::Unauthorized,
                            SuperAuthError::InvalidToken,
                        ));
                    }
                }
                Outcome::Error((Status::Unauthorized, SuperAuthError::MissingToken))
            }
            Err(_) => Outcome::Error((Status::Unauthorized, SuperAuthError::MissingToken)),
        }
    }
}
