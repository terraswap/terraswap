
cp ./artifacts/terraswap_factory.wasm ./contracts/terraswap_factory/artifacts
terrain contract:store terraswap_factory --signer pisco --network testnet --no-rebuild \
--config-path ./factory_config.terrain.json
