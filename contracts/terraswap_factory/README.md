# TerraSwap Factory

The factory contract can perform creation of terraswap pair contract and also be used as directory contract for all pairs.

## InstantiateMsg
Register verified pair contract and token contract for pair contract creation. The sender will be the owner of the factory contract.

```json
{
  "pair_code_id": 123,
  "token_code_id": 123,
  "init_hook": {
    "msg": "123",
    "contract_addr": "terra..."
  }
}
```

## ExecuteMsg

### `update_config`
Change the factory contract's owner and relevant code IDs for future pair contract creation. This execution is only permitted to the factory contract owner.

```json
{
  "update_config": {
    "owner": "terra...",
    "token_id": 123,
    "pair_code_id": 123
  }
}
```

### `create_pair`
When a user executes `CreatePair` operation, it creates `Pair` contract and `LP(liquidity provider)` token contract.

In order to create pairs with native tokens, including IBC tokens, they must first be registered with their decimals by the factory contract owner. See [add_native_token_decimals](#add_native_token_decimals) for more details.

```json
{
  "create_pair": {
    "assets": [
      {
        "info": {
          "token": {
            "contract_addr": "terra..."
          }
        },
        "amount": "0"
      },
      {
        "info": {
          "native_token": {
            "denom": "uluna"
          }
        },
        "amount": "0"
      }
    ]
  }
}
```

### `add_native_token_decimals`
This operation which is only allowed for the factory contract owner, registers native tokens (including IBC tokens) along with their decimals.

The contract will create a new pair using the provided token information if the pair contains a token registered by this operation,

```json
{
  "add_native_token_decimals": {
    "denom": "uluna",
    "decimals": 6
  }
}
```

### `migrate_pair`

```json
{
  "migrate_pair": {
    "contract": "terra...",
    "code_id": 123
  }
}
```

## QueryMsg

### `config`

```json
{
  "config": {}
}
```

### `pair`

```json
{
  "pair": {
    "asset_infos": [
      {
        "token": {
          "contract_addr": "terra..."
        }
      },
      {
        "native_token": {
          "denom": "uluna"
        }
      }
    ]
  }
}
```

### `pairs`

```json
{
  "pairs": {
    "start_after": [
      {
        "token": {
          "contract_addr": "terra..."
        }
      },
      {
        "native_token": {
          "denom": "uluna"
        }
      }
    ],
    "limit": 10
  }
}
```

### `native_token_decimals`
```json
{
  "native_token_decimals": {
    "denom": "uluna"
  }
}
```
