use crate::config::Node;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SwarmCmd {
    AddNode(Node),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum RelayCmd {
    AddUser(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Cmd {
    Swarm(SwarmCmd),
    Relay(RelayCmd), // service, command
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Kind;

    #[test]
    fn test_cmd() {
        let c = Cmd::Swarm(SwarmCmd::AddNode(Node::new(
            "bitcoind",
            Kind::Bitcoind,
            vec![],
        )));
        println!("{}", serde_json::to_string(&c).unwrap());

        let c2 = Cmd::Relay(RelayCmd::AddUser("evan".to_string()));
        println!("{}", serde_json::to_string(&c2).unwrap());

        assert!(true == true)
    }
}
