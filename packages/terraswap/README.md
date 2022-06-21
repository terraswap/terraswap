# Terraswap: Common Types

This is a collection of common types and the queriers which are commonly used in terraswap contracts.

## Data Types

### AssetInfo

AssetInfo is a convience wrapper to represent the native token and the contract token as a single type.

```rust
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    Token { contract_addr: HumanAddr },
    NativeToken { denom: String },
}
```

### Asset

It contains asset info with the amount of token. 

```rust
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}
```

### PairInfo

It is used to represent response data of [Pair-Info-Querier](#Pair-Info-Querier)

```rust
pub struct PairInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: String,
    pub liquidity_token: String,
    pub asset_decimals: [u8; 2],
}
```
## Queriers

### Native Token Balance Querier

It uses CosmWasm standard interface to query the account balance to chain.

```rust
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128>
```

### Token Balance Querier

It provides simliar query interface with [Native-Token-Balance-Querier](Native-Token-Balance-Querier) for CW20 token balance. 

```rust
pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128>
```

### Token Info Querier

It provides token info querier for CW20 token contract.

```rust
pub fn query_token_info(
    querier: &QuerierWrapper,
    contract_addr: Addr,
) -> StdResult<TokenInfoResponse>
```

### Native Token Deimals Querier

It provides native token decimals querier for factory contract.

```rust
pub fn query_native_decimals(
    querier: &QuerierWrapper,
    factory_contract: Addr,
    denom: String,
) -> StdResult<u8>
```

### Pair Info Querier From Factory

It also provides the query interface to query avaliable terraswap pair contract info. Any contract can query pair info to terraswap factory contract.

```rust
pub fn query_pair_info(
    querier: &QuerierWrapper,
    factory_contract: Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo>
```

### Pair Info Querier From Pair

It also provides the query interface to query avaliable terraswap pair contract info. Any contract can query pair info to pair contract.

```rust
pub fn query_pair_info_from_pair(
    querier: &QuerierWrapper,
    pair_contract: Addr,
) -> StdResult<PairInfo>
```