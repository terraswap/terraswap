// Will be moved into terraswap_interface (ToBeFixed)
use std::{collections::{HashMap, hash_map::IterMut}, fmt};
use serde::{Deserialize, Serialize};
use cosmwasm_std::{StdResult, Uint128, StdError};
use crate::asset::{Asset, AssetInfo};

#[derive(Clone, Debug, PartialEq)]
pub struct NewCalculatedBalacedAssets {
    pub new_virtual_pairs: HashMap<String, Pairset>,
    pub new_unmatched_assets: HashMap<String, Asset>,
    pub new_reserved_asset: Asset,
    pub new_used_reserved_asset: Asset,
    pub reserve_usage_ratio: Uint128, // 10^6 denominator
}

impl fmt::Display for NewCalculatedBalacedAssets {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut new_virtual_pairs_vec: Vec<String> = self.new_virtual_pairs.iter()
                                                    .map(|(k, v)| format!("{} / {}", *k , *v) )
                                                    .collect();
        new_virtual_pairs_vec.sort();
        let new_virtual_pairs_str = new_virtual_pairs_vec.join("\n");

        let mut new_unmatched_assets_vec: Vec<String> = self.new_unmatched_assets.iter()
                                                    .map(|(k, v)| format!("{} / {}", *k, *v) )
                                                    .collect();
        new_unmatched_assets_vec.sort();
        let new_unmatched_assets_str = new_unmatched_assets_vec.join("\n");

        write!(f, "Pairs\n{}\n\nUnmatched assets:\n{}\n\nReserved UST: {}\nUsed reserved UST: {}\nReserve usage ratio: {}",
                    new_virtual_pairs_str, new_unmatched_assets_str, self.new_reserved_asset, self.new_used_reserved_asset, self.reserve_usage_ratio)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pairset {
    pub stableleg: Asset,
    pub riskleg: Asset,
    pub riskleg_denominator: u32,
}

impl fmt::Display for Pairset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Stableleg: {:?} Riskleg: {:?} Denom: {}", self.stableleg, self.riskleg, self.riskleg_denominator)
    }
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
    reserve_usage_ratio: Uint128,
) -> StdResult<NewCalculatedBalacedAssets> {
    let mut temp_input_asset = asset.clone();
    let mut res = NewCalculatedBalacedAssets {
        new_virtual_pairs: virtual_pairs.clone(),
        new_unmatched_assets: unmatched_assets.clone(),
        new_reserved_asset: reserved_asset.clone(),
        new_used_reserved_asset: used_reserved_asset.clone(),
        reserve_usage_ratio: reserve_usage_ratio,
    };

    if is_provide {
        // provide

        // If the protocol has used the reserved UST + the provided currency is stableleg
        // -> pay back first
        let mut is_stableleg_provide = false;
        let asset_name = get_asset_name(&asset);
        if asset_name == String::from(UUSD) {
            is_stableleg_provide = true;
            (temp_input_asset, res) = reserve_asset_process(temp_input_asset, res);
        }

        // If paid back complete, nothing to do. Done it.
        if temp_input_asset.amount == Uint128::zero() 
                && res.new_used_reserved_asset.amount == Uint128::zero() { return Ok(res); }

        if (is_stableleg_provide && res.new_unmatched_assets.contains_key(&String::from(UUSD))) || 
            (!is_stableleg_provide && !res.new_unmatched_assets.contains_key(&String::from(UUSD))) {
            
            res = put_unmatched_asset(vec![temp_input_asset.clone()], res);

            if is_stableleg_provide {
                return Ok(res);
            }
        }

        let temp_asset_in_hashmap = HashMap::from([
            (get_asset_name(&temp_input_asset), temp_input_asset),
        ]);

        res = try_pairing_with_unmatched_assets(temp_asset_in_hashmap, res);
        
    } else {
        // withdraw

        // check the unmatched assets
        let withdraw_asset_name = get_asset_name(&asset);
        if res.new_unmatched_assets.contains_key(&withdraw_asset_name) {
            let mut unit_unmatched_asset = res.new_unmatched_assets.get_mut(&withdraw_asset_name).unwrap();
            if asset.amount < unit_unmatched_asset.amount {
                // If unmatched asset is same as withdraw asset type
                // withdraw asset amount < Unmatched asset
                //   -> withdraw from the unmached asset, and do nothing more
                unit_unmatched_asset.amount -= asset.amount;
                return Ok(res);

            } else {
                // If unmatched asset is same as withdraw asset type
                // withdraw asset amount >= Unmatched asset
                //   -> withdraw all unmacthed asset
                //      Need addtional assets for withdraw. Make it by pair withdraw
                temp_input_asset.amount -= unit_unmatched_asset.amount;
                unit_unmatched_asset.amount = Uint128::zero();

                res.new_unmatched_assets.retain(|_, v| v.amount != Uint128::zero());

                // will not return. Proceed to the forward
            }
        }

        if withdraw_asset_name == String::from(UUSD) {
            // UST withdraw -> weight withdraw from all pairs

            let possible_use_reserve_ust = res.new_reserved_asset.amount
                                              .checked_mul( res.reserve_usage_ratio ).unwrap()
                                              .checked_div( get_pow10(STABLELEG_DENOMINATOR) ).unwrap();

            if temp_input_asset.amount < possible_use_reserve_ust {
                res.new_reserved_asset.amount -= temp_input_asset.amount;
                res.new_used_reserved_asset.amount += temp_input_asset.amount;

                return Ok(res);
            } else {
                temp_input_asset.amount -= possible_use_reserve_ust;
                res.new_used_reserved_asset.amount += possible_use_reserve_ust;

                (res.new_virtual_pairs, res.new_unmatched_assets) =
                        try_withdrawing_with_ust(&temp_input_asset,res.new_virtual_pairs, res.new_unmatched_assets);

                res = try_pairing_with_unmatched_assets(res.new_unmatched_assets.clone(), res);
            }
        } else {
            // Risk withdraw -> only one pair withdraw

            let withdrawal_pair = res.new_virtual_pairs.get_mut(&withdraw_asset_name).unwrap();
            let withdrawal_ust = derive_unit_ratio(withdrawal_pair)
                                    .checked_mul(temp_input_asset.amount).unwrap()
                                    .checked_div(get_pow10(STABLELEG_DENOMINATOR)).unwrap();

            withdrawal_pair.riskleg.amount -= temp_input_asset.amount;
            withdrawal_pair.stableleg.amount -= withdrawal_ust;

            // unmached asset `withdrawal_ust`
            // reserve processing
            let mut unpaired_ust = _asset_generator_raw(UUSD, true, withdrawal_ust);
            (unpaired_ust, res) = reserve_asset_process(unpaired_ust, res);

            if unpaired_ust.amount > Uint128::zero() {
                let zero_ust = _asset_generator(UUSD, true, 0, STABLELEG_DENOMINATOR);
                let unmatched_ust = res.new_unmatched_assets.entry(String::from(UUSD)).or_insert(zero_ust);

                unmatched_ust.amount += unpaired_ust.amount;
            }
        }
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
        let token_name = get_asset_name(&unit_asset);
        let mut zero_asset = unit_asset.clone();
        zero_asset.amount = Uint128::zero();

        let point = balanced_assets_info.new_unmatched_assets
            .entry(token_name)
            .or_insert(zero_asset);

        point.amount += unit_asset.amount;
    }

    balanced_assets_info
}

fn try_pairing_with_unmatched_assets(
    mut input_assets: HashMap<String, Asset>,
    mut balanced_assets_info: NewCalculatedBalacedAssets
) -> NewCalculatedBalacedAssets {
    let portions = calculate_weight_unmatched_assets(
        &balanced_assets_info.new_virtual_pairs,
        &balanced_assets_info.new_unmatched_assets,
        &input_assets,
    ).unwrap();
    
    let mut provided_ust = &mut _asset_generator(UUSD, true, 0, STABLELEG_DENOMINATOR);

    if input_assets.contains_key(&String::from(UUSD)) &&
        !balanced_assets_info.new_unmatched_assets.contains_key(&String::from(UUSD))
    {
        // UST provide
        // Riskleg in unmatched assets
        provided_ust = input_assets.get_mut(&String::from(UUSD)).unwrap();

        (
            *provided_ust,
            balanced_assets_info.new_virtual_pairs,
        ) = actual_paring(
            portions.clone(),
            provided_ust.clone(),
            balanced_assets_info.new_virtual_pairs,
            balanced_assets_info.new_unmatched_assets.iter_mut()
        );

        if provided_ust.amount > Uint128::zero() {
            let unit_ust = _asset_generator(UUSD, true, 0, STABLELEG_DENOMINATOR);
            let unmatched_usd_asset = balanced_assets_info.new_unmatched_assets.entry(String::from(UUSD)).or_insert(unit_ust);
            unmatched_usd_asset.amount += provided_ust.amount;
        }

    } else if !input_assets.contains_key(&String::from(UUSD)) &&
        balanced_assets_info.new_unmatched_assets.contains_key(&String::from(UUSD)){

        // Riskleg provide
        // UST in unmatched assets

        provided_ust = balanced_assets_info.new_unmatched_assets.get_mut(&String::from(UUSD)).unwrap();

        (
            *provided_ust,
            balanced_assets_info.new_virtual_pairs,
        ) = actual_paring(
            portions.clone(),
            provided_ust.clone(),
            balanced_assets_info.new_virtual_pairs,
            input_assets.iter_mut()
        );

        // reforming input asset -> unmatched asset if riskleg input is remained
        balanced_assets_info.new_unmatched_assets.retain(|_, v| v.amount != Uint128::zero() );
        input_assets.retain(|_, v| v.amount != Uint128::zero() );

        for (token_name, remain_unit_input_asset) in input_assets.iter() {
            let mut zero_asset = remain_unit_input_asset.clone();
            zero_asset.amount = Uint128::zero();

            let unmatched_obj = balanced_assets_info.new_unmatched_assets.entry(token_name.clone()).or_insert(zero_asset);
            unmatched_obj.amount += remain_unit_input_asset.amount;
        }
    }

    let mut is_unmatched_riskleg = false;
    for (_, unmatched_unit_asset) in balanced_assets_info.new_unmatched_assets.iter_mut() {
        if unmatched_unit_asset.amount > Uint128::zero() && get_asset_name(&unmatched_unit_asset) != String::from(UUSD) {
            is_unmatched_riskleg = true;
            break;
        }
    }

    if is_unmatched_riskleg {
        let before_reserve_ust = Asset {
            info: AssetInfo::NativeToken{ denom: String::from(UUSD) },
            amount: balanced_assets_info.new_reserved_asset.amount
                        .checked_mul(balanced_assets_info.reserve_usage_ratio).unwrap()
                        .checked_div( get_pow10(STABLELEG_DENOMINATOR) ).unwrap(),
        };

        let (
            after_reserve_ust,
            new_virtual_pairs,
        ) = actual_paring(
            portions,
            before_reserve_ust.clone(),
            balanced_assets_info.new_virtual_pairs,
            balanced_assets_info.new_unmatched_assets.iter_mut()
        );

        balanced_assets_info.new_reserved_asset.amount -= before_reserve_ust.amount - after_reserve_ust.amount;
        balanced_assets_info.new_used_reserved_asset.amount += before_reserve_ust.amount - after_reserve_ust.amount;
        balanced_assets_info.new_virtual_pairs = new_virtual_pairs;
    }

    // Delete 0-amount entry
    balanced_assets_info.new_unmatched_assets.retain(|_, v| v.amount != Uint128::zero() );

    balanced_assets_info
}

fn try_withdrawing_with_ust(
    required_ust: &Asset,
    mut pairset: HashMap<String, Pairset>,
    mut unmatched_asset: HashMap<String, Asset>,
) -> (HashMap<String, Pairset>, HashMap<String, Asset>) {

    let portion = calculate_weight_from_whole_pairs(&pairset).unwrap();
    for (unit_token_name, unit_pair) in pairset.iter_mut() {
        if *unit_token_name == String::from(UUSD) { continue; }

        let unit_withdraw_ust = required_ust.amount
                                .checked_mul(*portion.get(unit_token_name).unwrap()).unwrap()
                                .checked_div(get_pow10(STABLELEG_DENOMINATOR)).unwrap();
        let unit_withdraw_risk = unit_pair.riskleg.amount
                                .checked_mul(*portion.get(unit_token_name).unwrap()).unwrap()
                                .checked_div(get_pow10(STABLELEG_DENOMINATOR)).unwrap();

        unit_pair.stableleg.amount -= unit_withdraw_ust;
        unit_pair.riskleg.amount -= unit_withdraw_risk;

        let mut unit_zero_risk = unit_pair.riskleg.clone();
        unit_zero_risk.amount = Uint128::zero();

        let unit_unmatched_asset_entry = unmatched_asset.entry(unit_token_name.clone()).or_insert(unit_zero_risk);
        unit_unmatched_asset_entry.amount += unit_withdraw_risk;
    }
    
    (pairset, unmatched_asset)
}

fn actual_paring(
    portions: HashMap<String, Uint128>,
    provided_ust: Asset,
    mut pairset: HashMap<String, Pairset>,
    enumerated_assets: IterMut<String, Asset>
) -> (Asset, HashMap<String, Pairset>) {
    let mut remain_ust = _asset_generator(UUSD, true, 0, STABLELEG_DENOMINATOR);

    for (token_name, unmatched_unit_asset) in enumerated_assets {
        if *token_name == String::from(UUSD) { continue; }

        // Calculate UST portion of all unmatched assets
        let ust_portion = provided_ust.amount
                            .checked_mul(
                                *portions.get(token_name)
                                         .unwrap()
                            ).unwrap()
                            .checked_div( get_pow10(STABLELEG_DENOMINATOR) ).unwrap();

        // Calculate the UST value of the unmatched asset
        let curr_pairset = pairset.get_mut(token_name).unwrap();
        let unit_asset_price = derive_unit_ratio(curr_pairset);
        let riskleg_ust_value = unmatched_unit_asset.amount
                                    .checked_mul(unit_asset_price).unwrap()
                                    .checked_div( get_pow10(curr_pairset.riskleg_denominator) ).unwrap();

        if ust_portion > riskleg_ust_value {
            curr_pairset.stableleg.amount += riskleg_ust_value;
            curr_pairset.riskleg.amount += unmatched_unit_asset.amount;
            unmatched_unit_asset.amount = Uint128::zero();
            remain_ust.amount += ust_portion - riskleg_ust_value;
        } else {
            curr_pairset.stableleg.amount += ust_portion;

            let provided_riskleg_amount = ust_portion
                                            .checked_mul( get_pow10(STABLELEG_DENOMINATOR) ).unwrap()
                                            .checked_div(unit_asset_price).unwrap();
            curr_pairset.riskleg.amount += provided_riskleg_amount;

            unmatched_unit_asset.amount -= provided_riskleg_amount;
        }
    }

    (remain_ust, pairset)
}

fn calculate_weight_unmatched_assets(
    curr_pairs: &HashMap<String, Pairset>,
    unmatched_assets: &HashMap<String, Asset>,
    input_asset: &HashMap<String, Asset>, // If input asset is riskleg && stableleg is in unmatched asset
) -> StdResult<HashMap<String, Uint128>> {
    let mut res: HashMap<String, Uint128> = HashMap::new();
    let mut whole_portion = Uint128::zero();

    for (token_name, unit_asset) in unmatched_assets.iter() {
        // Calculate the UST value of each asset, and summize
        
        // Only UST is the unmatched asset -> Assign 100%
        // Incoming asset got 100% portion, and the asset should be a key
        if *token_name == String::from(UUSD) {
            let riskleg_token_list: Vec<String> = input_asset.clone().into_keys().collect();
            if riskleg_token_list.len() == 0 {
                return Err(StdError::not_found("if unmatched asset is stableleg, incoming asset should exist in this line, and it should be exact one. No asset in here."));
            } else if riskleg_token_list.len() > 1 {
                return Err(StdError::not_found(
                    format!("if unmatched asset is stableleg, incoming asset should exist in this line, and it should be exact one. Too many assets.\nAssets: {:?}", riskleg_token_list)
                ));
            }

            res.insert(riskleg_token_list[0].clone(), Uint128::from(1_000000u128));
            return Ok(res);
        }

        let pairset = match curr_pairs.get(token_name) {
            Some(pairinfo) => pairinfo,
            None => return Err(StdError::not_found(format!("no pair info {}", token_name))),
        };

        let unit_asset_price = derive_unit_ratio(pairset);
        let portion = unit_asset.amount.checked_mul(unit_asset_price).unwrap()
                                .checked_div(get_pow10(pairset.riskleg_denominator)).unwrap();
        whole_portion += portion.clone();

        res.insert(token_name.clone(), portion);
    }
    
    for (_, val) in res.iter_mut() {
        // Divide by whole portion
        *val = val
                .checked_mul(get_pow10(STABLELEG_DENOMINATOR)).unwrap()
                .checked_div(whole_portion).unwrap();
    }

    Ok(res)
}

fn calculate_weight_from_whole_pairs(
    curr_pairs: &HashMap<String, Pairset>,
) -> StdResult<HashMap<String, Uint128>> {
    let mut res: HashMap<String, Uint128> = HashMap::new();
    let mut whole_portion = Uint128::from(0u128);

    for (token_name, unit_pair) in curr_pairs.iter() {
        let ust_value = unit_pair.stableleg.amount;
        whole_portion += ust_value;

        res.insert(token_name.clone(), ust_value.clone());
    }
    
    for (_, val) in res.iter_mut() {
        *val = val
                .checked_mul(get_pow10(STABLELEG_DENOMINATOR)).unwrap()
                .checked_div(whole_portion).unwrap();
    }

    Ok(res)
}

fn _asset_generator(symbol: &str, is_native: bool, amount: u128, denom: u32) -> Asset {
    match is_native {
        true => 
            Asset {
                info: AssetInfo::NativeToken { denom: String::from(symbol) },
                amount: Uint128::from(amount)
                            .checked_mul(get_pow10(denom)).unwrap(),
            },
        false => 
            Asset {
                info: AssetInfo::Token { contract_addr: String::from(symbol) },
                amount: Uint128::from(amount)
                            .checked_mul(get_pow10(denom)).unwrap(),
            },
    }
}

fn _asset_generator_raw(symbol: &str, is_native: bool, amount: Uint128) -> Asset {
    match is_native {
        true => 
            Asset {
                info: AssetInfo::NativeToken { denom: String::from(symbol) },
                amount: amount,
            },
        false => 
            Asset {
                info: AssetInfo::Token { contract_addr: String::from(symbol) },
                amount: amount,
            },
    }
}

fn derive_unit_ratio(
    unit_pair: &Pairset
) -> Uint128 {
    unit_pair.stableleg.amount
        .checked_mul( get_pow10(unit_pair.riskleg_denominator) ).unwrap()
        .checked_div(unit_pair.riskleg.amount).unwrap()
}

fn get_asset_name(asset: &Asset) -> String {
    match &asset.info {
        AssetInfo::NativeToken{ denom } => denom.clone(),
        AssetInfo::Token{ contract_addr } => contract_addr.clone(),
    }
}

fn get_pow10(denom: u32) -> Uint128 {
    Uint128::from(u128::pow(10, denom))
}

#[cfg(test)]
mod test {
    use super::*;

    const LUNA: &str = "uluna";
    const ANC: &str = "terra14z56l0fp2lsf86zy3hty2z47ezkhnthtr9yq76";
    const SMALL: &str = "msmall";
    const BIG: &str = "pbig";

    fn initilaizer() -> NewCalculatedBalacedAssets {
        let new_virtual_pairs = HashMap::from([
            (
                String::from(LUNA),
                Pairset{
                    riskleg: _asset_generator(LUNA, true, 100, STABLELEG_DENOMINATOR),
                    riskleg_denominator: 6,
                    stableleg: _asset_generator(UUSD, true, 10000, STABLELEG_DENOMINATOR),
                }
            ),
            (
                String::from(ANC),
                Pairset{
                    riskleg: _asset_generator(ANC, false, 1000, STABLELEG_DENOMINATOR),
                    riskleg_denominator: 6,
                    stableleg: _asset_generator(UUSD, true, 1000, STABLELEG_DENOMINATOR),
                }
            ),
            (
                String::from(SMALL),
                Pairset{
                    riskleg: _asset_generator(SMALL, true, 10000, 4),
                    riskleg_denominator: 4,
                    stableleg: _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR),
                }
            ),
            (
                String::from(BIG),
                Pairset{
                    riskleg: _asset_generator(BIG, true, 10000, 9),
                    riskleg_denominator: 9,
                    stableleg: _asset_generator(UUSD, true, 1000000, STABLELEG_DENOMINATOR),
                }
            ),
        ]);

        NewCalculatedBalacedAssets {
            new_virtual_pairs: new_virtual_pairs,
            new_unmatched_assets: HashMap::new(),
            new_reserved_asset: _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR), // meaningless
            new_used_reserved_asset: _asset_generator(UUSD, true, 0, STABLELEG_DENOMINATOR), // meaningless
            reserve_usage_ratio: Uint128::from(100000u128), // 10%
        }
    }

    #[test]
    fn test_balancer_001_stable_provide_unmatched_stable() {
        // Provide stableleg
        // Stable asset exists in the unmatched assets

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);
        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        expected_state.new_unmatched_assets.insert(String::from(UUSD), _asset_generator(UUSD, true, 200, STABLELEG_DENOMINATOR));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_002_big_stable_provide_unmatched_risk() {
        // Provide stableleg
        // Risk asset exists in the unmatched assets
        // Provided asset > Unmatched assets

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 1000, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 1, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);
        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator(LUNA, true, 101, STABLELEG_DENOMINATOR),
            riskleg_denominator: 6,
            stableleg: _asset_generator(UUSD, true, 10100, STABLELEG_DENOMINATOR),
        };

        expected_state.new_unmatched_assets.insert(String::from(UUSD), _asset_generator(UUSD, true, 900, STABLELEG_DENOMINATOR));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_003_small_stable_provide_unmatched_risk_big_reserve() {
        // Provide stableleg
        // Risk asset exists in the unmatched assets
        // Provided asset < Unmatched assets
        // Enough reserve UST

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 10, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);
        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator(LUNA, true, 110, STABLELEG_DENOMINATOR),
            riskleg_denominator: 6,
            stableleg: _asset_generator(UUSD, true, 11000, STABLELEG_DENOMINATOR),
        };

        expected_state.new_unmatched_assets = HashMap::new();
        expected_state.new_reserved_asset = _asset_generator(UUSD, true, 99100, STABLELEG_DENOMINATOR);
        expected_state.new_used_reserved_asset = _asset_generator(UUSD, true, 900, STABLELEG_DENOMINATOR);

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_004_small_stable_provide_unmatched_risk_small_reserve() {
        // Provide stableleg
        // Risk asset exists in the unmatched assets
        // Provided asset < Unmatched assets
        // Small reserve UST

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 10, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(101_100000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator(UUSD, true, 10110, STABLELEG_DENOMINATOR),
        };

        expected_state.new_unmatched_assets = HashMap::from([
            (String::from(LUNA), _asset_generator_raw(LUNA, true, Uint128::from(8_900000u128))),
        ]);

        expected_state.new_reserved_asset = _asset_generator(UUSD, true, 90, STABLELEG_DENOMINATOR);
        expected_state.new_used_reserved_asset = _asset_generator(UUSD, true, 10, STABLELEG_DENOMINATOR);

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_005_big_stable_multiple_provide_unmatched_risks() {
        // Provide stableleg
        // Risk asset exists in the unmatched assets
        // Provided asset > Unmatched assets

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 1000, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 1, STABLELEG_DENOMINATOR)),
            (String::from(ANC), _asset_generator(ANC, false, 1, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator(LUNA, true, 101, STABLELEG_DENOMINATOR),
            riskleg_denominator: 6,
            stableleg: _asset_generator(UUSD, true, 10100, STABLELEG_DENOMINATOR),
        };

        let anc_pair = expected_state.new_virtual_pairs.get_mut(&String::from(ANC)).unwrap();
        *anc_pair = Pairset{
            riskleg: _asset_generator(ANC, false, 1001, STABLELEG_DENOMINATOR),
            riskleg_denominator: 6,
            stableleg: _asset_generator(UUSD, true, 1001, STABLELEG_DENOMINATOR),
        };

        expected_state.new_unmatched_assets = HashMap::from([
            (String::from(UUSD), _asset_generator_raw(UUSD, true, Uint128::from(898_999000u128))),
        ]);

        expected_state.new_reserved_asset = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        expected_state.new_used_reserved_asset = _asset_generator(UUSD, true, 0, STABLELEG_DENOMINATOR);

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_006_small_stable_provide_multiple_unmatched_risks_big_reserve() {
        // Provide stableleg
        // Risk asset exists in the unmatched assets
        // Provided asset < Unmatched assets
        // Big reserve

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 10, STABLELEG_DENOMINATOR)),
            (String::from(ANC), _asset_generator(ANC, false, 10, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);
        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator(LUNA, true, 110, STABLELEG_DENOMINATOR),
            riskleg_denominator: 6,
            stableleg: _asset_generator(UUSD, true, 11000, STABLELEG_DENOMINATOR),
        };

        let anc_pair = expected_state.new_virtual_pairs.get_mut(&String::from(ANC)).unwrap();
        *anc_pair = Pairset{
            riskleg: _asset_generator(ANC, false, 1010, STABLELEG_DENOMINATOR),
            riskleg_denominator: 6,
            stableleg: _asset_generator(UUSD, true, 1010, STABLELEG_DENOMINATOR),
        };

        expected_state.new_unmatched_assets = HashMap::from([]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(99089_989900u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(910_010100u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_007_small_stable_provide_multiple_unmatched_risk_small_reserve() {
        // Provide stableleg
        // Risk asset exists in the unmatched assets
        // Provided asset < Unmatched assets
        // Small reserve

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 10, STABLELEG_DENOMINATOR)),
            (String::from(ANC), _asset_generator(ANC, false, 10, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(101_089108u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(10108_910890u128)),
        };

        let anc_pair = expected_state.new_virtual_pairs.get_mut(&String::from(ANC)).unwrap();
        *anc_pair = Pairset{
            riskleg: _asset_generator_raw(ANC, false, Uint128::from(1001_089000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(1001_089000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([
            (String::from(LUNA), _asset_generator_raw(LUNA, true, Uint128::from(8_910892u128))),
            (String::from(ANC), _asset_generator_raw(ANC, false, Uint128::from(8_911000u128))),
        ]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(90_000000u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(10_000000u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_008_small_stable_provide_multiple_unmatched_risks_big_reserve_with_small_used_reserve() {
        // Provide stableleg
        // Risk asset exists in the unmatched assets
        // Provided asset < Unmatched assets
        // Big reserve
        // Used reserve exists

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 1000, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 10, STABLELEG_DENOMINATOR)),
            (String::from(ANC), _asset_generator(ANC, false, 10, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 99100, STABLELEG_DENOMINATOR);
        let used_reserved_ust = _asset_generator(UUSD, true, 900, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;
        before_state.new_used_reserved_asset = used_reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(110_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(11000_000000u128)),
        };

        let anc_pair = expected_state.new_virtual_pairs.get_mut(&String::from(ANC)).unwrap();
        *anc_pair = Pairset{
            riskleg: _asset_generator_raw(ANC, false, Uint128::from(1010_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(1010_000000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(99089_989900u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(910_010100u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_009_small_stable_provide_multiple_unmatched_risks_big_reserve_with_big_used_reserve() {
        // Provide stableleg, small
        // Risk asset exists in the unmatched assets
        // Provided asset < Unmatched assets
        // Big reserve
        // Used reserve exists

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 10, STABLELEG_DENOMINATOR)),
            (String::from(ANC), _asset_generator(ANC, false, 100, STABLELEG_DENOMINATOR))
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 99900, STABLELEG_DENOMINATOR);
        let used_reserved_ust = _asset_generator(UUSD, true, 900, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;
        before_state.new_used_reserved_asset = used_reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(110_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(11000_000000u128)),
        };

        let anc_pair = expected_state.new_virtual_pairs.get_mut(&String::from(ANC)).unwrap();
        *anc_pair = Pairset{
            riskleg: _asset_generator_raw(ANC, false, Uint128::from(1100_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(1100_000000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(98899_990000u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(1900_010000u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_010_risk_provide_risk_unmatched_asset_big_reserve() {
        // Provide riskleg
        // Riskleg unmatched asset
        // big reserve UST

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(LUNA, true, 1, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 9, STABLELEG_DENOMINATOR)),
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(110_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(11000_000000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(99000_000000u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(1000_000000u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_011_risk_provide_risk_unmatched_asset_small_reserve() {
        // Provide riskleg
        // Riskleg unmatched asset
        // small reserve UST

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(LUNA, true, 1, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 9, STABLELEG_DENOMINATOR)),
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 1000, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(101_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(10100_000000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([
            (String::from(LUNA), _asset_generator(LUNA, true, 9, STABLELEG_DENOMINATOR)),
        ]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(900_000000u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(100_000000u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_012_risk_provide_big_unmatched_stable() {
        // Provide riskleg
        // Stableleg unmatched asset

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(LUNA, true, 1, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 1000, STABLELEG_DENOMINATOR)),
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(101_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(10100_000000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 900, STABLELEG_DENOMINATOR)),
        ]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(100000_000000u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_013_risk_provide_small_unmatched_stable_big_reserve() {
        // Provide riskleg
        // Stableleg unmatched asset - small
        // Big reserve

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(LUNA, true, 1, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 10, STABLELEG_DENOMINATOR)),
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(101_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(10100_000000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([]);
        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(99910_000000u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(90_000000u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_014_risk_provide_small_unmatched_stable_small_reserve() {
        // Provide riskleg
        // Stableleg unmatched asset - small
        // Small reserve

        let mut before_state = initilaizer();

        let incoming_provide = _asset_generator(LUNA, true, 10, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR)),
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 1000, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            true,
            incoming_provide,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();
        let luna_pair = expected_state.new_virtual_pairs.get_mut(&String::from(LUNA)).unwrap();
        *luna_pair = Pairset{
            riskleg: _asset_generator_raw(LUNA, true, Uint128::from(102_000000u128)),
            riskleg_denominator: 6,
            stableleg: _asset_generator_raw(UUSD, true, Uint128::from(10200_000000u128)),
        };

        expected_state.new_unmatched_assets = HashMap::from([
            (String::from(LUNA), _asset_generator_raw(LUNA, true, Uint128::from(8_000000u128))),
        ]);

        expected_state.new_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(900_000000u128));
        expected_state.new_used_reserved_asset = _asset_generator_raw(UUSD, true, Uint128::from(100_000000u128));

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_015_stable_withdraw_unmatched_big_stable_asset() {
        // Withdraw stableleg
        // Stablelg in the unmatched asset

        let mut before_state = initilaizer();

        let outgoing_withdraw = _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 200, STABLELEG_DENOMINATOR)),
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            false,
            outgoing_withdraw,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();

        expected_state.new_unmatched_assets = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 100, STABLELEG_DENOMINATOR)),
        ]);

        expected_state.new_reserved_asset = _asset_generator(UUSD, true, 100000, STABLELEG_DENOMINATOR);

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    #[test]
    fn test_balancer_016_stable_withdraw_unmatched_small_stable_asset_big_reserve() {
        // Withdraw stableleg
        // Stablelg in the unmatched asset, small
        // Big reserve

        let mut before_state = initilaizer();

        let outgoing_withdraw = _asset_generator(UUSD, true, 111110, STABLELEG_DENOMINATOR);
        let unmatched_asset = HashMap::from([
            (String::from(UUSD), _asset_generator(UUSD, true, 10000, STABLELEG_DENOMINATOR)),
        ]);
        let reserved_ust = _asset_generator(UUSD, true, 101_110_000, STABLELEG_DENOMINATOR);

        before_state.new_unmatched_assets = unmatched_asset;
        before_state.new_reserved_asset = reserved_ust;

        let after_state = calculate_balanced_assets(
            false,
            outgoing_withdraw,
            before_state.new_virtual_pairs,
            before_state.new_unmatched_assets,
            before_state.new_reserved_asset,
            before_state.new_used_reserved_asset,
            before_state.reserve_usage_ratio,
        ).unwrap();

        let mut expected_state = initilaizer();

        expected_state.new_unmatched_assets = HashMap::from([]);

        expected_state.new_reserved_asset = _asset_generator(UUSD, true, 101_008_890, STABLELEG_DENOMINATOR);
        expected_state.new_used_reserved_asset = _asset_generator(UUSD, true, 101_110, STABLELEG_DENOMINATOR);

        _state_print(&after_state, &expected_state);
        assert_eq!(after_state, expected_state);
    }

    fn _state_print(
        after_state: &NewCalculatedBalacedAssets,
        expected_state: &NewCalculatedBalacedAssets) {

        println!("Expected state:");
        println!("{}", expected_state);
        println!();
        println!("Actual:");
        println!("{}", after_state);
    }
}
