
cp ./artifacts/terraswap_router.wasm ./contracts/terraswap_router/artifacts
terrain contract:store terraswap_router --signer test --network localterra --no-rebuild \
--config-path ./router_config.terrain.json