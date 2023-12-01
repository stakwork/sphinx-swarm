use crate::utils::{domain, host_config, user};
use bollard::container::Config;

pub fn postgres() -> Config<String> {
    let name = "postgres";
    let root_vol = "/var/lib/postgresql/data";
    Config {
        image: Some("postgres".to_string()),
        hostname: Some(domain(name)),
        user: user(),
        host_config: host_config(name, vec![], root_vol, None, None),
        ..Default::default()
    }
}
