use cosmwasm::types::{Coin, CanonicalAddr, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub region: String,
    pub beneficiary: HumanAddr,
    pub oracle: HumanAddr,
    pub ecostate: i64,
    pub total_tokens: i64,
    pub payout_start_height: Option<i64>,
    pub payout_end_height: Option<i64>,
    pub is_locked: Option<i64>

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    UpdateEcostate {ecostate: i64},
    Lock {},
    UnLock {},
    ChangeBeneficiary {beneficiary: HumanAddr},
    TransferOwnership {owner: HumanAddr},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryMsg {
    State {},
    GetBalance {address: HumanAddr}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BalanceResponse {
    pub balance: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub region: String,
    pub ecostate: i64,
    pub total_tokens: i64,
    pub released_tokens: i64,
    pub payout_start_height: Option<i64>,
    pub payout_end_height: Option<i64>,
    pub is_locked: Option<i64>,
    pub status: String,
    pub beneficiary: HumanAddr
}
