use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::factory::QueryMsg as FactoryQueryMsg;
use crate::pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse, SimulationResponse};
use crate::query::{
    QueryDenomMetadataRequest, QueryDenomMetadataResponse, QueryDenomTraceRequest,
    QueryDenomTraceResponse,
};

use cosmwasm_std::{
    to_binary, to_vec, Addr, AllBalanceResponse, BalanceResponse, BankQuery, Binary, Coin, Empty,
    QuerierWrapper, QueryRequest, StdError, StdResult, Uint128, WasmQuery,
};

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use protobuf::Message;

pub fn query_balance(
    querier: &QuerierWrapper<Empty>,
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
    querier: &QuerierWrapper<Empty>,
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
    querier: &QuerierWrapper<Empty>,
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

pub fn query_supply(querier: &QuerierWrapper<Empty>, contract_addr: Addr) -> StdResult<Uint128> {
    // load price form the oracle
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(token_info.total_supply)
}

pub fn query_decimals(querier: &QuerierWrapper<Empty>, contract_addr: Addr) -> StdResult<u8> {
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(token_info.decimals)
}

pub fn query_pair_info(
    querier: &QuerierWrapper<Empty>,
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
    querier: &QuerierWrapper<Empty>,
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
    querier: &QuerierWrapper<Empty>,
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

pub fn query_ibc_denom(querier: &QuerierWrapper<Empty>, denom: String) -> StdResult<String> {
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

pub fn query_denom_info(querier: &QuerierWrapper<Empty>, denom: String) -> StdResult<String> {
    let mut req = QueryDenomMetadataRequest::new();
    req.set_denom(denom);
    let binary_req: Binary = Binary::from(req.write_to_bytes().unwrap());

    let req = to_vec::<QueryRequest<Empty>>(&QueryRequest::Stargate {
        path: "/cosmos.bank.v1beta1.Query/DenomMetadata".to_string(),
        data: binary_req,
    })
    .unwrap();

    let res: Binary = querier.raw_query(req.as_slice()).unwrap().unwrap();
    let res: QueryDenomMetadataResponse = Message::parse_from_bytes(res.as_slice())
        .map_err(|_| StdError::parse_err("QueryDenomMetadataResponse", "failed to parse data"))?;

    Ok(res.get_metadata().base.to_string())
}
