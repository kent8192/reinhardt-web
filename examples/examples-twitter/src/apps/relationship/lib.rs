//! relationship application module
//!
//! User relationship models for examples-twitter
#[cfg(native)]
use reinhardt::app_config;
#[cfg(wasm)]
pub mod client;
#[cfg(native)]
pub mod server;
pub mod shared;
#[cfg(test)]
pub mod tests;
pub mod urls;
#[cfg(native)]
#[app_config(
	name = "relationship",
	label = "relationship",
	verbose_name = "User Relationships"
)]
pub struct RelationshipConfig;
