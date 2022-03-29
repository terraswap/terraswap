// Will be moved into terraswap_interface (ToBeFixed)
use cosmwasm_std::{Addr, DepsMut, StdResult, Uint128, Order};
use cw_storage_plus::Map;
use crate::asset::{Asset, AssetInfo, PairInfo};

pub struct NewCalculatedBalacedAssets<'a> {
    pub new_virtual_pairs: Map<'a, &'a str, PairInfo>,
    pub new_unmatched_assets: Map<'a, &'a str, Asset>,
    pub new_reserved_asset: Asset,
}

/// calculate new balanced assets from the existing one & input assets
/// calculate_balanced_assets()
///     -> StdResult<NewCalculatedBalacedAssets>
/// 
/// 1. Summize the pool size of the stableleg
/// 2. Check the type of the input asset
/// 3. Add or sub the asset from the proper pair
/// 4. Derive the expected result
/// 5. Check the demand of the reserved stableleg pool
/// 6-1. If needed, provide max 10% from the reserved pool at onece
/// 6-2. If exceed, make the # of stable asset as the unmatched asset
/// 7. Calibrate the pair & unmatched asset info
pub fn calculate_balanced_assets(
    deps: DepsMut,
    virtual_pairs: Map<&str, PairInfo>,
    unmatched_assets: Map<&str, Asset>,
    is_provide: bool,
    asset: Asset,
    reserved_asset: Asset,
) -> StdResult<NewCalculatedBalacedAssets> {
    // 1. Summize the pool size fo the stableleg
    let mut whole_stableleg_size = Uint128::from(0u128);
    for may_unit_pair in virtual_pairs.range(deps.storage, None, None, Order::Ascending) {
        let unit_pair = may_unit_pair.expect("Wrong asset info is given").1;

        for unit_asset in unit_pair.asset_infos {
            // this "1" does not mean 1st asset, but asset info from enumerated array
            match unit_asset {
                AssetInfo::NativeToken{ denom } => match denom.as_str() {
                    "uusd" => {
                        let unit_pool_addr: Addr = deps.api.addr_validate(unit_pair.contract_addr.as_str())?;
                        let uusd_amount = unit_asset.query_pool(&deps.querier, deps.api, unit_pool_addr)?;
                        whole_stableleg_size += uusd_amount;
                    },
                    _ => (),
                },
                _ => (),
            }
        }
    }

    // 2. Check the type of the input asset
    // match asset.info {
    //     AssetInfo::NativeToken{ denom } => match denom.as_str() {
    //         "uusd" | "ukrw" => whole_stableleg_size += asset.amount,
    //         _ => (),
    //     },
    //     AssetInfo::Token{ contract_addr } => (),
    // };

    // return;
}
