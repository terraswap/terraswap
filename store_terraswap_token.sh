
cp ./artifacts/terraswap_token.wasm ./contracts/terraswap_token/artifacts
terrain contract:store terraswap_token --signer test --network localterra --no-rebuild \
--config-path ./token_config.terrain.json