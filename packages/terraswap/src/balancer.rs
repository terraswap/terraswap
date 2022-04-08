// Will be moved into terraswap_interface (ToBeFixed)
use cosmwasm_std::{Addr, DepsMut, StdResult, Uint128, Order, StdError};
use cw_storage_plus::Map;
use crate::asset::{Asset, AssetInfo, PairInfo};

#[derive(Clone)]
pub struct NewCalculatedBalacedAssets<'a> {
    pub new_virtual_pairs: Map<'a, &'a str, PairInfo>,
    pub new_unmatched_assets: Map<'a, &'a str, Asset>,
    pub new_reserved_asset: Asset,
    pub new_used_reserved_asset: Asset,
}

/// calculate new balanced assets from the existing one & input assets
/// calculate_balanced_assets()
///     -> StdResult<NewCalculatedBalacedAssets>
/// 
/// Check https://www.notion.so/delight-labs/beb67d49bdda488fb222bf56ffa9f2ed#de128f3c50cd42bcb71b17aa53429245
/// TODO: should change the link into the official docs
pub fn calculate_balanced_assets<'a>(
    deps: DepsMut<'a>,
    is_provide: bool,
    asset: Asset,
    virtual_pairs: &'a Map<'a, &str, PairInfo>,
    unmatched_assets: &'a Map<'a, &str, Asset>,
    reserved_asset: Asset,
    used_reserved_asset: Asset,
) -> StdResult<NewCalculatedBalacedAssets<'a>> {
    let mut temp_input_asset = asset.clone();
    let mut res = NewCalculatedBalacedAssets::<'a> {
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
                    "uusd" => {
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

        if (is_stableleg_provide && res.new_unmatched_assets.has(deps.storage, "uusd")) || 
            (!is_stableleg_provide && !res.new_unmatched_assets.has(deps.storage, "uusd")) {
            
            res = put_unmatched_asset(deps, vec![temp_input_asset], res);
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
    mut res: NewCalculatedBalacedAssets
) -> (Asset, NewCalculatedBalacedAssets) {
    if res.new_used_reserved_asset.amount >= input_asset.amount {
        res.new_used_reserved_asset.amount -= input_asset.amount;
        res.new_reserved_asset.amount += input_asset.amount;
        input_asset.amount = Uint128::from(0u128);
    } else {
        res.new_reserved_asset.amount += res.new_used_reserved_asset.amount;
        input_asset.amount -= res.new_used_reserved_asset.amount;
        res.new_used_reserved_asset.amount = Uint128::from(0u128);
    }

    (input_asset, res)
}

fn put_unmatched_asset<'a>(
    deps: DepsMut,
    input_assets: Vec<Asset>,
    res: NewCalculatedBalacedAssets<'a>
) -> NewCalculatedBalacedAssets<'a> {
    for unit_asset in input_assets.iter() {
        let token_name = match &unit_asset.info {
            AssetInfo::NativeToken{ denom } => denom,
            AssetInfo::Token{ contract_addr } => contract_addr,      
        };

        res.new_unmatched_assets.update(
            deps.storage,
            token_name.as_str(),
            |may_stored_token: Option<Asset>| -> StdResult<Asset> {
                match may_stored_token {
                    Some(stored_token) => {
                        let mut new_token = stored_token.clone();
                        new_token.amount += unit_asset.amount;
                        Ok(new_token)
                    },
                    None => Ok(unit_asset.clone()),
                }
            }
        ).ok();
    }

    res
}
