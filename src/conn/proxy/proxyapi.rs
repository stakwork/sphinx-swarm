use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::images::ProxyImage;

pub struct ProxyAPI;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComplexBalance {
    reserve: u128,
    full_balance: u128,
    balance: u128,
    pending_open_balance: u128,
}

impl ProxyAPI {
    pub fn new(proxy: &ProxyImage) -> Result<Self> {
       // Ok(Self())
        todo!()
    }

    pub async fn get_balance(&self) -> Result<ComplexBalance> {
        todo!();
    }
}
