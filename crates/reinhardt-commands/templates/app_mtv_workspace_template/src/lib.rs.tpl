//! {{ app_name }} application crate
//!
//! A Model-Template-View application

use reinhardt::AppConfig;

pub mod admin;
pub mod models;
pub mod urls;
pub mod views;

#[derive(AppConfig)]
#[app_config(name = "{{ app_name }}", label = "{{ app_name }}")]
pub struct {{ camel_case_app_name }}Config;
