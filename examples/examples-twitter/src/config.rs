//! Configuration module for examples-twitter

#[cfg(server)]
pub mod admin;
pub mod apps;
#[cfg(server)]
pub mod middleware;
#[cfg(server)]
pub mod settings;
pub mod urls;
#[cfg(server)]
pub mod wasm;
