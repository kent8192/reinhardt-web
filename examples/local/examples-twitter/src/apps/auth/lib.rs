//! auth application module
//!
//! A RESTful API application for user authentication

use reinhardt::AppConfig;

pub mod admin;
pub mod models;
pub mod serializers;
pub mod urls;
pub mod views;

#[derive(AppConfig)]
#[app_config(name = "auth", label = "auth", verbose_name = "Authentication")]
pub struct AuthConfig;
