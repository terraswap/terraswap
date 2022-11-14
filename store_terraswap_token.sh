
cp ./artifacts/terraswap_token.wasm ./contracts/terraswap_token/artifacts
terrain contract:store terraswap_token --signer pisco --network testnet --no-rebuild \
--config-path ./token_config.terrain.json