#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use terraswap::querier::{query_balance, query_pair_info_from_pair};

use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    add_allow_native_token, pair_key, read_pairs, Config, TmpPairInfo, ALLOW_NATIVE_TOKENS, CONFIG,
    PAIRS, TMP_PAIR_INFO,
};

use protobuf::Message;
use terraswap::asset::{AssetInfo, PairInfo, PairInfoRaw};
use terraswap::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, NativeTokenDecimalsResponse,
    PairsResponse, QueryMsg,
};
use terraswap::pair::{InstantiateMsg as PairInstantiateMsg, MigrateMsg as PairMigrateMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:terraswap-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            token_code_id,
            pair_code_id,
        } => execute_update_config(deps, env, info, owner, token_code_id, pair_code_id),
        ExecuteMsg::CreatePair { asset_infos } => execute_create_pair(deps, env, info, asset_infos),
        ExecuteMsg::AddNativeTokenDecimals { denom, decimals } => {
            execute_add_native_token_decimals(deps, env, info, denom, decimals)
        }
        ExecuteMsg::MigratePair { contract, code_id } => {
            execute_migrate_pair(deps, env, info, contract, code_id)
        }
    }
}

// Only owner can execute it
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        // validate address format
        let _ = deps.api.addr_validate(&owner)?;

        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }

    if let Some(pair_code_id) = pair_code_id {
        config.pair_code_id = pair_code_id;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// Anyone can execute it to create swap pair
pub fn execute_create_pair(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    asset_infos: [AssetInfo; 2],
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    if asset_infos[0] == asset_infos[1] {
        return Err(StdError::generic_err("same asset"));
    }

    let asset_1_decimal =
        match asset_infos[0].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => return Err(StdError::generic_err("asset1 is invalid")),
        };

    let asset_2_decimal =
        match asset_infos[1].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => return Err(StdError::generic_err("asset2 is invalid")),
        };

    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ];

    let asset_decimals = [asset_1_decimal, asset_2_decimal];

    let pair_key = pair_key(&raw_infos);
    if let Ok(Some(_)) = PAIRS.may_load(deps.storage, &pair_key) {
        return Err(StdError::generic_err("Pair already exists"));
    }

    TMP_PAIR_INFO.save(
        deps.storage,
        &TmpPairInfo {
            pair_key,
            asset_infos: raw_infos,
            asset_decimals,
        },
    )?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &format!("{}-{}", asset_infos[0], asset_infos[1])),
        ])
        .add_submessage(SubMsg {
            id: 1,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.pair_code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label: "pair".to_string(),
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos,
                    token_code_id: config.token_code_id,
                    asset_decimals,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

pub fn execute_add_native_token_decimals(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    decimals: u8,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let balance = query_balance(&deps.querier, env.contract.address, denom.to_string())?;
    if balance.is_zero() {
        return Err(StdError::generic_err(
            "a balance greater than zero is required by the factory for verification",
        ));
    }

    add_allow_native_token(deps.storage, denom.to_string(), decimals)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "add_allow_native_token"),
        ("denom", &denom),
        ("decimals", &decimals.to_string()),
    ]))
}

pub fn execute_migrate_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract: String,
    code_id: Option<u64>,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let code_id = code_id.unwrap_or(config.pair_code_id);

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: contract,
            new_code_id: code_id,
            msg: to_binary(&PairMigrateMsg {})?,
        })),
    )
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let tmp_pair_info = TMP_PAIR_INFO.load(deps.storage)?;

    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(msg.result.unwrap().data.unwrap().as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let pair_contract = res.get_address();
    let pair_info = query_pair_info_from_pair(&deps.querier, Addr::unchecked(pair_contract))?;

    PAIRS.save(
        deps.storage,
        &tmp_pair_info.pair_key,
        &PairInfoRaw {
            liquidity_token: deps.api.addr_canonicalize(&pair_info.liquidity_token)?,
            contract_addr: deps.api.addr_canonicalize(pair_contract)?,
            asset_infos: tmp_pair_info.asset_infos,
            asset_decimals: tmp_pair_info.asset_decimals,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        ("pair_contract_addr", pair_contract),
        ("liquidity_token_addr", pair_info.liquidity_token.as_str()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Pair { asset_infos } => to_binary(&query_pair(deps, asset_infos)?),
        QueryMsg::Pairs { start_after, limit } => {
            to_binary(&query_pairs(deps, start_after, limit)?)
        }
        QueryMsg::NativeTokenDecimals { denom } => {
            to_binary(&query_native_token_decimal(deps, denom)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        token_code_id: state.token_code_id,
        pair_code_id: state.pair_code_id,
    };

    Ok(resp)
}

pub fn query_pair(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let pair_info: PairInfoRaw = PAIRS.load(deps.storage, &pair_key)?;
    pair_info.to_normal(deps.api)
}

pub fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some([
            start_after[0].to_raw(deps.api)?,
            start_after[1].to_raw(deps.api)?,
        ])
    } else {
        None
    };

    let pairs: Vec<PairInfo> = read_pairs(deps.storage, deps.api, start_after, limit)?;
    let resp = PairsResponse { pairs };

    Ok(resp)
}

pub fn query_native_token_decimal(
    deps: Deps,
    denom: String,
) -> StdResult<NativeTokenDecimalsResponse> {
    let decimals = ALLOW_NATIVE_TOKENS.load(deps.storage, denom.as_bytes())?;

    Ok(NativeTokenDecimalsResponse { decimals })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
