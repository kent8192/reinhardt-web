//! profile application module
//!
//! User profile models for examples-twitter

use reinhardt::AppConfig;

pub mod admin;
pub mod models;

#[derive(AppConfig)]
#[app_config(name = "profile", label = "profile", verbose_name = "User Profiles")]
pub struct ProfileConfig;
