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
