# TerraSwap

Uniswap-inspired automated market-maker (AMM) protocol powered by Smart Contracts on the [Terra](https://terra.money) blockchain.

## Contracts

| Name                                               | Description                                  |
| -------------------------------------------------- | -------------------------------------------- |
| [`terraswap_factory`](contracts/terraswap_factory) |                                              |
| [`terraswap_pair`](contracts/terraswap_pair)       |                                              |
| [`terraswap_router`](contracts/terraswap_router)   |                                              |
| [`terraswap_token`](contracts/terraswap_token)     | CW20 (ERC20 equivalent) token implementation |

* terraswap_factory

   Mainnet: `terra1ulgw0td86nvs4wtpsc80thv6xelk76ut7a7apj`

   Testnet: `terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf`

* terraswap_pair

   Mainnet (CodeID): 4

   Testnet (CodeID): 155

* terraswap_token

   Mainnet (CodeID): 3

   Testnet (CodeID): 148

* terraswap_router

   Mainnet: `terra19f36nz49pt0a4elfkd6x7gxxfmn3apj7emnenf`

   Testnet: `terra1c58wrdkyc0ynvvxcv834kz65nfsxmw2w0pwusq`

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
  cosmwasm/workspace-optimizer:0.12.4
```

The optimized contracts are generated in the artifacts/ directory.
