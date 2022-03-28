// Will be moved into terraswap_interface (ToBeFixed)
use cosmwasm_std::StdResult;
use cw_storage_plus::Map;
use crate::asset::{Asset, PairInfo};

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
    virtual_pairs: Map<&str, PairInfo>,
    unmatched_assets: Map<&str, Asset>,
    is_provide: bool,
    asset: Asset,
    reserved_asset: Asset,
) -> StdResult<NewCalculatedBalacedAssets> {

}
