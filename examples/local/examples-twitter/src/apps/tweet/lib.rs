//! Tweet application module

pub mod admin;
pub mod models;
pub mod shared;
pub mod urls;

#[cfg(target_arch = "wasm32")]
pub mod client;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(test)]
pub mod tests;
