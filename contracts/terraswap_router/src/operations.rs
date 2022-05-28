use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult, WasmMsg,
};

use crate::state::{Config, CONFIG};

use cw20::Cw20ExecuteMsg;
use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::pair::ExecuteMsg as PairExecuteMsg;
use terraswap::querier::{query_balance, query_pair_info, query_token_balance};
use terraswap::router::SwapOperation;

/// Execute swap operation
/// swap all offer asset to ask asset
pub fn execute_swap_operation(
    deps: DepsMut<Empty>,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
    to: Option<String>,
) -> StdResult<Response> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err("unauthorized"));
    }

    let messages: Vec<CosmosMsg> = match operation {
        SwapOperation::TerraSwap {
            offer_asset_info,
            ask_asset_info,
        } => {
            let config: Config = CONFIG.load(deps.as_ref().storage)?;
            let terraswap_factory = deps.api.addr_humanize(&config.terraswap_factory)?;
            let pair_info: PairInfo = query_pair_info(
                &deps.querier,
                terraswap_factory,
                &[offer_asset_info.clone(), ask_asset_info],
            )?;

            let amount = match offer_asset_info.clone() {
                AssetInfo::NativeToken { denom } => {
                    query_balance(&deps.querier, env.contract.address, denom)?
                }
                AssetInfo::Token { contract_addr } => query_token_balance(
                    &deps.querier,
                    deps.api.addr_validate(contract_addr.as_str())?,
                    env.contract.address,
                )?,
            };
            let offer_asset: Asset = Asset {
                info: offer_asset_info,
                amount,
            };

            vec![asset_into_swap_msg(
                deps.as_ref(),
                Addr::unchecked(pair_info.contract_addr),
                offer_asset,
                None,
                to,
            )?]
        }
    };

    Ok(Response::new().add_messages(messages))
}

pub fn asset_into_swap_msg(
    _deps: Deps<Empty>,
    pair_contract: Addr,
    offer_asset: Asset,
    max_spread: Option<Decimal>,
    to: Option<String>,
) -> StdResult<CosmosMsg> {
    match offer_asset.info.clone() {
        AssetInfo::NativeToken { denom } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_contract.to_string(),
            funds: vec![Coin {
                denom,
                amount: offer_asset.amount,
            }],
            msg: to_binary(&PairExecuteMsg::Swap {
                offer_asset,
                belief_price: None,
                max_spread,
                to,
            })?,
        })),
        AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: pair_contract.to_string(),
                amount: offer_asset.amount,
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset,
                    belief_price: None,
                    max_spread,
                    to,
                })?,
            })?,
        })),
    }
}
