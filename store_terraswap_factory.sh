
cp ./artifacts/terraswap_factory.wasm ./contracts/terraswap_factory/artifacts
terrain contract:store terraswap_factory --signer test --network localterra --no-rebuild \
--config-path ./factory_config.terrain.json
