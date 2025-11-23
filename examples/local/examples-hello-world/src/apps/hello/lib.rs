//! hello application module
//!
//! A simple hello world application

use reinhardt::AppConfig;

pub mod urls;
pub mod views;

#[derive(AppConfig)]
#[app_config(name = "hello", label = "hello")]
pub struct HelloConfig;
