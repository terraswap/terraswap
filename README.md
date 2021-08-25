# TerraSwap

Uniswap-inspired automated market-maker (AMM) protocol powered by Smart Contracts on the [Terra](https://terra.money) blockchain.

## Contracts

| Name                                               | Description                                  |
| -------------------------------------------------- | -------------------------------------------- |
| [`terraswap_factory`](contracts/terraswap_factory) |                                              |
| [`terraswap_pair`](contracts/terraswap_pair)       |                                              |
| [`terraswap_token`](contracts/terraswap_token)     | CW20 (ERC20 equivalent) token implementation |

* terraswap_factory

   Mainnet: `terra1ulgw0td86nvs4wtpsc80thv6xelk76ut7a7apj`

   Testnet: `terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf`

* terraswap_pair

   Mainnet (CodeID): 4

   Testnet (CodeID): 7869

* terraswap_token

   Mainnet (CodeID): 3

   Testnet (CodeID): 148

## Running this contract

You will need Rust 1.44.1+ with wasm32-unknown-unknown target installed.

You can run unit tests on this on each contracts directory via :

```
cargo unit-test
cargo integration-test
```

Once you are happy with the content, you can compile it to wasm on each contracts directory via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_subkeys.wasm .
ls -l cw1_subkeys.wasm
sha256sum cw1_subkeys.wasm
```

Or for a production-ready (compressed) build, run the following from the repository root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.10.2
```

The optimized contracts are generated in the artifacts/ directory.
