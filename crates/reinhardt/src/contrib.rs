//! Contributed applications and features module.
//!
//! This module provides access to optional contributed features like
//! authentication, sessions, messages, static files, mail, GraphQL,
//! WebSockets, i18n, and management commands.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::contrib::auth::{JwtAuth, User};
//! use reinhardt::contrib::sessions::Session;
//! use reinhardt::contrib::mail::EmailBackend;
//! ```

#[cfg(feature = "contrib")]
pub use reinhardt_contrib::*;
