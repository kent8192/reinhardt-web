//! Users application
//!
//! Provides session-based authentication for the tutorial-basis example.
//! Defines a minimal `User` model and exposes server functions for login,
//! logout, sign-up, and current-user introspection via
//! `crate::apps::users::server_fn`.

#[cfg(server)]
pub mod models;

#[cfg(client)]
pub mod client;
pub mod server_fn;
pub mod urls;
