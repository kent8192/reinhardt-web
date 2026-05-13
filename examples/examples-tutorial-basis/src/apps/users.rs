//! Users application
//!
//! Provides session-based authentication for the tutorial-basis example.
//! Defines a minimal `User` model and exposes server functions for login,
//! logout, and current-user introspection via `crate::server_fn::users`.

#[cfg(native)]
pub mod models;

pub mod urls;
