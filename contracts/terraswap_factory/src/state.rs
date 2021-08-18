use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Api, CanonicalAddr, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};
use terraswap::asset::{AssetInfoRaw, PairInfo, PairInfoRaw};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub pair_code_id: u64,
    pub token_code_id: u64,
}

// put the length bytes at the first for compatibility with legacy singleton store
pub const CONFIG: Item<Config> = Item::new("\u{0}\u{6}config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TmpPairInfo {
    pub pair_key: Vec<u8>,
    pub asset_infos: [AssetInfoRaw; 2],
}

pub const TMP_PAIR_INFO: Item<TmpPairInfo> = Item::new("tmp_pair_info");
pub const PAIRS: Map<&[u8], PairInfoRaw> = Map::new("pair_info");

pub fn pair_key(asset_infos: &[AssetInfoRaw; 2]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat()
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_pairs(
    storage: &dyn Storage,
    api: &dyn Api,
    start_after: Option<[AssetInfoRaw; 2]>,
    limit: Option<u32>,
) -> StdResult<Vec<PairInfo>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::exclusive);

    PAIRS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            v.to_normal(api)
        })
        .collect::<StdResult<Vec<PairInfo>>>()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<[AssetInfoRaw; 2]>) -> Option<Vec<u8>> {
    start_after.map(|asset_infos| {
        let mut asset_infos = asset_infos.to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

        let mut v = [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()]
            .concat()
            .as_slice()
            .to_vec();
        v.push(1);
        v
    })
}

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{Api, StdResult, Storage};
    use cosmwasm_storage::{
        bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket,
    };
    const KEY_CONFIG: &[u8] = b"config";

    pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
        singleton(storage, KEY_CONFIG).save(config)
    }
    pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
        singleton_read(storage, KEY_CONFIG).load()
    }

    #[test]
    fn config_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        store_config(
            &mut deps.storage,
            &Config {
                owner: deps.api.addr_canonicalize("owner0000").unwrap(),
                pair_code_id: 1,
                token_code_id: 1,
            },
        )
        .unwrap();

        assert_eq!(
            CONFIG.load(&deps.storage).unwrap(),
            read_config(&deps.storage).unwrap()
        );
    }

    const PREFIX_PAIR_INFO: &[u8] = b"pair_info";
    pub fn store_pair(storage: &mut dyn Storage, data: &PairInfoRaw) -> StdResult<()> {
        let mut asset_infos = data.asset_infos.clone().to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

        let mut pair_bucket: Bucket<PairInfoRaw> = bucket(storage, PREFIX_PAIR_INFO);
        pair_bucket.save(
            &[asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat(),
            data,
        )
    }
    pub fn read_pair(
        storage: &dyn Storage,
        asset_infos: &[AssetInfoRaw; 2],
    ) -> StdResult<PairInfoRaw> {
        let mut asset_infos = asset_infos.clone().to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

        let pair_bucket: ReadonlyBucket<PairInfoRaw> = bucket_read(storage, PREFIX_PAIR_INFO);
        pair_bucket.load(&[asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat())
    }

    pub fn legacy_read_pairs(
        storage: &dyn Storage,
        api: &dyn Api,
        start_after: Option<[AssetInfoRaw; 2]>,
        limit: Option<u32>,
    ) -> StdResult<Vec<PairInfo>> {
        let pair_bucket: ReadonlyBucket<PairInfoRaw> = bucket_read(storage, PREFIX_PAIR_INFO);
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = calc_range_start(start_after);
        pair_bucket
            .range(start.as_deref(), None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (_, v) = item?;
                v.to_normal(api)
            })
            .collect()
    }

    #[test]
    fn pair_info_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        let pair_info = PairInfoRaw {
            asset_infos: [
                AssetInfoRaw::NativeToken {
                    denom: "uusd".to_string(),
                },
                AssetInfoRaw::Token {
                    contract_addr: deps.api.addr_canonicalize("token0000").unwrap(),
                },
            ],
            contract_addr: deps.api.addr_canonicalize("pair0000").unwrap(),
            liquidity_token: deps.api.addr_canonicalize("liquidity0000").unwrap(),
        };

        let pair_info2 = PairInfoRaw {
            asset_infos: [
                AssetInfoRaw::NativeToken {
                    denom: "uusd".to_string(),
                },
                AssetInfoRaw::Token {
                    contract_addr: deps.api.addr_canonicalize("token0001").unwrap(),
                },
            ],
            contract_addr: deps.api.addr_canonicalize("pair0001").unwrap(),
            liquidity_token: deps.api.addr_canonicalize("liquidity0001").unwrap(),
        };

        store_pair(&mut deps.storage, &pair_info).unwrap();
        store_pair(&mut deps.storage, &pair_info2).unwrap();

        assert_eq!(
            PAIRS
                .load(&deps.storage, &pair_key(&pair_info.asset_infos))
                .unwrap(),
            read_pair(&deps.storage, &pair_info.asset_infos).unwrap()
        );

        assert_eq!(
            PAIRS
                .load(&deps.storage, &pair_key(&pair_info2.asset_infos))
                .unwrap(),
            read_pair(&deps.storage, &pair_info2.asset_infos).unwrap()
        );

        assert_eq!(
            read_pairs(&deps.storage, &deps.api, None, None),
            legacy_read_pairs(&deps.storage, &deps.api, None, None),
        );
    }
}
