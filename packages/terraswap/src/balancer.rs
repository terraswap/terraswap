// Will be moved into terraswap_interface (ToBeFixed)
use std::{str::FromStr, collections::HashMap};
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, DepsMut, Decimal, StdResult, Uint128, Order, StdError};
use cw_storage_plus::Map;
use crate::asset::{Asset, AssetInfo};

#[derive(Clone)]
pub struct NewCalculatedBalacedAssets {
    pub new_virtual_pairs: HashMap<String, Pairset>,
    pub new_unmatched_assets: HashMap<String, Asset>,
    pub new_reserved_asset: Asset,
    pub new_used_reserved_asset: Asset,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Pairset {
    pub stableleg: Asset,
    pub riskleg: Asset,
    pub riskleg_denominator: u32,
}

const STABLELEG_DENOMINATOR: u32 = 6;
const UUSD: &str = "uusd";

/// calculate new balanced assets from the existing one & input assets
/// calculate_balanced_assets()
///     -> StdResult<NewCalculatedBalacedAssets>
/// 
/// Check https://www.notion.so/delight-labs/beb67d49bdda488fb222bf56ffa9f2ed#de128f3c50cd42bcb71b17aa53429245
/// TODO: should change the link into the official docs
pub fn calculate_balanced_assets(
    deps: DepsMut,
    is_provide: bool,
    asset: Asset,
    virtual_pairs: HashMap<String, Pairset>,
    unmatched_assets: HashMap<String, Asset>,
    reserved_asset: Asset,
    used_reserved_asset: Asset,
) -> StdResult<NewCalculatedBalacedAssets> {
    let mut temp_input_asset = asset.clone();
    let mut res = NewCalculatedBalacedAssets {
        new_virtual_pairs: virtual_pairs.clone(),
        new_unmatched_assets: unmatched_assets.clone(),
        new_reserved_asset: reserved_asset.clone(),
        new_used_reserved_asset: used_reserved_asset.clone(),
    };

    // provide
    if is_provide {
        // If the protocol has used the reserved UST + the provided currency is stableleg
        // -> pay back first
        let mut is_stableleg_provide = false;
        if used_reserved_asset.amount > Uint128::from(0u128) {
            match asset.info {
                AssetInfo::NativeToken{ denom } => match denom.as_str() {
                    // TODO: how to treat 'ukrw' + how to avoid 'uluna' & IBC tokens
                    UUSD => {
                        is_stableleg_provide = true;
                        (temp_input_asset, res) = reserve_asset_process(temp_input_asset, res);
                    },
                    _ => (),
                },
                _ => (),
            };
        }

        // If paid back complete, nothing to do. Done it.
        if temp_input_asset.amount == Uint128::from(0u128) { return Ok(res); }

        if (is_stableleg_provide && res.new_unmatched_assets.contains_key(&String::from(UUSD))) || 
            (!is_stableleg_provide && !res.new_unmatched_assets.contains_key(&String::from(UUSD))) {
            
            res = put_unmatched_asset(vec![temp_input_asset], res);
            return Ok(res);
        }
        
    } else {
        // withdraw
    }
    

    // just template for the future
    // not usable
    // let mut whole_stableleg_size = Uint128::from(0u128);
    // for may_unit_pair in virtual_pairs.range(deps.storage, None, None, Order::Ascending) {
    //     let unit_pair = may_unit_pair.expect("Wrong asset info is given").1;

    //     for unit_asset in unit_pair.asset_infos {
    //         match unit_asset {
    //             AssetInfo::NativeToken{ denom } => match denom.as_str() {
    //                 // TODO: how to treat 'ukrw' + how to avoid 'uluna' & IBC tokens
    //                 "uusd" => {
    //                     let unit_pool_addr: Addr = deps.api.addr_validate(unit_pair.contract_addr.as_str())?;
    //                     let uusd_amount = unit_asset.query_pool(&deps.querier, deps.api, unit_pool_addr)?;
    //                     whole_stableleg_size += uusd_amount;
    //                 },
    //                 _ => (),
    //             },
    //             _ => (),
    //         }
    //     }
    // }
    
    return Ok(res);
}

fn reserve_asset_process(
    mut input_asset: Asset,
    mut balanced_assets_info: NewCalculatedBalacedAssets
) -> (Asset, NewCalculatedBalacedAssets) {
    if balanced_assets_info.new_used_reserved_asset.amount >= input_asset.amount {
        balanced_assets_info.new_used_reserved_asset.amount -= input_asset.amount;
        balanced_assets_info.new_reserved_asset.amount += input_asset.amount;
        input_asset.amount = Uint128::zero();
    } else {
        balanced_assets_info.new_reserved_asset.amount += balanced_assets_info.new_used_reserved_asset.amount;
        input_asset.amount -= balanced_assets_info.new_used_reserved_asset.amount;
        balanced_assets_info.new_used_reserved_asset.amount = Uint128::zero();
    }

    (input_asset, balanced_assets_info)
}

fn put_unmatched_asset(
    input_assets: Vec<Asset>,
    mut balanced_assets_info: NewCalculatedBalacedAssets
) -> NewCalculatedBalacedAssets {
    for unit_asset in input_assets.iter() {
        let token_name = match &unit_asset.info {
            AssetInfo::NativeToken{ denom } => denom,
            AssetInfo::Token{ contract_addr } => contract_addr,      
        }.to_string();

        let point = balanced_assets_info.new_unmatched_assets
            .entry(token_name)
            .or_insert(unit_asset.clone());

        point.amount += unit_asset.amount;
    }

    balanced_assets_info
}

fn try_pairing_with_unmatched_assets(
    deps: DepsMut,
    input_assets: Vec<Asset>,
    balanced_assets_info: NewCalculatedBalacedAssets
) -> NewCalculatedBalacedAssets {
    // if input_assets.len() == 1 && input_assets.0.info
    balanced_assets_info
}

fn calculate_weight_unmatched_assets(
    curr_pairs: HashMap<String, Pairset>,
    unmatched_assets: HashMap<String, Asset>
) -> StdResult<HashMap<String, Uint128>> {
    let mut res: HashMap<String, Uint128> = HashMap::new();
    let mut whole_portion = Uint128::from(0u128);

    for (token_name, unit_asset) in unmatched_assets.iter() {
        let pairset = match curr_pairs.get(token_name) {
            Some(pairinfo) => pairinfo,
            None => return Err(StdError::not_found("no pair info")),
        };

        let ratio = derive_unit_ratio(pairset);
        let portion = unit_asset.amount.checked_mul(ratio).unwrap();
        whole_portion += portion.clone();

        res.insert(token_name.clone(), ratio);
    }
    
    for (_, val) in res.iter_mut() {
        *val = val
                .checked_mul(Uint128::from(u128::pow(10, STABLELEG_DENOMINATOR))).unwrap()
                .checked_div(whole_portion).unwrap();
    }

    Ok(res)
}

fn derive_unit_ratio(
    unit_pair: &Pairset
) -> Uint128 {
    // Riskleg amount * 10^6 / stableleg
    // 10^6 is already multiplied
    if unit_pair.riskleg_denominator < STABLELEG_DENOMINATOR {
        unit_pair.riskleg.amount
            .checked_mul(Uint128::from(u128::pow(10, 2 * STABLELEG_DENOMINATOR - unit_pair.riskleg_denominator))).unwrap()
            .checked_div(unit_pair.stableleg.amount).unwrap()
    } else {
        unit_pair.riskleg.amount
            .checked_mul(Uint128::from(u128::pow(10, unit_pair.riskleg_denominator))).unwrap()
            .checked_div(unit_pair.stableleg.amount).unwrap()
    }
}
