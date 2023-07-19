use crate::contract::{execute, instantiate, query, reply};
use terraswap::mock_querier::{mock_dependencies, WasmMockQuerier};

use crate::state::{pair_key, TmpPairInfo, TMP_PAIR_INFO};

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coin, coins, from_binary, to_binary, Addr, CosmosMsg, OwnedDeps, Reply, ReplyOn,
    Response, StdError, SubMsg, SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, NativeTokenDecimalsResponse, QueryMsg,
};
use terraswap::pair::{
    ExecuteMsg as PairExecuteMsg, InstantiateMsg as PairInstantiateMsg,
    MigrateMsg as PairMigrateMsg,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("addr0000".to_string(), config_res.owner);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("addr0001".to_string()),
        pair_code_id: None,
        token_code_id: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("addr0001".to_string(), config_res.owner);

    // update left items
    let env = mock_env();
    let info = mock_info("addr0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: Some(100u64),
        token_code_id: Some(200u64),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(200u64, config_res.token_code_id);
    assert_eq!(100u64, config_res.pair_code_id);
    assert_eq!("addr0001".to_string(), config_res.owner);

    // Unauthorized err
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        pair_code_id: None,
        token_code_id: None,
    };

    let res = execute(deps.as_mut(), env, info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Must return unauthorized error"),
    }
}

fn init(
    mut deps: OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let msg = InstantiateMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    deps.querier.with_token_balances(&[(
        &"asset0001".to_string(),
        &[(&"addr0000".to_string(), &Uint128::zero())],
    )]);
    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

    deps
}

#[test]
fn create_pair() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);
    deps.querier
        .with_terraswap_factory(&[], &[("uusd".to_string(), 6u8)]);
    let assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
            amount: Uint128::zero(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        assets: assets.clone(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "uusd-asset0001")
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uusd".to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: "asset0001".to_string(),
                        }
                    ],
                    token_code_id: 123u64,
                    asset_decimals: [6u8, 8u8]
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "pair".to_string(),
                admin: Some(MOCK_CONTRACT_ADDR.to_string()),
            }
            .into()
        },]
    );

    let raw_assets = [
        assets[0].to_raw(deps.as_ref().api).unwrap(),
        assets[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    let raw_infos = [
        assets[0].info.to_raw(deps.as_ref().api).unwrap(),
        assets[1].info.to_raw(deps.as_ref().api).unwrap(),
    ];

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            assets: raw_assets,
            pair_key: pair_key(&raw_infos),
            sender: Addr::unchecked("addr0000"),
            asset_decimals: [6u8, 8u8]
        }
    );
}

#[test]
fn create_pair_native_token_and_ibc_token() {
    let mut deps = mock_dependencies(&[
        coin(10u128, "uusd".to_string()),
        coin(10u128, "ibc/HASH".to_string()),
    ]);
    deps = init(deps);
    deps.querier.with_terraswap_factory(
        &[],
        &[("uusd".to_string(), 6u8), ("ibc/HASH".to_string(), 6u8)],
    );

    let assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "ibc/HASH".to_string(),
            },
            amount: Uint128::zero(),
        },
    ];

    let msg = ExecuteMsg::CreatePair {
        assets: assets.clone(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "create_pair"), attr("pair", "uusd-ibc/HASH")]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg {
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success,
            msg: WasmMsg::Instantiate {
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uusd".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: "ibc/HASH".to_string(),
                        }
                    ],
                    token_code_id: 123u64,
                    asset_decimals: [6u8, 6u8]
                })
                .unwrap(),
                code_id: 321u64,
                funds: vec![],
                label: "pair".to_string(),
                admin: Some(MOCK_CONTRACT_ADDR.to_string()),
            }
            .into()
        },]
    );

    let raw_assets = [
        assets[0].to_raw(deps.as_ref().api).unwrap(),
        assets[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    let raw_infos = [
        assets[0].info.to_raw(deps.as_ref().api).unwrap(),
        assets[1].info.to_raw(deps.as_ref().api).unwrap(),
    ];

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage).unwrap(),
        TmpPairInfo {
            assets: raw_assets,
            pair_key: pair_key(&raw_infos),
            sender: Addr::unchecked("addr0000"),
            asset_decimals: [6u8, 6u8]
        }
    );
}

#[test]
fn fail_to_create_same_pair() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);

    let assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::zero(),
        },
    ];

    let msg = ExecuteMsg::CreatePair { assets };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    match execute(deps.as_mut(), env, info, msg).unwrap_err() {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "same asset".to_string()),
        _ => panic!("Must return generic error"),
    }
}

#[test]
fn fail_to_create_pair_with_unknown_denom() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);

    deps.querier
        .with_terraswap_factory(&[], &[("uusd".to_string(), 6u8)]);

    let assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uxxx".to_string(),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::zero(),
        },
    ];

    let msg = ExecuteMsg::CreatePair { assets };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    match execute(deps.as_mut(), env, info, msg).unwrap_err() {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "asset1 is invalid".to_string()),
        _ => panic!("Must return generic error"),
    }
}

#[test]
fn fail_to_create_pair_with_unknown_token() {
    let mut deps = mock_dependencies(&[coin(10u128, "uusd".to_string())]);
    deps = init(deps);

    deps.querier
        .with_terraswap_factory(&[], &[("uluna".to_string(), 6u8)]);

    let assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: "terra123".to_string(),
            },
            amount: Uint128::zero(),
        },
    ];

    let msg = ExecuteMsg::CreatePair { assets };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    match execute(deps.as_mut(), env, info, msg).unwrap_err() {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "asset2 is invalid".to_string()),
        _ => panic!("Must return generic error"),
    }
}

#[test]
fn reply_only_create_pair() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[
            (&"asset0000".to_string(), &Uint128::from(100u128)),
            (&"asset0001".to_string(), &Uint128::from(100u128)),
        ],
    )]);

    let assets = [
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
            amount: Uint128::zero(),
        },
    ];

    let raw_assets = [
        assets[0].to_raw(deps.as_ref().api).unwrap(),
        assets[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    let raw_infos = [
        assets[0].info.to_raw(deps.as_ref().api).unwrap(),
        assets[1].info.to_raw(deps.as_ref().api).unwrap(),
    ];

    let pair_key = pair_key(&raw_infos);
    TMP_PAIR_INFO
        .save(
            &mut deps.storage,
            &TmpPairInfo {
                assets: raw_assets,
                pair_key,
                sender: Addr::unchecked("addr0000"),
                asset_decimals: [8u8, 8u8],
            },
        )
        .unwrap();

    let reply_msg = Reply {
        id: 1,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(vec![10, 4, 48, 48, 48, 48].into()),
        }),
    };

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    // register terraswap pair querier
    deps.querier.with_terraswap_factory(
        &[(
            &"0000".to_string(),
            &PairInfo {
                asset_infos,
                contract_addr: "0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_decimals: [8u8, 8u8],
            },
        )],
        &[],
    );

    let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    assert_eq!(res.messages.len(), 0);
    assert_eq!(res.attributes[0], attr("pair_contract_addr", "0000"));
    assert_eq!(
        res.attributes[1],
        attr("liquidity_token_addr", "liquidity0000")
    );
}

#[test]
fn reply_create_pair_with_provide() {
    let mut deps = mock_dependencies(&[]);

    deps.querier
        .with_balance(&[(&MOCK_CONTRACT_ADDR.to_string(), coins(100u128, "uluna"))]);

    deps.querier.with_token_balances(&[(
        &"pair0000".to_string(),
        &[(&"asset0000".to_string(), &Uint128::from(100u128))],
    )]);

    let assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::from(100u128),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            amount: Uint128::from(100u128),
        },
    ];

    let raw_assets = [
        assets[0].to_raw(deps.as_ref().api).unwrap(),
        assets[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    let raw_infos = [
        assets[0].info.to_raw(deps.as_ref().api).unwrap(),
        assets[1].info.to_raw(deps.as_ref().api).unwrap(),
    ];

    let pair_key = pair_key(&raw_infos);
    TMP_PAIR_INFO
        .save(
            &mut deps.storage,
            &TmpPairInfo {
                assets: raw_assets,
                pair_key,
                sender: Addr::unchecked("addr0000"),
                asset_decimals: [18u8, 8u8],
            },
        )
        .unwrap();

    let reply_msg = Reply {
        id: 1,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: Some(vec![10, 8, 112, 97, 105, 114, 48, 48, 48, 48].into()),
        }),
    };

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
    ];

    // register terraswap pair querier
    deps.querier.with_terraswap_factory(
        &[(
            &"pair0000".to_string(),
            &PairInfo {
                asset_infos,
                contract_addr: "pair0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_decimals: [18u8, 8u8],
            },
        )],
        &[("uluna".to_string(), 18u8)],
    );

    let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: "pair0000".to_string(),
                    amount: Uint128::from(100u128),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    amount: Uint128::from(100u128),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(
        res.messages[2],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "pair0000".to_string(),
                msg: to_binary(&PairExecuteMsg::ProvideLiquidity {
                    assets,
                    receiver: Some("addr0000".to_string()),
                    deadline: None,
                    slippage_tolerance: None,
                })
                .unwrap(),
                funds: coins(100u128, "uluna".to_string()),
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    assert_eq!(res.attributes[0], attr("pair_contract_addr", "pair0000"));
    assert_eq!(
        res.attributes[1],
        attr("liquidity_token_addr", "liquidity0000")
    );
}

#[test]
fn failed_reply_with_unknown_id() {
    let mut deps = mock_dependencies(&[]);

    let res = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 9,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(vec![].into()),
            }),
        },
    );

    assert_eq!(res, Err(StdError::generic_err("invalid reply msg")))
}

#[test]
fn normal_add_allow_native_token() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("addr0000", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg).unwrap(),
        Response::new().add_attributes(vec![
            ("action", "add_allow_native_token"),
            ("denom", "uluna"),
            ("decimals", "6"),
        ])
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NativeTokenDecimals {
            denom: "uluna".to_string(),
        },
    )
    .unwrap();
    let res: NativeTokenDecimalsResponse = from_binary(&res).unwrap();
    assert_eq!(6u8, res.decimals)
}

#[test]
fn failed_add_allow_native_token_with_non_admin() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("noadmin", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg),
        Err(StdError::generic_err("unauthorized"))
    );
}

#[test]
fn failed_add_allow_native_token_with_zero_factory_balance() {
    let mut deps = mock_dependencies(&[coin(0u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("addr0000", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg),
        Err(StdError::generic_err(
            "a balance greater than zero is required by the factory for verification",
        ))
    );
}

#[test]
fn append_add_allow_native_token_with_already_exist_token() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),

        decimals: 6u8,
    };

    let info = mock_info("addr0000", &[]);

    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NativeTokenDecimals {
            denom: "uluna".to_string(),
        },
    )
    .unwrap();
    let res: NativeTokenDecimalsResponse = from_binary(&res).unwrap();
    assert_eq!(6u8, res.decimals);

    let msg = ExecuteMsg::AddNativeTokenDecimals {
        denom: "uluna".to_string(),
        decimals: 7u8,
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NativeTokenDecimals {
            denom: "uluna".to_string(),
        },
    )
    .unwrap();
    let res: NativeTokenDecimalsResponse = from_binary(&res).unwrap();
    assert_eq!(7u8, res.decimals)
}

#[test]
fn normal_migrate_pair() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::MigratePair {
        code_id: Some(123u64),
        contract: "contract0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg).unwrap(),
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: "contract0000".to_string(),
            new_code_id: 123u64,
            msg: to_binary(&PairMigrateMsg {}).unwrap(),
        })),
    );
}

#[test]
fn normal_migrate_pair_with_none_code_id_will_config_code_id() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::MigratePair {
        code_id: None,
        contract: "contract0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg).unwrap(),
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: "contract0000".to_string(),
            new_code_id: 321u64,
            msg: to_binary(&PairMigrateMsg {}).unwrap(),
        })),
    );
}

#[test]
fn failed_migrate_pair_with_no_admin() {
    let mut deps = mock_dependencies(&[coin(1u128, "uluna".to_string())]);
    deps = init(deps);

    let msg = ExecuteMsg::MigratePair {
        code_id: None,
        contract: "contract0000".to_string(),
    };

    let info = mock_info("noadmin", &[]);

    assert_eq!(
        execute(deps.as_mut(), mock_env(), info, msg),
        Err(StdError::generic_err("unauthorized")),
    );
}
