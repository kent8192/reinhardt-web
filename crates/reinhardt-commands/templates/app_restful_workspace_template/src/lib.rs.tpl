//! {{ app_name }} app module (RESTful)

pub mod models;
pub mod views;
pub mod serializers;
pub mod admin;
pub mod tests;
pub mod urls;
pub mod apps;

pub use apps::{{ camel_case_app_name }}Config;
