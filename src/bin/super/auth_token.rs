use rocket::request::{FromRequest, Outcome, Request};

#[derive(Debug)]
pub enum SuperAuthError {}

#[derive(Clone)]
pub struct VerifySuperToken {
    pub token: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for VerifySuperToken {
    type Error = SuperAuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(token) = request.headers().get_one("x-super-token") {
            return Outcome::Success(VerifySuperToken {
                token: Some(token.to_string()),
            });
        } else {
            Outcome::Success(VerifySuperToken { token: None })
        }
    }
}
