//! Server-side facade for the {{ app_name }} application.
//!
//! Reached only on native targets via `#[cfg(server)] pub mod server;` in the
//! parent app aggregator. Keep native-only models, serializers, views, and
//! route implementation details under `server/*.rs`.

pub mod admin;
pub mod models;
pub mod serializers;
pub mod views;
