use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coin, from_binary, to_binary, Addr, Coin, CosmosMsg, StdError, SubMsg, Uint128, WasmMsg,
};

use crate::contract::{execute, instantiate, query};
use crate::operations::asset_into_swap_msg;
use terraswap::mock_querier::mock_dependencies;

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::pair::ExecuteMsg as PairExecuteMsg;
use terraswap::router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("terraswapfactory", config.terraswap_factory.as_str());
}

#[test]
fn execute_swap_operations() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"asset0002".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![],
        minimum_receive: None,
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "must provide operations"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0001".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0001".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0002".to_string(),
                },
            },
        ],
        minimum_receive: Some(Uint128::from(1000000u128)),
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                        ask_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0002".to_string(),
                        },
                    },
                    to: Some("addr0000".to_string()),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::AssertMinimumReceive {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset0002".to_string(),
                    },
                    prev_balance: Uint128::zero(),
                    minimum_receive: Uint128::from(1000000u128),
                    receiver: "addr0000".to_string(),
                })
                .unwrap(),
            })),
        ]
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: "asset0001".to_string(),
                    },
                },
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: "asset0001".to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
                SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: "asset0002".to_string(),
                    },
                },
            ],
            minimum_receive: None,
            to: Some("addr0002".to_string()),
        })
        .unwrap(),
    });

    let info = mock_info("asset0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        },
                        ask_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: SwapOperation::TerraSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0002".to_string(),
                        },
                    },
                    to: Some("addr0002".to_string()),
                })
                .unwrap(),
            }))
        ]
    );
}

#[test]
fn execute_swap_operation() {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_terraswap_pairs(&[(
        &"uusdasset0000".to_string(),
        &PairInfo {
            asset_infos: [
                AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
            ],
            contract_addr: "pair0000".to_string(),
            liquidity_token: "liquidity0000".to_string(),
            asset_decimals: [6u8, 6u8],
        },
    )]);
    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        [Coin {
            amount: Uint128::from(1000000u128),
            denom: "uusd".to_string(),
        }]
        .to_vec(),
    )]);

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        },
        to: None,
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(
            asset_into_swap_msg(
                deps.as_ref(),
                Addr::unchecked("pair0000"),
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128)
                },
                None,
                None
            )
            .unwrap()
        )],
    );

    // optional to address
    // swap_send
    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        },
        to: Some("addr0000".to_string()),
    };
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(
            asset_into_swap_msg(
                deps.as_ref(),
                Addr::unchecked("pair0000"),
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::from(1000000u128)
                },
                None,
                Some("addr0000".to_string())
            )
            .unwrap()
        )],
    );
    deps.querier.with_terraswap_pairs(&[(
        &"assetuusd".to_string(),
        &PairInfo {
            asset_infos: [
                AssetInfo::Token {
                    contract_addr: "asset".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            ],
            contract_addr: "pair0000".to_string(),
            liquidity_token: "liquidity0000".to_string(),
            asset_decimals: [6u8, 6u8],
        },
    )]);
    deps.querier.with_token_balances(&[(
        &"asset".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
        to: Some("addr0000".to_string()),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "pair0000".to_string(),
                amount: Uint128::from(1000000u128),
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset: Asset {
                        info: AssetInfo::Token {
                            contract_addr: "asset".to_string(),
                        },
                        amount: Uint128::from(1000000u128),
                    },
                    belief_price: None,
                    max_spread: None,
                    to: Some("addr0000".to_string()),
                })
                .unwrap()
            })
            .unwrap()
        }))]
    );
}

#[test]
fn query_buy_with_routes() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(1000000u128),
        operations: vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
        ],
    };

    deps.querier.with_terraswap_pairs(&[
        (
            &"ukrwasset0000".to_string(),
            &PairInfo {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                ],
                contract_addr: "pair0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_decimals: [6u8, 6u8],
            },
        ),
        (
            &"asset0000uluna".to_string(),
            &PairInfo {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                ],
                contract_addr: "pair0001".to_string(),
                liquidity_token: "liquidity0001".to_string(),
                asset_decimals: [6u8, 6u8],
            },
        ),
    ]);

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(1000000u128)
        }
    );
}

#[test]
fn query_reverse_routes_with_from_native() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let target_amount = 1000000u128;

    let info = mock_info("addr0000", &[coin(10000000, "ukrw")]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_balance(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        [Coin {
            amount: Uint128::from(1000000u128),
            denom: "ukrw".to_string(),
        }]
        .to_vec(),
    )]);

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = QueryMsg::ReverseSimulateSwapOperations {
        ask_amount: Uint128::from(target_amount),
        operations: vec![SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        }],
    };

    deps.querier.with_terraswap_pairs(&[
        (
            &"ukrwasset0000".to_string(),
            &PairInfo {
                contract_addr: "pair0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                ],
                asset_decimals: [8u8, 6u8],
            },
        ),
        (
            &"asset0000uluna".to_string(),
            &PairInfo {
                contract_addr: "pair0001".to_string(),
                liquidity_token: "liquidity0001".to_string(),
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                ],
                asset_decimals: [8u8, 6u8],
            },
        ),
    ]);

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(1000000u128),
        }
    );

    let offer_amount = res.amount;

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
        },
        to: None,
    };
    let info = mock_info("addr0", &[coin(offer_amount.u128(), "ukrw")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "pair0000".to_string(),
            funds: vec![coin(target_amount, "ukrw")],
            msg: to_binary(&PairExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                    amount: Uint128::from(target_amount),
                },
                belief_price: None,
                max_spread: None,
                to: None,
            })
            .unwrap(),
        })),],
    );
}

#[test]
fn query_reverse_routes_with_to_native() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        terraswap_factory: "terraswapfactory".to_string(),
    };

    let target_amount = 1000000u128;

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier.with_token_balances(&[
        (
            &"asset0000".to_string(),
            &[(&"pair0000".to_string(), &Uint128::from(1000000u128))],
        ),
        (
            &"asset0000".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
        ),
    ]);

    let msg = QueryMsg::ReverseSimulateSwapOperations {
        ask_amount: Uint128::from(target_amount),
        operations: vec![SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
        }],
    };

    deps.querier.with_terraswap_pairs(&[
        (
            &"ukrwasset0000".to_string(),
            &PairInfo {
                contract_addr: "pair0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                ],
                asset_decimals: [8u8, 6u8],
            },
        ),
        (
            &"asset0000uluna".to_string(),
            &PairInfo {
                contract_addr: "pair0001".to_string(),
                liquidity_token: "liquidity0001".to_string(),
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                ],
                asset_decimals: [8u8, 6u8],
            },
        ),
    ]);

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();

    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(target_amount),
        }
    );

    let offer_amount = res.amount;

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0".to_string(),
        amount: offer_amount,
        msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
            }],
            minimum_receive: None,
            to: None,
        })
        .unwrap(),
    });
    let info = mock_info("addr0", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                operation: SwapOperation::TerraSwap {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                },
                to: Some("addr0".to_string()),
            })
            .unwrap(),
        })),],
    );

    let msg = ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
        },
        to: None,
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "pair0000".to_string(),
                amount: Uint128::from(target_amount),
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset: Asset {
                        info: AssetInfo::Token {
                            contract_addr: "asset0000".to_string(),
                        },
                        amount: Uint128::from(target_amount),
                    },
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            })
            .unwrap(),
        }))],
    );
}

#[test]
fn assert_minimum_receive_native_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        &"addr0000".to_string(),
        [Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }]
        .to_vec(),
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: "addr0000".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: "addr0000".to_string(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "assertion failed; minimum receive amount: 1000001, swap amount: 1000000"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn assert_minimum_receive_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"token0000".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000u128))],
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Token {
            contract_addr: "token0000".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: "addr0000".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = ExecuteMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Token {
            contract_addr: "token0000".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: "addr0000".to_string(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "assertion failed; minimum receive amount: 1000001, swap amount: 1000000"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }
}
