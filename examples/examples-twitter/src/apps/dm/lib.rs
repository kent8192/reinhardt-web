//! dm application module
//!
//! Direct message models for examples-twitter
#[cfg(native)]
use reinhardt::app_config;
#[cfg(native)]
pub mod admin;
#[cfg(wasm)]
pub mod client;
#[cfg(native)]
pub mod models;
#[cfg(native)]
pub mod server;
pub mod shared;
#[cfg(test)]
pub mod tests;
pub mod urls;
#[cfg(native)]
#[app_config(name = "dm", label = "dm", verbose_name = "Direct Messages")]
pub struct DmConfig;
