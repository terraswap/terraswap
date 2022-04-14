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
                                                    .map(|(k, v)|{
                                                        format!("{} / {}", *k , *v)
                                                    }).collect();
        new_virtual_pairs_vec.sort();
        let new_virtual_pairs_str = new_virtual_pairs_vec.join("\n");

        let mut new_unmatched_assets_vec: Vec<String> = self.new_unmatched_assets.iter()
                                                    .map(|(k, v)|{
                                                        format!("{} / {}", *k, *v)
                                                    }).collect();
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

    // provide
    if is_provide {
        // If the protocol has used the reserved UST + the provided currency is stableleg
        // -> pay back first
        let mut is_stableleg_provide = false;
        match asset.info {
            AssetInfo::NativeToken{ denom } => match denom.as_str() {
                // TODO: how to treat 'ukrw' + how to avoid 'uluna' & IBC tokens
                UUSD => {
                    is_stableleg_provide = true;
                    if used_reserved_asset.amount > Uint128::zero() {
                        (temp_input_asset, res) = reserve_asset_process(temp_input_asset, res);
                    }
                },
                _ => (),
            },
            _ => (),
        };

        // If paid back complete, nothing to do. Done it.
        if temp_input_asset.amount == Uint128::zero() { return Ok(res); }

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
        balanced_assets_info.new_virtual_pairs.clone(),
        balanced_assets_info.new_unmatched_assets.clone(),
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
    }

    let mut is_unmatched_riskleg = false;
    for (_, unmatched_unit_asset) in balanced_assets_info.new_unmatched_assets.iter_mut() {
        if unmatched_unit_asset.amount > Uint128::zero() && get_asset_name(unmatched_unit_asset.clone()) != String::from(UUSD) {
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

        balanced_assets_info.new_reserved_asset.amount += before_reserve_ust.amount - after_reserve_ust.amount;
        balanced_assets_info.new_used_reserved_asset.amount -= before_reserve_ust.amount - after_reserve_ust.amount;
        balanced_assets_info.new_virtual_pairs = new_virtual_pairs;
    }

    // Delete 0-amount entry
    balanced_assets_info.new_unmatched_assets.retain(|_, v| v.amount != Uint128::zero() );

    balanced_assets_info
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

        let ust_portion = provided_ust.amount
                            .checked_mul(
                                *portions.get(token_name)
                                         .unwrap()
                            ).unwrap()
                            .checked_div( get_pow10(STABLELEG_DENOMINATOR) ).unwrap();

        let curr_pairset = pairset.get_mut(token_name).unwrap();
        let ratio = derive_unit_ratio(curr_pairset);
        let riskleg_ust_value = unmatched_unit_asset.amount
                                .checked_mul(ratio).unwrap()
                                .checked_div( get_pow10(STABLELEG_DENOMINATOR) ).unwrap();

        if ust_portion > riskleg_ust_value {
            curr_pairset.stableleg.amount += riskleg_ust_value;
            curr_pairset.riskleg.amount += unmatched_unit_asset.amount;
            unmatched_unit_asset.amount = Uint128::zero();
            remain_ust.amount += ust_portion - riskleg_ust_value;
        } else {
            curr_pairset.stableleg.amount += ust_portion;

            let provided_riskleg_amount = ust_portion
                                            .checked_mul( get_pow10(STABLELEG_DENOMINATOR) ).unwrap()
                                            .checked_div(unmatched_unit_asset.amount).unwrap();
            curr_pairset.riskleg.amount += provided_riskleg_amount;

            unmatched_unit_asset.amount -= provided_riskleg_amount;
        }
    }

    (remain_ust, pairset)
}

fn calculate_weight_unmatched_assets(
    curr_pairs: HashMap<String, Pairset>,
    unmatched_assets: HashMap<String, Asset>
) -> StdResult<HashMap<String, Uint128>> {
    let mut res: HashMap<String, Uint128> = HashMap::new();
    let mut whole_portion = Uint128::from(0u128);

    for (token_name, unit_asset) in unmatched_assets.iter() {
        if *token_name == String::from(UUSD) { continue; }

        let pairset = match curr_pairs.get(token_name) {
            Some(pairinfo) => pairinfo,
            None => return Err(StdError::not_found(format!("no pair info {}", token_name))),
        };

        let ratio = derive_unit_ratio(pairset);
        let portion = unit_asset.amount.checked_mul(ratio).unwrap()
                                .checked_div(get_pow10(STABLELEG_DENOMINATOR)).unwrap();
        whole_portion += portion.clone();

        res.insert(token_name.clone(), ratio);
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

fn derive_unit_ratio(
    unit_pair: &Pairset
) -> Uint128 {
    // Riskleg amount * 10^6 / stableleg
    // 10^6 is already multiplied
    if unit_pair.riskleg_denominator < STABLELEG_DENOMINATOR {
        unit_pair.stableleg.amount
            .checked_mul( get_pow10(2 * STABLELEG_DENOMINATOR - unit_pair.riskleg_denominator) ).unwrap()
            .checked_div(unit_pair.riskleg.amount).unwrap()
    } else {
        unit_pair.stableleg.amount
            .checked_mul( get_pow10(unit_pair.riskleg_denominator) ).unwrap()
            .checked_div(unit_pair.riskleg.amount).unwrap()
    }
}

fn get_asset_name(asset: Asset) -> String {
    match asset.info {
        AssetInfo::NativeToken{ denom } => denom,
        AssetInfo::Token{ contract_addr } => contract_addr,      
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
            reserve_usage_ratio: Uint128::from(10000u128), // 10%
        }
    }

    #[test]
    fn test001_stable_provide_unmatched_stable() {
        // Provide stabeleg
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
    fn test002_stable_provide_unmatched_risk_1() {
        // Provide stabeleg
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
