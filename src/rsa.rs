use rsa::{
    pkcs1::{EncodeRsaPrivateKey, LineEnding},
    RsaPrivateKey,
};

pub fn _rand_key() -> String {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");

    if let Ok(ps) = priv_key.to_pkcs1_pem(LineEnding::LF) {
        return ps.to_string();
    }

    "".to_string()
}
