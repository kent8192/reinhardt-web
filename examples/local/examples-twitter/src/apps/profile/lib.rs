//! profile application module
//!
//! A RESTful API application for user profiles

use reinhardt::AppConfig;

pub mod admin;
pub mod models;
pub mod serializers;
pub mod urls;
pub mod views;

#[derive(AppConfig)]
#[app_config(name = "profile", label = "profile", verbose_name = "User Profiles")]
pub struct ProfileConfig;
