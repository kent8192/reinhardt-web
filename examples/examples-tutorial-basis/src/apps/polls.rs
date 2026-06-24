//! Polls application - Tutorial basis example
//!
//! This app demonstrates:
//! - Database models (Question, Choice)
//! - Views and URL routing
//! - Forms and generic views
//! - Admin panel integration

#[cfg(server)]
use reinhardt::app_config;

#[cfg(client)]
pub mod client;
pub mod models;
#[cfg(server)]
pub mod server;
pub mod server_fn;
pub mod services;
pub mod urls;

#[cfg(server)]
#[app_config(name = "polls", label = "polls")]
pub struct PollsConfig;
