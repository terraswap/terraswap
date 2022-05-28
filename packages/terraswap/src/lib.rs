pub mod asset;
pub mod factory;
pub mod pair;
pub mod querier;
pub mod router;
pub mod token;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock_querier;

#[cfg(test)]
mod testing;
