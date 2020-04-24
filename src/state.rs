use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm::traits::Storage;
use cosmwasm::types::{log, CanonicalAddr, Coin, CosmosMsg, Env, Response};
use cw_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub region: String,
    pub beneficiary: CanonicalAddr,
    pub owner: CanonicalAddr,
    pub oracle: CanonicalAddr,
    pub ecostate: i64,
    pub total_tokens: i64,
    pub released_tokens: i64,
    pub payout_start_height: Option<i64>,
    pub payout_end_height: Option<i64>,
    pub is_locked: Option<i64>,
    pub status: String,

}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

impl State {
    pub fn is_expired(&self, env: &Env) -> bool {
        if let Some(payout_end_height) = self.payout_end_height {
            if env.block.height > payout_end_height {
                return true;
            }
        }

        return false;
    }
}
