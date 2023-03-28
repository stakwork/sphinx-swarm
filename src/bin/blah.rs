use anyhow::anyhow;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelayRes<T> {
    pub success: bool,
    pub response: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> RelayRes<T> {
    pub fn to_string(&self) -> Result<String> {
        if let Some(r) = &self.response {
            Ok(serde_json::to_string::<T>(r)?)
        } else if let Some(e) = &self.error {
            Err(anyhow!("{:?}", e))
        } else {
            if self.success {
                Ok(serde_json::to_string(&true)?)
            } else {
                Err(anyhow!("failed"))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chat {
    id: u16,
    uuid: Option<String>,
    name: Option<String>,
    photo_url: Option<String>,
    r#type: Option<u16>,
    group_key: Option<String>,
    host: Option<String>,
    price_to_join: Option<u64>,
    price_per_message: Option<u64>,
    escrow_amount: Option<u64>,
    escrow_millis: Option<u64>,
    private: Option<u8>,
    app_url: Option<String>,
    feed_url: Option<String>,
    tenant: Option<u16>,
    pin: Option<String>,
    default_join: Option<u8>,
}
pub type ChatsRes = Vec<Chat>;

#[rocket::main]
pub async fn main() -> Result<()> {
    let d = "{\"success\":true,\"response\":[{\"id\":8,\"uuid\":\"X3IWAiAW5vNrtOX5TLEJzqNWWr3rrUaXUwaqsfUXRMGNF7IWOHroTGbD4Gn2_rFuRZcsER0tZkrLw3sMnzj4RFAk_sx0\",\"name\":\"Planet Sphinx\",\"photo_url\":\"https://memes.sphinx.chat/public/HoQTHP3oOn0NAXOTqJEWb6HCtxIyN_14WGgiIgXpxWI=\",\"type\":2,\"status\":0,\"contact_ids\":[1,16],\"is_muted\":0,\"created_at\":\"2023-03-28T05:01:03.000Z\",\"updated_at\":\"2023-03-28T05:01:11.254Z\",\"deleted\":0,\"group_key\":\"MIIBCgKCAQEA0/tIQgklA0q9RlJ1ew4VN4HACPlSrf0/3wnI8YwqJUdyI9OwRz3EoxeHzWCxb5fk0ha15ry7csq7H1HohnS7DVtt0SVhBE9gX1OTktiuYAVUsfONnYqMsym8PRdAvOPO8h1arUfy0GobPOuZf449rFsMtz4sX07RERXsfiiA+B7UgmH/Wjw8GHoJznjz4F/zlhrmFvsSoUOoBRDZSqVROhC6vfDRrQTXYYD0pXlobE0Dp3zgDuVEZq+NoyaRYxSo0NVhGwSG/v5TbtHxfJmjv9qSQbNI2DaMkRtnFpckFdXVvFKtE6UZ8PvBEKC83FvTQ4rasN9PdhQdghnT7EJKewIDAQAB\",\"host\":\"tribes.sphinx.chat\",\"price_to_join\":0,\"price_per_message\":null,\"escrow_amount\":null,\"escrow_millis\":null,\"unlisted\":0,\"private\":0,\"owner_pubkey\":\"03a9a8d953fe747d0dd94dd3c567ddc58451101e987e2d2bf7a4d1e10a2c89ff38\",\"seen\":1,\"app_url\":null,\"feed_url\":null,\"feed_type\":null,\"meta\":null,\"my_photo_url\":null,\"my_alias\":null,\"tenant\":1,\"skip_broadcast_joins\":0,\"pin\":null,\"notify\":null,\"profile_filters\":null,\"call_recording\":null,\"meme_server_location\":null,\"jitsi_server\":null,\"stakwork_api_key\":null,\"stakwork_webhook\":null,\"default_join\":0},{\"id\":9,\"uuid\":\"dbd7b9eec395eeae563fb2ba41dbc446\",\"name\":null,\"photo_url\":null,\"type\":0,\"status\":null,\"contact_ids\":[1,17],\"is_muted\":0,\"created_at\":\"2023-03-28T05:03:25.000Z\",\"updated_at\":\"2023-03-28T05:14:52.561Z\",\"deleted\":0,\"group_key\":null,\"host\":null,\"price_to_join\":null,\"price_per_message\":null,\"escrow_amount\":null,\"escrow_millis\":null,\"unlisted\":0,\"private\":0,\"owner_pubkey\":null,\"seen\":1,\"app_url\":null,\"feed_url\":null,\"feed_type\":null,\"meta\":null,\"my_photo_url\":null,\"my_alias\":null,\"tenant\":1,\"skip_broadcast_joins\":0,\"pin\":null,\"notify\":null,\"profile_filters\":null,\"call_recording\":null,\"meme_server_location\":null,\"jitsi_server\":null,\"stakwork_api_key\":null,\"stakwork_webhook\":null,\"default_join\":0},{\"id\":10,\"uuid\":\"ZCJ2xx9xuYrZg51NIp57LGp5E1Zg_ustmD5GQYiOnydXnrlHMTodmERh5Je8LRed8rEOj3CbKXrnt60Jr3WPm4zUdspM\",\"name\":\"SphinxSwarm\",\"photo_url\":\"https://memes.sphinx.chat/public/EWicpVpp0x8fCSyAnfMH_iBOXJy_gzjovtZp0zKfpS4=\",\"type\":2,\"status\":null,\"contact_ids\":[1],\"is_muted\":0,\"created_at\":\"2023-03-28T05:10:31.000Z\",\"updated_at\":\"2023-03-28T05:29:53.263Z\",\"deleted\":0,\"group_key\":\"MIIBCgKCAQEAviEx7nDGw1omh1HQYpRwVG44msW3J2qCVhXikozLv6e0dHqwa5cn\\n8O8a7XpNhDs0UmvY4afmRRJtvdxIBo8uwgP1/k1nS0jf2o6q2GjQKzQxH7x0qFID\\ncRVx+aUEOFNIGowfm5Z3xPTK+YPAf+Q7j+hhChN+BiFwj3jZ9SjUGueMzuH1RLi/\\nuqof62D3xkmwXFPKwDnK2tyFuYQLuQ1idoLacdNGv1moVRvNtguI/BssWXI9/LHE\\nhomrstv0evjG4VyPZKXzq0Bj+UMtZ0p15KjTFLndcU8EIfrViVFxFbv/s1kjvPYv\\n689IWX5aJNlJdyKNLymuNpt28+W29u0CzQIDAQAB\",\"host\":\"tribes.sphinx.chat\",\"price_to_join\":0,\"price_per_message\":0,\"escrow_amount\":0,\"escrow_millis\":0,\"unlisted\":1,\"private\":0,\"owner_pubkey\":\"03a6ea2d9ead2120b12bd66292bb4a302c756983dc45dcb2b364b461c66fd53bcb\",\"seen\":0,\"app_url\":\"https://nav.swarm4.sphinx.chat\",\"feed_url\":\"\",\"feed_type\":0,\"meta\":null,\"my_photo_url\":null,\"my_alias\":null,\"tenant\":1,\"skip_broadcast_joins\":0,\"pin\":\"\",\"notify\":null,\"profile_filters\":\"\",\"call_recording\":0,\"meme_server_location\":\"\",\"jitsi_server\":\"\",\"stakwork_api_key\":\"\",\"stakwork_webhook\":\"\",\"default_join\":0}]}";

    let cs: RelayRes<ChatsRes> = serde_json::from_str(&d)?;
    println!("CS {:?}", cs.response);
    Ok(())
}
