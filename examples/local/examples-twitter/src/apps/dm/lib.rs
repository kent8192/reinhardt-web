//! dm application module
//!
//! Direct message models for examples-twitter

use reinhardt::AppConfig;

pub mod admin;
pub mod models;

#[derive(AppConfig)]
#[app_config(name = "dm", label = "dm", verbose_name = "Direct Messages")]
pub struct DmConfig;
