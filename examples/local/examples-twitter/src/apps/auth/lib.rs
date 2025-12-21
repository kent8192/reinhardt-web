//! auth application module
//!
//! User authentication models for examples-twitter

use reinhardt::AppConfig;

pub mod models;

#[derive(AppConfig)]
#[app_config(name = "auth", label = "auth", verbose_name = "Authentication")]
pub struct AuthConfig;
