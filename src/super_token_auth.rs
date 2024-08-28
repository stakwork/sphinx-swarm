use rocket::request::{FromRequest, Outcome, Request};

#[derive(Debug)]
pub enum SuperAuthError {
    Unauthorized,
}

#[derive(Clone)]
pub struct VerifySuperToken {
    pub token: String,
    pub verified: bool,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for VerifySuperToken {
    type Error = SuperAuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(token) = request.headers().get_one("x-super-token") {
            return Outcome::Success(VerifySuperToken {
                token: token.to_string(),
                verified: true,
            });
        } else {
            Outcome::Success(VerifySuperToken {
                token: "".to_string(),
                verified: false,
            })
        }
    }
}
