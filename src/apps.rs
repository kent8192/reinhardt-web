//! Application configuration and registry module.
//!
//! This module provides access to the application configuration and
//! registry system in Reinhardt.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::apps::{AppConfig, Apps};
//!
//! let app_config = AppConfig::new("myapp");
//! ```

#[cfg(feature = "core")]
pub use reinhardt_core::apps::*;
