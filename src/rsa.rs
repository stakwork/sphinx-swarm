use rsa::{
    pkcs1::{EncodeRsaPrivateKey, LineEnding},
    RsaPrivateKey,
};

pub fn rand_key() -> String {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    // remove cert stuff
    let mut priv_pem = priv_key
        .to_pkcs1_pem(LineEnding::LF)
        .expect("failed to_pkcs1_pem")
        .to_string();
    priv_pem.replace_range(0..32, "");
    priv_pem.replace_range(priv_pem.len() - 31.., "");
    // return with no \n
    priv_pem.replace('\n', "")
}
