
cp ./artifacts/terraswap_pair.wasm ./contracts/terraswap_pair/artifacts
terrain contract:store terraswap_pair --signer pisco --network testnet --no-rebuild \
--config-path ./pair_config.terrain.json