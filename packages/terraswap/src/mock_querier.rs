use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, Binary, Coin, ContractResult, Decimal, OwnedDeps,
    Querier, QuerierResult, QueryRequest, StdError, SystemError, SystemResult, Uint128, WasmQuery,
};
use protobuf::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic;

use crate::asset::{Asset, AssetInfo, AssetInfoRaw, PairInfo, PairInfoRaw};
use crate::pair::{ReverseSimulationResponse, SimulationResponse};
use crate::query::{
    DenomTrace, QueryActivesResponse, QueryDenomTraceRequest, QueryDenomTraceResponse,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use terra_cosmwasm::{
    SwapResponse, TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair { asset_infos: [AssetInfo; 2] },
    Simulation { offer_asset: Asset },
    ReverseSimulation { ask_asset: Asset },
}

use std::iter::FromIterator;

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    token_querier: TokenQuerier,
    tax_querier: TaxQuerier,
    terraswap_factory_querier: TerraswapFactoryQuerier,
    oracle_querier: OracleQuerier,
    ibc_querier: IbcQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
}

#[derive(Clone, Default)]
pub struct TerraswapFactoryQuerier {
    pairs: HashMap<String, PairInfo>,
}

impl TerraswapFactoryQuerier {
    pub fn new(pairs: &[(&String, &PairInfo)]) -> Self {
        TerraswapFactoryQuerier {
            pairs: pairs_to_map(pairs),
        }
    }
}

pub(crate) fn pairs_to_map(pairs: &[(&String, &PairInfo)]) -> HashMap<String, PairInfo> {
    let mut pairs_map: HashMap<String, PairInfo> = HashMap::new();
    for (key, pair) in pairs.iter() {
        let mut sort_key: Vec<char> = key.chars().collect();
        sort_key.sort_by(|a, b| b.cmp(a));
        pairs_map.insert(String::from_iter(sort_key.iter()), (**pair).clone());
    }
    pairs_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

#[derive(Clone, Default)]
pub struct OracleQuerier {
    actives: Vec<String>,
}

impl OracleQuerier {
    pub fn new(actives: &[String]) -> Self {
        OracleQuerier {
            actives: actives.to_vec(),
        }
    }
}

#[derive(Clone, Default)]
pub struct IbcQuerier {
    denom_traces: HashMap<String, DenomTrace>,
}

impl IbcQuerier {
    pub fn new(denom_treaces: &[(&String, (&String, &String))]) -> Self {
        let mut denom_traces_map: HashMap<String, DenomTrace> = HashMap::new();
        for (hash, denom_trace) in denom_treaces.iter() {
            let mut proto_denom_trace = DenomTrace::new();
            proto_denom_trace.set_path(denom_trace.0.to_string());
            proto_denom_trace.set_base_denom(denom_trace.1.to_string());

            denom_traces_map.insert(hash.to_string(), proto_denom_trace);
        }
        IbcQuerier {
            denom_traces: denom_traces_map,
        }
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                if &TerraRoute::Treasury == route {
                    match query_data {
                        TerraQuery::TaxRate {} => {
                            let res = TaxRateResponse {
                                rate: self.tax_querier.rate,
                            };
                            SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                        }
                        TerraQuery::TaxCap { denom } => {
                            let cap = self
                                .tax_querier
                                .caps
                                .get(denom)
                                .copied()
                                .unwrap_or_default();
                            let res = TaxCapResponse { cap };
                            SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else if route == &TerraRoute::Market {
                    match query_data {
                        TerraQuery::Swap {
                            offer_coin,
                            ask_denom: _,
                        } => {
                            let res = SwapResponse {
                                receive: offer_coin.clone(),
                            };
                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => match from_binary(msg) {
                Ok(QueryMsg::Pair { asset_infos }) => {
                    let key = [asset_infos[0].to_string(), asset_infos[1].to_string()].join("");
                    let mut sort_key: Vec<char> = key.chars().collect();
                    sort_key.sort_by(|a, b| b.cmp(a));
                    match self
                        .terraswap_factory_querier
                        .pairs
                        .get(&String::from_iter(sort_key.iter()))
                    {
                        Some(v) => SystemResult::Ok(ContractResult::Ok(to_binary(v).unwrap())),
                        None => SystemResult::Err(SystemError::InvalidRequest {
                            error: "No pair info exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    }
                }
                Ok(QueryMsg::Simulation { offer_asset }) => {
                    SystemResult::Ok(ContractResult::from(to_binary(&SimulationResponse {
                        return_amount: offer_asset.amount,
                        commission_amount: Uint128::zero(),
                        spread_amount: Uint128::zero(),
                    })))
                }
                Ok(QueryMsg::ReverseSimulation { ask_asset }) => SystemResult::Ok(
                    ContractResult::from(to_binary(&ReverseSimulationResponse {
                        offer_amount: ask_asset.amount,
                        commission_amount: Uint128::zero(),
                        spread_amount: Uint128::zero(),
                    })),
                ),
                _ => match from_binary(msg).unwrap() {
                    Cw20QueryMsg::TokenInfo {} => {
                        let balances: &HashMap<String, Uint128> =
                            match self.token_querier.balances.get(contract_addr) {
                                Some(balances) => balances,
                                None => {
                                    return SystemResult::Err(SystemError::InvalidRequest {
                                        error: format!(
                                            "No balance info exists for the contract {}",
                                            contract_addr
                                        ),
                                        request: msg.as_slice().into(),
                                    })
                                }
                            };

                        let mut total_supply = Uint128::zero();

                        for balance in balances {
                            total_supply += *balance.1;
                        }

                        SystemResult::Ok(ContractResult::Ok(
                            to_binary(&TokenInfoResponse {
                                name: "mAAPL".to_string(),
                                symbol: "mAAPL".to_string(),
                                decimals: 8,
                                total_supply,
                            })
                            .unwrap(),
                        ))
                    }
                    Cw20QueryMsg::Balance { address } => {
                        let balances: &HashMap<String, Uint128> =
                            match self.token_querier.balances.get(contract_addr) {
                                Some(balances) => balances,
                                None => {
                                    return SystemResult::Err(SystemError::InvalidRequest {
                                        error: format!(
                                            "No balance info exists for the contract {}",
                                            contract_addr
                                        ),
                                        request: msg.as_slice().into(),
                                    })
                                }
                            };

                        let balance = match balances.get(&address) {
                            Some(v) => *v,
                            None => {
                                return SystemResult::Ok(ContractResult::Ok(
                                    to_binary(&Cw20BalanceResponse {
                                        balance: Uint128::zero(),
                                    })
                                    .unwrap(),
                                ));
                            }
                        };

                        SystemResult::Ok(ContractResult::Ok(
                            to_binary(&Cw20BalanceResponse { balance }).unwrap(),
                        ))
                    }
                    _ => panic!("DO NOT ENTER HERE"),
                },
            },
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                let key: &[u8] = key.as_slice();
                let prefix_pair_info = Binary::from("pair_info".as_bytes());

                if key == prefix_pair_info.as_slice() {
                    let pair_info: PairInfo =
                        match self.terraswap_factory_querier.pairs.get(contract_addr) {
                            Some(v) => v.clone(),
                            None => {
                                return SystemResult::Err(SystemError::InvalidRequest {
                                    error: format!("PairInfo is not found for {}", contract_addr),
                                    request: key.into(),
                                })
                            }
                        };

                    let api: MockApi = MockApi::default();
                    SystemResult::Ok(ContractResult::from(to_binary(&PairInfoRaw {
                        contract_addr: api
                            .addr_canonicalize(pair_info.contract_addr.as_str())
                            .unwrap(),
                        liquidity_token: api
                            .addr_canonicalize(pair_info.liquidity_token.as_str())
                            .unwrap(),
                        asset_infos: [
                            AssetInfoRaw::NativeToken {
                                denom: "uusd".to_string(),
                            },
                            AssetInfoRaw::NativeToken {
                                denom: "uusd".to_string(),
                            },
                        ],
                    })))
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Stargate { path, data } => match path.as_str() {
                "/terra.oracle.v1beta1.Query/Actives" => {
                    let mut res: QueryActivesResponse = QueryActivesResponse::new();
                    res.set_actives(self.oracle_querier.actives.to_vec().into());
                    SystemResult::Ok(ContractResult::Ok(Binary::from(
                        res.write_to_bytes().unwrap().to_vec(),
                    )))
                }
                "/ibc.applications.transfer.v1.Query/DenomTrace" => {
                    let req: QueryDenomTraceRequest = Message::parse_from_bytes(data.as_slice())
                        .map_err(|_| {
                            StdError::parse_err("QueryDenomTraceRequest", "failed to parse data")
                        })
                        .unwrap();
                    let denom_trace = self.ibc_querier.denom_traces.get(&req.hash).unwrap();
                    let mut proto_denom_trace = DenomTrace::new();
                    proto_denom_trace.set_path(denom_trace.path.to_string());
                    proto_denom_trace.set_base_denom(denom_trace.base_denom.to_string());

                    let mut res = QueryDenomTraceResponse::new();
                    res.set_denom_trace(proto_denom_trace);

                    SystemResult::Ok(ContractResult::Ok(Binary::from(
                        res.write_to_bytes().unwrap(),
                    )))
                }
                _ => panic!(""),
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            tax_querier: TaxQuerier::default(),
            terraswap_factory_querier: TerraswapFactoryQuerier::default(),
            oracle_querier: OracleQuerier::default(),
            ibc_querier: IbcQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    // configure the token owner mock querier
    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    // configure the terraswap pair
    pub fn with_terraswap_pairs(&mut self, pairs: &[(&String, &PairInfo)]) {
        self.terraswap_factory_querier = TerraswapFactoryQuerier::new(pairs);
    }

    pub fn with_balance(&mut self, balances: &[(&String, Vec<Coin>)]) {
        for (addr, balance) in balances {
            self.base.update_balance(addr.to_string(), balance.clone());
        }
    }

    pub fn with_active_denoms(&mut self, actives: &[String]) {
        self.oracle_querier = OracleQuerier::new(actives);
    }

    pub fn with_ibc_denom_traces(&mut self, denom_traces: &[(&String, (&String, &String))]) {
        self.ibc_querier = IbcQuerier::new(denom_traces);
    }
}
