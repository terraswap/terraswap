// Will be moved into terraswap_interface (ToBeFixed)
use std::{collections::{HashMap, hash_map::IterMut}};
use serde::{Deserialize, Serialize};
use cosmwasm_std::{StdResult, Uint128, StdError};
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

        let temp_ust_in_hashmap = HashMap::from([
            (String::from(UUSD), temp_input_asset),
        ]);
        res = try_pairing_with_unmatched_assets(temp_ust_in_hashmap, res);
        
    } else {
        // withdraw
    }

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
        let token_name = get_asset_name(unit_asset.clone());
        let point = balanced_assets_info.new_unmatched_assets
            .entry(token_name)
            .or_insert(unit_asset.clone());

        point.amount += unit_asset.amount;
    }

    balanced_assets_info
}

fn try_pairing_with_unmatched_assets(
    mut input_assets: HashMap<String, Asset>,
    mut balanced_assets_info: NewCalculatedBalacedAssets
) -> NewCalculatedBalacedAssets {
    let portions = calculate_weight_unmatched_assets(
        balanced_assets_info.new_virtual_pairs.clone(),
        balanced_assets_info.new_unmatched_assets.clone(),
    ).unwrap();

    if input_assets.contains_key(&String::from(UUSD)) &&
        !balanced_assets_info.new_unmatched_assets.contains_key(&String::from(UUSD))
    {
        // UST provide
        // Riskleg in unmatched assets
        let provided_ust = input_assets.get_mut(&String::from(UUSD)).unwrap();

        (
            *provided_ust,
            balanced_assets_info.new_virtual_pairs,
            balanced_assets_info.new_reserved_asset,
            balanced_assets_info.new_used_reserved_asset
        ) = actual_paring(
            portions,
            provided_ust.clone(),
            balanced_assets_info.new_virtual_pairs,
            balanced_assets_info.new_reserved_asset,
            balanced_assets_info.new_used_reserved_asset,
            balanced_assets_info.new_unmatched_assets.iter_mut()
        );

    } else if !input_assets.contains_key(&String::from(UUSD)) &&
        balanced_assets_info.new_unmatched_assets.contains_key(&String::from(UUSD)){

        // Riskleg provide
        // UST in unmatched assets

        let provided_ust = balanced_assets_info.new_unmatched_assets.get_mut(&String::from(UUSD)).unwrap();

        (
            *provided_ust,
            balanced_assets_info.new_virtual_pairs,
            balanced_assets_info.new_reserved_asset,
            balanced_assets_info.new_used_reserved_asset
        ) = actual_paring(
            portions,
            provided_ust.clone(),
            balanced_assets_info.new_virtual_pairs,
            balanced_assets_info.new_reserved_asset,
            balanced_assets_info.new_used_reserved_asset,
            input_assets.iter_mut()
        );
    }

    balanced_assets_info
}

fn actual_paring(
    portions: HashMap<String, Uint128>,
    mut provided_ust: Asset,
    mut pairset: HashMap<String, Pairset>,
    mut reserve_ust: Asset,
    mut used_reserve_ust: Asset,
    enumerated_assets: IterMut<String, Asset>
) -> (Asset, HashMap<String, Pairset>, Asset, Asset) {
    for (token_name, unmatched_unit_asset) in enumerated_assets {
        let ust_portion = unmatched_unit_asset.amount
                            .checked_mul(
                                *portions.get(token_name)
                                         .unwrap()
                            ).unwrap()
                            .checked_div(Uint128::from(STABLELEG_DENOMINATOR)).unwrap();

        let curr_pairset = pairset.get_mut(token_name).unwrap();
        let ratio = derive_unit_ratio(curr_pairset);
        let riskleg_ust_value = unmatched_unit_asset.amount
                                .checked_mul(ratio).unwrap()
                                .checked_div(Uint128::from(STABLELEG_DENOMINATOR)).unwrap();

        if ust_portion > riskleg_ust_value {
            curr_pairset.stableleg.amount += riskleg_ust_value;
            curr_pairset.riskleg.amount += unmatched_unit_asset.amount;
            unmatched_unit_asset.amount = Uint128::zero();
            provided_ust.amount = provided_ust.amount - riskleg_ust_value;
        } else {
            curr_pairset.stableleg.amount += ust_portion;

            let provided_riskleg_amount = ust_portion
                                            .checked_mul(Uint128::from(STABLELEG_DENOMINATOR)).unwrap()
                                            .checked_div(unmatched_unit_asset.amount).unwrap();
            curr_pairset.riskleg.amount += provided_riskleg_amount;

            unmatched_unit_asset.amount -= provided_riskleg_amount;
            provided_ust.amount = Uint128::zero();
        }
    }

    (provided_ust, pairset, reserve_ust, used_reserve_ust)
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

fn get_asset_name(asset: Asset) -> String {
    match asset.info {
        AssetInfo::NativeToken{ denom } => denom,
        AssetInfo::Token{ contract_addr } => contract_addr,      
    }
}
