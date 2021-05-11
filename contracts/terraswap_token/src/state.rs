use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Uint128};
use cw_storage_plus::{Item, Map};

use cw20::AllowanceResponse;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub mint: Option<MinterData>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MinterData {
    pub minter: CanonicalAddr,
    /// cap is how many more tokens can be issued by the minter
    pub cap: Option<Uint128>,
}

impl TokenInfo {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }
}

pub const TOKEN_INFO: Item<TokenInfo> = Item::new("\u{0}\ntoken_info");
pub const BALANCES: Map<&[u8], Uint128> = Map::new("balance");
pub const ALLOWANCES: Map<(&[u8], &[u8]), AllowanceResponse> = Map::new("allowance");

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{StdResult, Storage};
    use cosmwasm_storage::{singleton, singleton_read};
    const KEY_TOKEN_INFO: &[u8] = b"token_info";

    pub fn store_token_info(storage: &mut dyn Storage, token_info: &TokenInfo) -> StdResult<()> {
        singleton(storage, KEY_TOKEN_INFO).save(token_info)
    }
    pub fn read_token_info(storage: &dyn Storage) -> StdResult<TokenInfo> {
        singleton_read(storage, KEY_TOKEN_INFO).load()
    }

    #[test]
    fn token_info_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        store_token_info(
            &mut deps.storage,
            &TokenInfo {
                name: "token".to_string(),
                symbol: "TOK".to_string(),
                decimals: 8,
                total_supply: Uint128::zero(),
                mint: None,
            },
        )
        .unwrap();

        assert_eq!(
            TOKEN_INFO.load(&deps.storage).unwrap(),
            read_token_info(&deps.storage).unwrap()
        );
    }
}
