use cosmwasm_std::{Addr, Api, CanonicalAddr, Deps, Order, StdResult};
use cw20::{AllAccountsResponse, AllAllowancesResponse, AllowanceInfo};

use crate::state::{ALLOWANCES, BALANCES};
use cw_storage_plus::Bound;

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn calc_range_start_human(
    api: &dyn Api,
    start_after: Option<Addr>,
) -> StdResult<Option<Vec<u8>>> {
    match start_after {
        Some(human) => {
            let mut v: Vec<u8> = api.addr_canonicalize(human.as_ref())?.0.into();
            v.push(0);
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

pub fn query_all_allowances(
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let owner_addr = deps.api.addr_canonicalize(&owner)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start =
        calc_range_start_human(deps.api, start_after.map(Addr::unchecked))?.map(Bound::exclusive);

    let allowances: StdResult<Vec<AllowanceInfo>> = ALLOWANCES
        .prefix(owner_addr.as_slice())
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(AllowanceInfo {
                spender: deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string(),
                allowance: v.allowance,
                expires: v.expires,
            })
        })
        .collect();
    Ok(AllAllowancesResponse {
        allowances: allowances?,
    })
}

pub fn query_all_accounts(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAccountsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start =
        calc_range_start_human(deps.api, start_after.map(Addr::unchecked))?.map(Bound::exclusive);

    let accounts: Result<Vec<_>, _> = BALANCES
        .keys(deps.storage, start, None, Order::Ascending)
        .map(|k| {
            deps.api
                .addr_humanize(&CanonicalAddr::from(k))
                .map(|v| v.to_string())
        })
        .take(limit)
        .collect();

    Ok(AllAccountsResponse {
        accounts: accounts?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Api;
    use cosmwasm_std::{coins, DepsMut, Uint128};
    use cw20::{Cw20Coin, Expiration, TokenInfoResponse};

    use crate::contract::{execute, instantiate, query_token_info};
    use crate::msg::{ExecuteMsg, InstantiateMsg};

    // this will set up the instantiation for other tests
    fn do_instantiate(mut deps: DepsMut, addr: &str, amount: Uint128) -> TokenInfoResponse {
        let instantiate_msg = InstantiateMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20Coin {
                address: addr.into(),
                amount,
            }],
            mint: None,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
        query_token_info(deps.as_ref()).unwrap()
    }

    #[test]
    fn query_all_allowances_works() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let owner = String::from("owner");
        // these are in alphabetical order same than insert order

        let spender1 = deps
            .api
            .addr_humanize(&CanonicalAddr::from(vec![
                1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]))
            .unwrap();
        let spender2 = deps
            .api
            .addr_humanize(&CanonicalAddr::from(vec![
                1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]))
            .unwrap();

        let info = mock_info(owner.as_ref(), &[]);
        let env = mock_env();
        do_instantiate(deps.as_mut(), &owner, Uint128::from(12340000u128));

        // no allowance to start
        let allowances = query_all_allowances(deps.as_ref(), owner.clone(), None, None).unwrap();
        assert_eq!(allowances.allowances, vec![]);

        // set allowance with height expiration
        let allow1 = Uint128::from(7777u128);
        let expires = Expiration::AtHeight(5432);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender1.to_string(),
            amount: allow1,
            expires: Some(expires),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // set allowance with no expiration
        let allow2 = Uint128::from(54321u128);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender2.to_string(),
            amount: allow2,
            expires: None,
        };
        execute(deps.as_mut(), env, info, msg).unwrap();

        // query list gets 2
        let allowances = query_all_allowances(deps.as_ref(), owner.clone(), None, None).unwrap();
        assert_eq!(allowances.allowances.len(), 2);

        // first one is spender1 (order of CanonicalAddr uncorrelated with String)
        let allowances = query_all_allowances(deps.as_ref(), owner.clone(), None, Some(1)).unwrap();
        assert_eq!(allowances.allowances.len(), 1);
        let allow = &allowances.allowances[0];
        assert_eq!(&allow.spender, &spender1);
        assert_eq!(&allow.expires, &expires);
        assert_eq!(&allow.allowance, &allow1);

        // next one is spender2
        let allowances = query_all_allowances(
            deps.as_ref(),
            owner,
            Some(allow.spender.clone()),
            Some(10000),
        )
        .unwrap();
        assert_eq!(allowances.allowances.len(), 1);
        let allow = &allowances.allowances[0];
        assert_eq!(&allow.spender, &spender2);
        assert_eq!(&allow.expires, &Expiration::Never {});
        assert_eq!(&allow.allowance, &allow2);
    }

    #[test]
    fn query_all_accounts_works() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        // insert order and lexicographical order are different
        let acct1 = deps
            .api
            .addr_humanize(&CanonicalAddr::from(vec![
                1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]))
            .unwrap()
            .to_string();
        let acct2 = deps
            .api
            .addr_humanize(&CanonicalAddr::from(vec![
                1, 1, 1, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]))
            .unwrap()
            .to_string();
        let acct3 = deps
            .api
            .addr_humanize(&CanonicalAddr::from(vec![
                1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]))
            .unwrap()
            .to_string();
        let acct4 = deps
            .api
            .addr_humanize(&CanonicalAddr::from(vec![
                1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]))
            .unwrap()
            .to_string();
        let expected_order = [acct4.clone(), acct1.clone(), acct3.clone(), acct2.clone()];

        do_instantiate(deps.as_mut(), &acct1, Uint128::from(12340000u128));

        // put money everywhere (to create balances)
        let info = mock_info(acct1.as_ref(), &[]);
        let env = mock_env();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Transfer {
                recipient: acct2,
                amount: Uint128::from(222222u128),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Transfer {
                recipient: acct3,
                amount: Uint128::from(333333u128),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::Transfer {
                recipient: acct4,
                amount: Uint128::from(444444u128),
            },
        )
        .unwrap();

        // make sure we get the proper results
        let accounts = query_all_accounts(deps.as_ref(), None, None).unwrap();
        assert_eq!(accounts.accounts, expected_order);

        // let's do pagination
        let accounts = query_all_accounts(deps.as_ref(), None, Some(2)).unwrap();
        assert_eq!(accounts.accounts, expected_order[0..2].to_vec());

        let accounts =
            query_all_accounts(deps.as_ref(), Some(accounts.accounts[1].clone()), Some(1)).unwrap();
        assert_eq!(accounts.accounts, expected_order[2..3].to_vec());

        let accounts =
            query_all_accounts(deps.as_ref(), Some(accounts.accounts[0].clone()), Some(777))
                .unwrap();
        assert_eq!(accounts.accounts, expected_order[3..].to_vec());
    }

    use cosmwasm_std::{StdResult, Storage};
    use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket, ReadonlyPrefixedStorage};
    use cw20::AllowanceResponse;

    const PREFIX_BALANCE: &[u8] = b"balance";
    const PREFIX_ALLOWANCE: &[u8] = b"allowance";

    /// balances are state of the erc20 tokens
    pub fn legacy_balances(storage: &mut dyn Storage) -> Bucket<Uint128> {
        bucket(storage, PREFIX_BALANCE)
    }

    /// balances are state of the erc20 tokens (read-only version for queries)
    pub fn legacy_balances_read(storage: &dyn Storage) -> ReadonlyBucket<Uint128> {
        bucket_read(storage, PREFIX_BALANCE)
    }

    pub fn legacy_balances_prefix_read(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
        ReadonlyPrefixedStorage::new(storage, PREFIX_BALANCE)
    }

    /// returns a bucket with all allowances authorized by this owner (query it by spender)
    pub fn legacy_allowances<'a>(
        storage: &'a mut dyn Storage,
        owner: &CanonicalAddr,
    ) -> Bucket<'a, AllowanceResponse> {
        Bucket::multilevel(storage, &[PREFIX_ALLOWANCE, owner.as_slice()])
    }

    /// returns a bucket with all allowances authorized by this owner (query it by spender)
    /// (read-only version for queries)
    pub fn legacy_allowances_read<'a>(
        storage: &'a dyn Storage,
        owner: &CanonicalAddr,
    ) -> ReadonlyBucket<'a, AllowanceResponse> {
        ReadonlyBucket::multilevel(storage, &[PREFIX_ALLOWANCE, owner.as_slice()])
    }

    // const PREFIX_PAIR_INFO: &[u8] = b"pair_info";
    pub fn legacy_query_all_allowances(
        storage: &dyn Storage,
        api: &dyn Api,
        owner: Addr,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> StdResult<AllAllowancesResponse> {
        let owner_raw = api.addr_canonicalize(owner.as_str())?;
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = calc_range_start_human(api, start_after)?;
        let allowances: StdResult<Vec<AllowanceInfo>> = legacy_allowances_read(storage, &owner_raw)
            .range(start.as_deref(), None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (k, v) = item?;
                Ok(AllowanceInfo {
                    spender: api.addr_humanize(&CanonicalAddr::from(k))?.to_string(),
                    allowance: v.allowance,
                    expires: v.expires,
                })
            })
            .collect();

        Ok(AllAllowancesResponse {
            allowances: allowances?,
        })
    }

    pub fn legacy_query_all_accounts(
        storage: &dyn Storage,
        api: &dyn Api,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> StdResult<AllAccountsResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = calc_range_start_human(api, start_after)?;
        let accounts: StdResult<Vec<_>> = legacy_balances_prefix_read(storage)
            .range(start.as_deref(), None, Order::Ascending)
            .take(limit)
            .map(|(k, _)| {
                api.addr_humanize(&CanonicalAddr::from(k))
                    .map(|v| v.to_string())
            })
            .collect();

        Ok(AllAccountsResponse {
            accounts: accounts?,
        })
    }

    #[test]
    fn balances_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        let mut balances = legacy_balances(&mut deps.storage);
        let addr1 = deps.api.addr_canonicalize("addr0000").unwrap();
        let addr2 = deps.api.addr_canonicalize("addr0001").unwrap();
        let key1 = addr1.as_slice();
        let key2 = addr2.as_slice();

        balances.save(key1, &Uint128::from(100u128)).unwrap();
        balances.save(key2, &Uint128::from(200u128)).unwrap();

        let balances_read = legacy_balances_read(&deps.storage);
        assert_eq!(
            BALANCES.load(&deps.storage, key1).unwrap(),
            balances_read.load(key1).unwrap()
        );

        assert_eq!(
            BALANCES.load(&deps.storage, key2).unwrap(),
            balances_read.load(key2).unwrap()
        );

        assert_eq!(
            query_all_accounts(deps.as_ref(), None, None),
            legacy_query_all_accounts(&deps.storage, &deps.api, None, None),
        );
    }

    #[test]
    fn allowance_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        let owner_addr = deps.api.addr_canonicalize("owner0000").unwrap();
        let owner_key = owner_addr.as_slice();
        let addr1 = deps.api.addr_canonicalize("addr0000").unwrap();
        let addr2 = deps.api.addr_canonicalize("addr0001").unwrap();
        let key1 = addr1.as_slice();
        let key2 = addr2.as_slice();

        let mut allowances = legacy_allowances(&mut deps.storage, &owner_addr);

        allowances
            .save(
                key1,
                &AllowanceResponse {
                    allowance: Uint128::from(100u128),
                    expires: Expiration::AtHeight(5432),
                },
            )
            .unwrap();

        allowances
            .save(
                key2,
                &AllowanceResponse {
                    allowance: Uint128::from(200u128),
                    expires: Expiration::AtHeight(2345),
                },
            )
            .unwrap();

        let allowance_read = legacy_allowances_read(&deps.storage, &owner_addr);
        assert_eq!(
            ALLOWANCES.load(&deps.storage, (owner_key, key1)).unwrap(),
            allowance_read.load(key1).unwrap()
        );

        assert_eq!(
            ALLOWANCES.load(&deps.storage, (owner_key, key2)).unwrap(),
            allowance_read.load(key2).unwrap()
        );

        assert_eq!(
            query_all_allowances(deps.as_ref(), "owner0000".to_string(), None, None),
            legacy_query_all_allowances(
                &deps.storage,
                &deps.api,
                Addr::unchecked("owner0000"),
                None,
                None
            ),
        );
    }
}
