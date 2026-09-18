use crate::secrets;
use anyhow::Context;
use anyhow::Result;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use jwt::VerifyWithKey;
use once_cell::sync::Lazy;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::sync::Mutex;

type Claims = BTreeMap<String, u32>;

/// Holds the admin JWT signing key. `None` until a key is configured
/// (or until a random key is generated on first use when nothing is configured).
pub static JWT_KEY: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(Default::default()));

/// Install the HMAC secret used to sign and verify admin JWTs.
///
/// Empty / whitespace-only keys are rejected: with an empty key anyone could
/// forge an admin token. Callers should treat the returned error as fatal.
pub fn set_jwt_key(b: &str) -> Result<()> {
    if b.trim().is_empty() {
        anyhow::bail!("jwt key cannot be empty");
    }
    *JWT_KEY.lock().unwrap() = Some(b.to_string());
    Ok(())
}

/// Install the signing key from a stack config, where an empty string means
/// "not configured" (local dev): [`get_jwt_key`] then generates a random key
/// on first use instead. A configured-but-blank key is still a hard error.
pub fn set_jwt_key_from_config(b: &str) -> Result<()> {
    if b.is_empty() {
        return Ok(());
    }
    set_jwt_key(b)
}

/// The admin JWT signing key:
/// - a key installed via [`set_jwt_key`] is used as-is;
/// - a key that is somehow empty is an error (never sign with an empty key);
/// - when nothing is configured, a random key is generated once and kept for
///   the process lifetime. There is no hardcoded fallback secret; note that a
///   generated key rotates on restart, invalidating previously issued tokens
///   (local dev: set `jwt_key` in the stack config to keep tokens stable).
pub fn get_jwt_key() -> Result<String> {
    let mut jk = JWT_KEY.lock().unwrap();
    match jk.as_deref() {
        Some(k) if k.trim().is_empty() => anyhow::bail!("jwt key is set but empty"),
        Some(k) => Ok(k.to_string()),
        None => {
            let k = secrets::random_word(48);
            *jk = Some(k.clone());
            Ok(k)
        }
    }
}

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
        let key = jwt_key().map_err(|_| JwtError::Invalid)?;
        let claims: Claims = token
            .verify_with_key(&key)
            .map_err(|_| JwtError::Invalid)?;
        let jwtc = AdminJwtClaims::from_claims(claims).map_err(|_| JwtError::Missing)?;
        if jwtc.clone().exp < now() {
            Err(JwtError::Expired)
        } else {
            Ok(jwtc)
        }
    }
}

fn jwt_key() -> Result<Hmac<Sha256>> {
    let jk = get_jwt_key()?;
    let key: Hmac<Sha256> = Hmac::new_from_slice(jk.as_bytes())
        .context("failed to build hmac key from jwt key")?;
    Ok(key)
}

pub fn make_jwt(user: u32) -> Result<String> {
    let mut claims = BTreeMap::new();
    claims.insert("exp", now() + days(7));
    claims.insert("user", user);
    let token = claims.sign_with_key(&jwt_key()?)?;
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
            return Outcome::Error((Status::Unauthorized, JwtError::Missing));
        }
        match AdminJwtClaims::check(token.unwrap()) {
            Ok(jwtc) => Outcome::Success(jwtc),
            Err(e) => Outcome::Error((Status::Unauthorized, e)),
        }
    }
}

pub fn hash_pass(pwd: &str) -> Result<bool> {
    let hashed = bcrypt::hash(pwd, bcrypt::DEFAULT_COST)?;
    let valid = bcrypt::verify(pwd, &hashed)?;
    Ok(valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JWT_KEY is process-global state: serialize every test that touches it.
    static KEY_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_key() {
        *JWT_KEY.lock().unwrap() = None;
    }

    #[test]
    fn get_jwt_key_generates_random_key_when_unset() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        let k1 = get_jwt_key().expect("should generate a random key");
        assert!(!k1.is_empty());
        assert_ne!(k1, "some-secret");
        assert_eq!(k1.len(), 48);
        assert!(k1.chars().all(|c| c.is_ascii_alphanumeric()));
        // the generated key is stable for the process lifetime
        let k2 = get_jwt_key().expect("should reuse the generated key");
        assert_eq!(k1, k2);
        reset_key();
    }

    #[test]
    fn set_jwt_key_rejects_empty_and_blank() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        assert!(set_jwt_key("").is_err());
        assert!(set_jwt_key("   ").is_err());
        // failed installs leave no key behind
        assert!(JWT_KEY.lock().unwrap().is_none());
        reset_key();
    }

    #[test]
    fn set_jwt_key_accepts_real_key() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        set_jwt_key("a-real-key").expect("valid key should install");
        assert_eq!(get_jwt_key().unwrap(), "a-real-key");
        reset_key();
    }

    #[test]
    fn get_jwt_key_errors_when_key_set_but_empty() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        // install directly, bypassing set_jwt_key, to simulate a legacy empty key
        *JWT_KEY.lock().unwrap() = Some("".to_string());
        assert!(get_jwt_key().is_err());
        *JWT_KEY.lock().unwrap() = Some("  ".to_string());
        assert!(get_jwt_key().is_err());
        reset_key();
    }

    #[test]
    fn set_jwt_key_from_config_empty_means_not_configured() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        // an empty config value means "not configured": a random key is
        // generated on first use instead of a hardcoded fallback
        set_jwt_key_from_config("").expect("empty = not configured");
        let k = get_jwt_key().unwrap();
        assert!(!k.is_empty());
        assert_ne!(k, "some-secret");
        reset_key();
    }

    #[test]
    fn set_jwt_key_from_config_blank_is_fatal() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        assert!(set_jwt_key_from_config("  ").is_err());
        reset_key();
    }

    #[test]
    fn make_jwt_roundtrips_through_check() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        let token = make_jwt(7).expect("sign with generated key");
        let claims = AdminJwtClaims::check(&token).expect("token verifies with same key");
        assert_eq!(claims.user, 7);
        assert!(claims.exp > 0);
        reset_key();
    }

    #[test]
    fn check_rejects_token_signed_with_different_key() {
        let _g = KEY_TEST_LOCK.lock().unwrap();
        reset_key();
        let token = make_jwt(7).unwrap();
        set_jwt_key("another-key").unwrap();
        match AdminJwtClaims::check(&token) {
            Err(JwtError::Invalid) => {}
            Err(e) => panic!("expected Invalid, got JwtError::{:?}", e),
            Ok(_) => panic!("expected Invalid, got a valid token"),
        }
        reset_key();
    }
}
