use crate::asset::{Asset, AssetInfo, AssetInfoRaw, AssetRaw, PairInfo};
use crate::mock_querier::mock_dependencies;
use crate::querier::{
    query_all_balances, query_balance, query_pair_info, query_token_balance, query_token_info,
};

use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{
    coin, to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, MessageInfo, StdError, SubMsg, Uint128,
    WasmMsg,
};
use cw20::Cw20ExecuteMsg;

#[test]
fn token_balance_querier() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"liquidity0000".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
    )]);

    assert_eq!(
        Uint128::from(123u128),
        query_token_balance(
            &deps.as_ref().querier,
            Addr::unchecked("liquidity0000"),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
        )
        .unwrap()
    );
}

#[test]
fn balance_querier() {
    let deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(200u128),
    }]);

    assert_eq!(
        query_balance(
            &deps.as_ref().querier,
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            "uusd".to_string()
        )
        .unwrap(),
        Uint128::from(200u128)
    );
}

#[test]
fn all_balances_querier() {
    let deps = mock_dependencies(&[
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        },
        Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::from(300u128),
        },
    ]);

    assert_eq!(
        query_all_balances(&deps.as_ref().querier, Addr::unchecked(MOCK_CONTRACT_ADDR),).unwrap(),
        vec![
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(300u128),
            }
        ]
    );
}

#[test]
fn supply_querier() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"liquidity0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    assert_eq!(
        query_token_info(&deps.as_ref().querier, Addr::unchecked("liquidity0000"))
            .unwrap()
            .total_supply,
        Uint128::from(492u128)
    )
}

#[test]
fn test_asset_info() {
    let token_info: AssetInfo = AssetInfo::Token {
        contract_addr: "asset0000".to_string(),
    };
    let native_token_info: AssetInfo = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    assert!(!token_info.equal(&native_token_info));

    assert!(!token_info.equal(&AssetInfo::Token {
        contract_addr: "asset0001".to_string(),
    }));

    assert!(token_info.equal(&AssetInfo::Token {
        contract_addr: "asset0000".to_string(),
    }));

    assert!(native_token_info.is_native_token());
    assert!(!token_info.is_native_token());

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(123u128),
    }]);
    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    assert_eq!(
        token_info
            .query_pool(
                &deps.as_ref().querier,
                deps.as_ref().api,
                Addr::unchecked(MOCK_CONTRACT_ADDR)
            )
            .unwrap(),
        Uint128::from(123u128)
    );
    assert_eq!(
        native_token_info
            .query_pool(
                &deps.as_ref().querier,
                deps.as_ref().api,
                Addr::unchecked(MOCK_CONTRACT_ADDR)
            )
            .unwrap(),
        Uint128::from(123u128)
    );
}

#[test]
fn test_asset() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(123u128),
    }]);

    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    let token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
    };

    let native_token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    };

    assert_eq!(
        token_asset
            .clone()
            .into_msg(Addr::unchecked("addr0000"))
            .unwrap(),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(123123u128),
            })
            .unwrap(),
            funds: vec![],
        })
    );

    assert_eq!(
        token_asset
            .into_submsg(Addr::unchecked("addr0000"))
            .unwrap(),
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(123123u128),
            })
            .unwrap(),
            funds: vec![],
        }))
    );

    assert_eq!(
        native_token_asset
            .into_msg(Addr::unchecked("addr0000"))
            .unwrap(),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(123123u128),
            }]
        })
    );
}

#[test]
fn test_assert_sent_native_token_balance() {
    // zero asset
    let message_info = MessageInfo {
        funds: vec![],
        sender: Addr::unchecked("addr0000"),
    };

    let zero_asset = Asset {
        amount: Uint128::zero(),
        info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
    };

    assert_eq!(
        zero_asset.assert_sent_native_token_balance(&message_info),
        Ok(())
    );

    // invalid message_info
    let message_info = MessageInfo {
        funds: vec![coin(123, "uluna")],
        sender: Addr::unchecked("addr0000"),
    };

    let invalid_amount_asset = Asset {
        amount: Uint128::from(1u8),
        info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
    };

    assert_eq!(
        invalid_amount_asset.assert_sent_native_token_balance(&message_info),
        Err(StdError::generic_err(
            "Native token balance mismatch between the argument and the transferred"
        ))
    );

    let invalid_amount_asset = Asset {
        amount: Uint128::from(1u8),
        info: AssetInfo::NativeToken {
            denom: "ulunc".to_string(),
        },
    };

    assert_eq!(
        invalid_amount_asset.assert_sent_native_token_balance(&message_info),
        Err(StdError::generic_err(
            "Native token balance mismatch between the argument and the transferred"
        ))
    )
}

#[test]
fn test_asset_to_raw() {
    let deps = mock_dependencies(&[]);
    let native_asset = Asset {
        amount: Uint128::from(1u128),
        info: AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
    };

    let native_asset_to_raw = native_asset.to_raw(&deps.api).unwrap();

    assert_eq!(
        native_asset_to_raw,
        AssetRaw {
            amount: Uint128::from(1u128),
            info: AssetInfoRaw::NativeToken {
                denom: "uluna".to_string()
            }
        }
    );

    assert_eq!(
        native_asset_to_raw.to_normal(&deps.api).unwrap(),
        native_asset
    );

    let token_asset = Asset {
        amount: Uint128::from(1u128),
        info: AssetInfo::Token {
            contract_addr: "contract0000".to_string(),
        },
    };

    let token_asset_to_raw = token_asset.to_raw(&deps.api).unwrap();

    assert_eq!(
        token_asset_to_raw,
        AssetRaw {
            amount: Uint128::from(1u128),
            info: AssetInfoRaw::Token {
                contract_addr: deps.api.addr_canonicalize("contract0000").unwrap()
            }
        }
    );

    assert_eq!(
        token_asset_to_raw.to_normal(&deps.api).unwrap(),
        token_asset
    )
}

#[test]
fn test_asset_info_raw_equal() {
    let native_asset_info_raw = AssetInfoRaw::NativeToken {
        denom: "uluna".to_string(),
    };

    assert!(native_asset_info_raw.equal(&AssetInfoRaw::NativeToken {
        denom: "uluna".to_string()
    }));

    assert!(!native_asset_info_raw.equal(&AssetInfoRaw::NativeToken {
        denom: "ulunc".to_string()
    }));

    let deps = mock_dependencies(&[]);
    assert!(!native_asset_info_raw.equal(&AssetInfoRaw::Token {
        contract_addr: deps.api.addr_canonicalize("contract0000").unwrap()
    }));

    let token_asset_info_raw = AssetInfoRaw::Token {
        contract_addr: deps.api.addr_canonicalize("contract0000").unwrap(),
    };
    assert!(token_asset_info_raw.equal(&AssetInfoRaw::Token {
        contract_addr: deps.api.addr_canonicalize("contract0000").unwrap()
    }));

    assert!(!token_asset_info_raw.equal(&AssetInfoRaw::Token {
        contract_addr: deps.api.addr_canonicalize("contract000").unwrap()
    }));

    assert!(!token_asset_info_raw.equal(&AssetInfoRaw::NativeToken {
        denom: "uluna".to_string()
    }));
}

#[test]
fn query_terraswap_pair_contract() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_terraswap_factory(
        &[(
            &"asset0000uusd".to_string(),
            &PairInfo {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "asset0000".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                ],
                contract_addr: "pair0000".to_string(),
                liquidity_token: "liquidity0000".to_string(),
                asset_decimals: [6u8, 6u8],
            },
        )],
        &[("uusd".to_string(), 6u8)],
    );

    let pair_info: PairInfo = query_pair_info(
        &deps.as_ref().querier,
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        &[
            AssetInfo::Token {
                contract_addr: "asset0000".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ],
    )
    .unwrap();

    assert_eq!(pair_info.contract_addr, Addr::unchecked("pair0000"),);
    assert_eq!(pair_info.liquidity_token, Addr::unchecked("liquidity0000"),);
}
