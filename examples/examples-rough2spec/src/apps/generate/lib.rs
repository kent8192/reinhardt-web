//! generate application module

use reinhardt::app_config;

pub mod urls;
pub mod views;

#[app_config(name = "generate", label = "generate")]
pub struct GenerateConfig;
