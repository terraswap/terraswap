use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::factory::QueryMsg as FactoryQueryMsg;
use crate::pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse, SimulationResponse};
use crate::query::{QueryActivesResponse, QueryDenomTraceRequest, QueryDenomTraceResponse};

use cosmwasm_std::{
    to_binary, to_vec, Addr, AllBalanceResponse, BalanceResponse, BankQuery, Binary, Coin, Empty,
    QuerierWrapper, QueryRequest, StdError, StdResult, Uint128, WasmQuery,
};

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use protobuf::Message;
use terra_cosmwasm::TerraQueryWrapper;

pub fn query_balance(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    // load price form the oracle
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

pub fn query_all_balances(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    account_addr: Addr,
) -> StdResult<Vec<Coin>> {
    // load price form the oracle
    let all_balances: AllBalanceResponse =
        querier.query(&QueryRequest::Bank(BankQuery::AllBalances {
            address: account_addr.to_string(),
        }))?;
    Ok(all_balances.amount)
}

pub fn query_token_balance(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;

    // load balance form the token contract
    Ok(res.balance)
}

pub fn query_supply(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    contract_addr: Addr,
) -> StdResult<Uint128> {
    // load price form the oracle
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(token_info.total_supply)
}

pub fn query_decimals(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    contract_addr: Addr,
) -> StdResult<u8> {
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(token_info.decimals)
}

pub fn query_pair_info(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    factory_contract: Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        })?,
    }))
}

pub fn simulate(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    pair_contract: Addr,
    offer_asset: &Asset,
) -> StdResult<SimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::Simulation {
            offer_asset: offer_asset.clone(),
        })?,
    }))
}

pub fn reverse_simulate(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    pair_contract: Addr,
    ask_asset: &Asset,
) -> StdResult<ReverseSimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::ReverseSimulation {
            ask_asset: ask_asset.clone(),
        })?,
    }))
}

pub fn query_active_denoms(querier: &QuerierWrapper<TerraQueryWrapper>) -> StdResult<Vec<String>> {
    let req = to_vec::<QueryRequest<Empty>>(&QueryRequest::Stargate {
        path: "/terra.oracle.v1beta1.Query/Actives".to_string(),
        data: Binary::from(vec![]),
    })
    .unwrap();

    let res: Binary = querier.raw_query(req.as_slice()).unwrap().unwrap();

    let res: QueryActivesResponse = Message::parse_from_bytes(res.as_slice())
        .map_err(|_| StdError::parse_err("QueryActivesResponse", "failed to parse data"))?;

    Ok(res.actives.to_vec())
}

pub fn query_ibc_denom(
    querier: &QuerierWrapper<TerraQueryWrapper>,
    denom: String,
) -> StdResult<String> {
    let denoms: Vec<&str> = denom.split('/').collect();
    if denoms.len() != 2 || denoms[0] != "ibc" {
        return Err(StdError::generic_err("invalid ibc denom"));
    }

    let mut req = QueryDenomTraceRequest::new();
    req.set_hash(denoms[1].to_string());
    let query_denom_trace_req: Binary = Binary::from(req.write_to_bytes().unwrap());

    let req = to_vec::<QueryRequest<Empty>>(&QueryRequest::Stargate {
        path: "/ibc.applications.transfer.v1.Query/DenomTrace".to_string(),
        data: query_denom_trace_req,
    })
    .unwrap();

    let res: Binary = querier.raw_query(req.as_slice()).unwrap().unwrap();

    let res: QueryDenomTraceResponse = Message::parse_from_bytes(res.as_slice())
        .map_err(|_| StdError::parse_err("QueryDenomTraceResponse", "failed to parse data"))?;

    Ok(res.denom_trace.unwrap().base_denom)
}
