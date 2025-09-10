use rand::{distributions::Alphanumeric, Rng};
use sphinx_swarm::utils::getenv;

pub fn check_reserve_swarm_flag_set() -> bool {
    match getenv("RESERVE_SWARM_ENABLED") {
        Ok(value) => {
            if value == "true" || value == "1" {
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

pub fn generate_random_secret(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric) // [A-Za-z0-9]
        .take(length)
        .map(char::from)
        .collect()
}
