//! dm application module
//!
//! A RESTful API application for direct messages

use reinhardt::AppConfig;

pub mod admin;
pub mod models;
pub mod serializers;
pub mod urls;
pub mod views;

#[derive(AppConfig)]
#[app_config(name = "dm", label = "dm", verbose_name = "Direct Messages")]
pub struct DmConfig;
