use snafu::ResultExt;

use cosmwasm::serde::to_vec;
use cosmwasm::traits::{Api, Extern, ReadonlyStorage, Storage};

use cosmwasm::types::{log, Env, Response, HumanAddr, CanonicalAddr, Coin, CosmosMsg};
use cosmwasm::errors::{contract_err, dyn_contract_err, unauthorized, Unauthorized, Result, SerializeErr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use cw_storage::{serialize, PrefixedStorage, ReadonlyPrefixedStorage};

use crate::state::{config, config_read, State};
use crate::msg::{ HandleMsg, InitMsg, QueryMsg, BalanceResponse, StateResponse};

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct Constants {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_BALANCES: &[u8] = b"balances";
pub const PREFIX_ALLOWANCES: &[u8] = b"allowances";

pub const KEY_CONSTANTS: &[u8] = b"constants";


pub fn init<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    msg: InitMsg,
) -> Result<Response> {
    let state = State {
        region: msg.region,
        beneficiary: deps.api.canonical_address(&msg.beneficiary)?,
        oracle: deps.api.canonical_address(&msg.oracle)?,
        ecostate: msg.ecostate,
        total_tokens: msg.total_tokens,
        released_tokens: 0,
        payout_start_height: msg.payout_start_height,
        payout_end_height: msg.payout_end_height,
        is_locked: msg.is_locked,
        owner: env.message.signer,
        status: "ACTIVE".to_string(),
    };

    config(&mut deps.storage).save(&state)?;

    let mut balances_store = PrefixedStorage::new(PREFIX_BALANCES, &mut deps.storage);

    let mut config_store = PrefixedStorage::new(PREFIX_CONFIG, &mut deps.storage);
    let constants = serialize(&Constants {
        name: "eco".to_string(),
        symbol: "ECO".to_string(),
        decimals: 0,
    })?;
    config_store.set(KEY_CONSTANTS, &constants);


    Ok(Response::default())
}

pub fn handle<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    msg: HandleMsg,
) -> Result<Response> {

    match msg {
        HandleMsg::UpdateEcostate {ecostate} => try_update_ecostate(deps, env, ecostate),
        HandleMsg::Lock { } => try_lock(deps, env),
        HandleMsg::UnLock { } => try_unlock(deps, env),
        HandleMsg::ChangeBeneficiary { beneficiary } =>
            try_change_beneficiary(deps, env,
                                   deps.api.canonical_address(&beneficiary)?),
        HandleMsg::TransferOwnership { owner } =>
            try_transfer_ownership(deps, env,
                                   deps.api.canonical_address(&owner)?)
    }
}


pub fn try_update_ecostate<S: Storage, A: Api>(deps: &mut Extern<S, A>, env: Env,
                                               ecostate: i64) -> Result<Response> {
    let log = vec![log("height", &env.block.height.to_string()),
                   log("ecostate", &ecostate.to_string())];

    let mut state = config(&mut deps.storage).load()?;
    let mut payout = 0;

    if (env.message.signer != state.oracle) {
        Unauthorized {}.fail()?;
    }
    else if (state.is_locked == Some(1)) {
        panic!("contract locked")
    } else if state.is_expired(&env) {
        panic!("contract expired")
    } else if (state.status == "DONE") {
        panic!("contract is done")
    }
    else {
        if (state.ecostate > ecostate) {
            //no payout
        } else {
            if (ecostate / 100 > 1) {
                payout = ecostate;
                //payout 100 coins per 1%
            } else {
                payout = ecostate / 2
            }
        }

        let mut total_tokens = state.total_tokens;

        let beneficiary_address_raw = &state.beneficiary;
        let mut account_balance = read_balance(&mut deps.storage,
                                               beneficiary_address_raw).unwrap();

        let mut balances_store = PrefixedStorage::new(PREFIX_BALANCES,
                                                      &mut deps.storage,);

        if (payout > total_tokens) {
            payout = total_tokens;
            state.status = "DONE".to_string();

        } else {
            total_tokens = total_tokens - payout;
        }
        account_balance += payout as u128;
        balances_store.set(state.beneficiary.as_slice(), &account_balance.to_be_bytes());
        state.total_tokens = total_tokens;

        state.ecostate = ecostate;

        config(&mut deps.storage).save(&state)?;
    }
    let from_human = deps.api.human_address(&state.oracle)?;
    let to_human = deps.api.human_address(&state.beneficiary)?;

    let amount = vec![coin(&payout.to_string(), "ecopay")];

    let r = Response {
        messages: vec![CosmosMsg::Send {
            from_address: from_human,
            to_address: to_human,
            amount,
        }],
        log: log,
        data: None,
    };
    Ok(r)
}

pub fn coin(amount: &str, denom: &str) -> Coin {
    Coin {
        amount: amount.to_string(),
        denom: denom.to_string(),
    }
}

pub fn try_lock<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
) -> Result<Response> {
    config(&mut deps.storage).update(&|mut state| {
        if (env.message.signer != state.owner) {
            Unauthorized {}.fail()?;
        }

        state.is_locked = Some(1);
        Ok(state)
    })?;
    Ok(Response::default())
}

pub fn try_unlock<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
) -> Result<Response> {
    config(&mut deps.storage).update(&|mut state| {
        if (env.message.signer != state.owner) {
            Unauthorized {}.fail()?;
        }

        state.is_locked = Some(0);
        Ok(state)
    })?;
    Ok(Response::default())
}

pub fn try_change_beneficiary<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    beneficiary: CanonicalAddr,
) -> Result<Response> {
    config(&mut deps.storage).update(&|mut state| {
        if (env.message.signer != state.owner) {
            Unauthorized {}.fail()?;
        }

        state.beneficiary = beneficiary.clone();
        Ok(state)
    })?;
    Ok(Response::default())
}

pub fn try_transfer_ownership<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    env: Env,
    owner: CanonicalAddr,
) -> Result<Response> {
    config(&mut deps.storage).update(&|mut state| {
        if env.message.signer != state.owner {
            Unauthorized {}.fail()?;
        }

        state.owner = owner.clone();
        Ok(state)
    })?;
    Ok(Response::default())
}


pub fn query<S: Storage, A: Api>(deps: &Extern<S, A>, msg: QueryMsg) -> Result<Vec<u8>> {
    match msg {
        QueryMsg::State {} => query_state(deps),

        QueryMsg::GetBalance { address } => {
            let address_key = deps.api.canonical_address(&address)?;
            let balance = read_balance(&deps.storage, &address_key)?;
            let out = serialize(&BalanceResponse {
                balance: balance.to_string(),
            })?;
            Ok(out)
        }
    }
}

fn query_state<S: Storage, A: Api>(deps: &Extern<S, A>) -> Result<Vec<u8>> {
    let state = config_read(&deps.storage).load()?;

    let out = serialize(&StateResponse {
        region: state.region.to_string(),
        total_tokens: state.total_tokens,
        ecostate: state.ecostate,
        payout_start_height: state.payout_start_height,
        payout_end_height: state.payout_end_height,
        is_locked: state.is_locked,
        released_tokens: state.released_tokens,
        status: state.status,
        beneficiary: HumanAddr::from("beneficiary")
    })?;
    Ok(out)
}

fn read_balance<S: Storage>(store: &S, owner: &CanonicalAddr) -> Result<u128> {
    let balance_store = ReadonlyPrefixedStorage::new(PREFIX_BALANCES, store);
    return read_u128(&balance_store, owner.as_slice());
}

// Converts 16 bytes value into u128
// Errors if data found that is not 16 bytes
pub fn bytes_to_u128(data: &[u8]) -> Result<u128> {
    match data[0..16].try_into() {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => contract_err("Corrupted data found. 16 byte expected."),
    }
}

// Reads 16 byte storage value into u128
// Returns zero if key does not exist. Errors if data found that is not 16 bytes
pub fn read_u128<S: ReadonlyStorage>(store: &S, key: &[u8]) -> Result<u128> {
    return match store.get(key) {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    };
}

// Source must be a decadic integer >= 0
pub fn parse_u128(source: &str) -> Result<u128> {
    match source.parse::<u128>() {
        Ok(value) => Ok(value),
        Err(_) => contract_err("Error while parsing string to u128"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm::errors::Error;
    use cosmwasm::mock::{dependencies, mock_env};
    use cosmwasm::serde::from_slice;
    use cosmwasm::types::coin;


    fn mock_env_height<A: Api>(
        api: &A,
        signer: &str,
        sent: &[Coin],
        balance: &[Coin],
        height: i64,
        time: i64,
    ) -> Env {
        let mut env = mock_env(api, signer, sent, balance);
        env.block.height = height;
        env.block.time = time;
        env
    }

    #[test]
    fn proper_initialization() {
        let mut deps = dependencies(20);

        let msg = InitMsg {
            region: "region-1".to_string(),
            beneficiary: HumanAddr::from("beneficiary"),
            oracle: HumanAddr::from("oracle"),
            ecostate: 2500,
            total_tokens: 100000,
            payout_start_height: Some(460000),
            payout_end_height: Some(1000000),
            is_locked: None
        };
        let beneficiary = deps
            .api
            .canonical_address(&HumanAddr::from("beneficiary"))
            .unwrap();

        let env = mock_env_height(&deps.api, "creator", &coin("1000", "earth"), &[], 460001, 0);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let beneficiary = HumanAddr::from("beneficiary");

        let res = query(&deps, QueryMsg::State {}).unwrap();
        let value: StateResponse = from_slice(&res).unwrap();
        assert_eq!(100000, value.total_tokens);
        assert_eq!(beneficiary, value.beneficiary);

        let res = query(&deps, QueryMsg::GetBalance {
            address: HumanAddr::from("beneficiary") }).unwrap();
        let value: BalanceResponse = from_slice(&res).unwrap();
        assert_eq!("0", value.balance);
    }

    #[test]
    fn update_ecostate() {
        let mut deps = dependencies(20);


        let beneficiary = HumanAddr::from("beneficiary");

        let msg = InitMsg {
            region: "region-1".to_string(),
            beneficiary: beneficiary,
            oracle: HumanAddr::from("oracle"),
            ecostate: 2500,
            total_tokens: 100000,
            payout_start_height: Some(460000),
            payout_end_height: Some(1000000),
            is_locked: None,
        };



        let env = mock_env_height(&deps.api, "creator", &coin("1000", "earth"), &[], 460000, 0);
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env(&deps.api, "oracle", &coin("2", "token"), &[]);
        let msg = HandleMsg::UpdateEcostate {ecostate: 5000};
        let _res = handle(&mut deps, env, msg).unwrap();


        // it worked, let's query the state
        let res = query(&deps, QueryMsg::State {}).unwrap();
        let value: StateResponse = from_slice(&res).unwrap();
        assert_eq!(5000, value.ecostate);

        let beneficiary = HumanAddr::from("beneficiary");

        let res = query(&deps, QueryMsg::GetBalance {
            address: beneficiary }).unwrap();
        let value: BalanceResponse = from_slice(&res).unwrap();
        assert_eq!("5000", value.balance);
    }

}
