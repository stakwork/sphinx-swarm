use anyhow::Context;
use anyhow::Result;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use jwt::VerifyWithKey;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use sha2::Sha256;
use std::collections::BTreeMap;
type Claims = BTreeMap<String, u32>;

#[derive(Clone)]
pub struct AdminJwtClaims {
    pub exp: u32,
    pub user: u32,
}

impl AdminJwtClaims {
    pub fn from_claims(claims: Claims) -> Result<Self> {
        Ok(Self {
            exp: *claims.get("exp").context("no exp")?,
            user: *claims.get("user").context("no user")?,
        })
    }
    pub fn check(token: &str) -> std::result::Result<Self, JwtError> {
        let claims: Claims = token
            .verify_with_key(&jwt_key())
            .map_err(|_| JwtError::Invalid)?;
        let jwtc = AdminJwtClaims::from_claims(claims).map_err(|_| JwtError::Missing)?;
        if jwtc.clone().exp < now() {
            Err(JwtError::Expired)
        } else {
            Ok(jwtc)
        }
    }
}

fn jwt_key() -> Hmac<Sha256> {
    let env_sec = std::env::var("JWT_KEY").unwrap_or("some-secret".to_string());
    let key: Hmac<Sha256> = Hmac::new_from_slice(env_sec.as_bytes()).expect("failed");
    key
}

pub fn make_jwt(user: u32) -> Result<String> {
    let mut claims = BTreeMap::new();
    claims.insert("exp", now() + days(7));
    claims.insert("user", user);
    let token = claims.sign_with_key(&jwt_key())?;
    Ok(token)
}

pub fn days(n: u32) -> u32 {
    n * 24 * 60 * 60
}

fn now() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    u32::try_from(
        start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs(),
    )
    .expect("Time jumped forward")
}

#[derive(Debug)]
pub enum JwtError {
    Missing,
    Invalid,
    Expired,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminJwtClaims {
    type Error = JwtError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = req.headers().get_one("x-jwt");
        if let None = token {
            return Outcome::Failure((Status::Unauthorized, JwtError::Missing));
        }
        match AdminJwtClaims::check(token.unwrap()) {
            Ok(jwtc) => Outcome::Success(jwtc),
            Err(e) => Outcome::Failure((Status::Unauthorized, e)),
        }
    }
}

pub fn hash_pass(pwd: &str) -> Result<bool> {
    let hashed = bcrypt::hash(pwd, bcrypt::DEFAULT_COST)?;
    let valid = bcrypt::verify(pwd, &hashed)?;
    Ok(valid)
}
