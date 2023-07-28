# TerraSwap Pair

## Handlers

### Initialize

This is mainly used from terraswap factory contract to create new terraswap pair. It initializes all swap created parameters which can be updated later with owner key.

It creates liquidity token contract as init response, and execute init hook to register created liquidity token contract to self.

```rust
{
    /// Asset infos
    pub asset_infos: [AssetInfo; 2],
    /// Token code ID for liqudity token creation
    pub token_code_id: u64,
    /// Hook for post initalization
    pub init_hook: Option<InitHook>,
}
```

### Liquidity Provider

The contract has two types of pool, the one is collateral and the other is asset pool. A user can provide liquidity to each pool by sending `provide_liquidity` msgs and also can withdraw with `withdraw_liquidity` msgs.

Whenever liquidity is deposited into a pool, special tokens known as liquidity tokens are minted to the provider’s address, in proportion to how much liquidity they contributed to the pool. These tokens are a representation of a liquidity provider’s contribution to a pool. Whenever a trade occurs, the `lp_commission%` of fee is distributed pro-rata to all LPs in the pool at the moment of the trade. To receive the underlying liquidity back, plus commission fees that were accrued while their liquidity was locked, LPs must burn their liquidity tokens.

When providing liquidity from a smart contract, tokens deposited into a pool at a rate different from the current oracle price will be returned to users.

> Note before executing the `provide_liqudity` operation, a user must allow the contract to use the liquidity amount of asset in the token contract.

#### Receiver

If a user specifies the `receiver` at `provide_liqudity` msg, sends LP token to receiver. The default value is sender.

#### Min Assets

If a user specifies the `min_assets` at `withdraw_liquidity` msg, the contract restricts the operation when the returned assets are less than the min assets.

#### Deadline

A `deadline` sets a time after which a transaction can no longer be executed. This limits validators holding signed transactions for extended durations and executing them based off market movements. It also reduces uncertainty around transactions that take a long time to execute due to issues with gas price.

#### Request Format

- Provide Liquidity

  ```json
  {
    "provide_liquidity": {
      "assets": [
        {
          "info": {
            "token": {
              "contract_addr": "terra..."
            }
          },
          "amount": "1000000"
        },
        {
          "info": {
            "native_token": {
              "denom": "uluna"
            }
          },
          "amount": "1000000"
        }
      ]
    }
  }
  ```

- Withdraw Liquidity (must be sent to liquidity token contract)
  1. With Min Assets

  ```json
  {
    "withdraw_liquidity": {
      "min_assets": [
        {
          "info": {
            "token": {
              "contract_addr": "terra..."
            }
          },
          "amount": "1000000"
        },
        {
          "info": {
            "native_token": {
              "denom": "uluna"
            }
          },
          "amount": "1000000"
        }
      ]
    }
  }
  ```

  2. Without Min Assets

  ```json
  {
    "withdraw_liquidity": {}
  }
  ```

### Swap

Any user can swap an asset by sending `swap` or invoking `send` msg to token contract with `swap` hook message.

- Native Token => Token

  ```json
  {
      "swap": {
          "offer_asset": {
              "info": {
                  "native_token": {
                      "denom": String
                  }
              },
              "amount": Uint128
          },
          "belief_price": Option<Decimal>,
          "max_spread": Option<Decimal>,
          "to": Option<HumanAddr>
      }
  }
  ```

- Token => Native Token

  **Must be sent to token contract**

  ```json
  {
      "send": {
          "contract": HumanAddr,
          "amount": Uint128,
          "msg": Binary({
              "swap": {
                  "belief_price": Option<Decimal>,
                  "max_spread": Option<Decimal>,
                  "to": Option<HumanAddr>
              }
          })
      }
  }
  ```

#### Swap Spread

The spread is determined with following uniswap mechanism:

```rust
// -max_minus_spread < spread < max_spread
// minus_spread means discount rate.
// Ensure `asset pool * collateral pool = constant product`
let cp = Uint128(offer_pool.u128() * ask_pool.u128());
let return_amount = offer_amount * exchange_rate;
let return_amount = (ask_pool - cp.multiply_ratio(1u128, offer_pool + offer_amount))?;


// calculate spread & commission
let spread_amount: Uint128 =
    (offer_amount * Decimal::from_ratio(ask_pool, offer_pool) - return_amount)?;
let lp_commission: Uint128 = return_amount * config.lp_commission;
let owner_commission: Uint128 = return_amount * config.owner_commission;

// commission will be absorbed to pool
let return_amount: Uint128 =
    (return_amount - (lp_commission + owner_commission)).unwrap();
```

#### Commission

The `lp_commission` remains in the swap pool, which is fixed to `0.3%`, causing a permanent increase in the constant product K. The value of this permanently increased pool goes to all LPs.

