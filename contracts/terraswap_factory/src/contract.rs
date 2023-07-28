#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;
use terraswap::querier::{query_balance, query_pair_info_from_pair};

use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    add_allow_native_token, pair_key, read_pairs, Config, TmpPairInfo, ALLOW_NATIVE_TOKENS, CONFIG,
    PAIRS, TMP_PAIR_INFO,
};

use protobuf::Message;
use terraswap::asset::{Asset, AssetInfo, AssetInfoRaw, PairInfo, PairInfoRaw};
use terraswap::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, NativeTokenDecimalsResponse,
    PairsResponse, QueryMsg,
};
use terraswap::pair::{
    ExecuteMsg as PairExecuteMsg, InstantiateMsg as PairInstantiateMsg,
    MigrateMsg as PairMigrateMsg,
};
use terraswap::util::migrate_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:terraswap-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const CREATE_PAIR_REPLY_ID: u64 = 1;

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
        ExecuteMsg::CreatePair { assets } => execute_create_pair(deps, env, info, assets),
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
    info: MessageInfo,
    assets: [Asset; 2],
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    if assets[0].info == assets[1].info {
        return Err(StdError::generic_err("same asset"));
    }

    let asset_1_decimal = match assets[0]
        .info
        .query_decimals(env.contract.address.clone(), &deps.querier)
    {
        Ok(decimal) => decimal,
        Err(_) => return Err(StdError::generic_err("asset1 is invalid")),
    };

    let asset_2_decimal = match assets[1]
        .info
        .query_decimals(env.contract.address.clone(), &deps.querier)
    {
        Ok(decimal) => decimal,
        Err(_) => return Err(StdError::generic_err("asset2 is invalid")),
    };

    let raw_assets = [assets[0].to_raw(deps.api)?, assets[1].to_raw(deps.api)?];

    let asset_infos = [assets[0].info.clone(), assets[1].info.clone()];
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
            assets: raw_assets,
            asset_decimals,
            sender: info.sender,
        },
    )?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &format!("{}-{}", assets[0].info, assets[1].info)),
        ])
        .add_submessage(SubMsg {
            id: CREATE_PAIR_REPLY_ID,
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
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    if msg.id != CREATE_PAIR_REPLY_ID {
        return Err(StdError::generic_err("invalid reply msg"));
    }

    let tmp_pair_info = TMP_PAIR_INFO.load(deps.storage)?;

    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(msg.result.unwrap().data.unwrap().as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let pair_contract = res.get_address();
    let pair_info = query_pair_info_from_pair(&deps.querier, Addr::unchecked(pair_contract))?;

    let raw_infos = [
        tmp_pair_info.assets[0].info.clone(),
        tmp_pair_info.assets[1].info.clone(),
    ];

    PAIRS.save(
        deps.storage,
        &tmp_pair_info.pair_key,
        &PairInfoRaw {
            liquidity_token: deps.api.addr_canonicalize(&pair_info.liquidity_token)?,
            contract_addr: deps.api.addr_canonicalize(pair_contract)?,
            asset_infos: raw_infos,
            asset_decimals: tmp_pair_info.asset_decimals,
        },
    )?;

    let mut messages: Vec<CosmosMsg> = vec![];
    if !tmp_pair_info.assets[0].amount.is_zero() || !tmp_pair_info.assets[1].amount.is_zero() {
        let assets = [
            tmp_pair_info.assets[0].to_normal(deps.api)?,
            tmp_pair_info.assets[1].to_normal(deps.api)?,
        ];
        let mut funds: Vec<Coin> = vec![];
        for asset in tmp_pair_info.assets.iter() {
            if let AssetInfoRaw::NativeToken { denom, .. } = &asset.info {
                funds.push(coin(asset.amount.u128(), denom.to_string()));
            } else if let AssetInfoRaw::Token { contract_addr } = &asset.info {
                let contract_addr = deps.api.addr_humanize(contract_addr)?.to_string();
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                        spender: pair_contract.to_string(),
                        amount: asset.amount,
                        expires: None,
                    })?,
                    funds: vec![],
                }));
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: tmp_pair_info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: asset.amount,
                    })?,
                    funds: vec![],
                }));
            }
        }

        funds.sort_by(|a, b| a.denom.cmp(&b.denom));
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_contract.to_string(),
            msg: to_binary(&PairExecuteMsg::ProvideLiquidity {
                assets,
                receiver: Some(tmp_pair_info.sender.to_string()),
                deadline: None,
                slippage_tolerance: None,
            })?,
            funds,
        }));
    }

    Ok(Response::new()
        .add_attributes(vec![
            ("pair_contract_addr", pair_contract),
            ("liquidity_token_addr", pair_info.liquidity_token.as_str()),
        ])
        .add_messages(messages))
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

const TARGET_CONTRACT_VERSION: &str = "0.1.0";
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    migrate_version(
        deps,
        TARGET_CONTRACT_VERSION,
        CONTRACT_NAME,
        CONTRACT_VERSION,
    )?;

    Ok(Response::default())
}
