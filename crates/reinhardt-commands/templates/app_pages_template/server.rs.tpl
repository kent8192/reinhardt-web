//! Server-side modules for the {{ app_name }} application.
//!
//! Reached only on native targets via `#[cfg(server)] pub mod server;` in the
//! parent app aggregator. Keep native-only forms, views, and route
//! implementation details under `server/*.rs`. Shared models live at the app
//! root so the generated info DTOs can also compile for the WASM target.

pub mod admin;
pub mod forms;
pub mod views;
