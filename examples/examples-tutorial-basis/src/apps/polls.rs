//! Polls application - Tutorial basis example
//!
//! This app demonstrates:
//! - Database models (Question, Choice)
//! - Views and URL routing
//! - Forms and generic views
//! - Admin panel integration

#[cfg(native)]
use reinhardt::app_config;

#[cfg(native)]
pub mod admin;
#[cfg(wasm)]
pub mod client;
#[cfg(native)]
pub mod di;
#[cfg(native)]
pub mod models;
#[cfg(native)]
pub mod serializers;
pub mod server_fn;
pub mod urls;

#[cfg(native)]
#[app_config(name = "polls", label = "polls")]
pub struct PollsConfig;
