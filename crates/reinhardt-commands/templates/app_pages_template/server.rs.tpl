//! Server-side modules for the {{ app_name }} application.
//!
//! Reached only on native targets via `#[cfg(server)] pub mod server;` in the
//! parent app aggregator. Keep native-only models, forms, views, and route
//! implementation details under `server/*.rs`.

pub mod admin;
pub mod forms;
pub mod models;
pub mod views;
