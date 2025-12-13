//! Application configuration and registry module.
//!
//! This module provides access to the application configuration and
//! registry system in Reinhardt.
//!
//! # Examples
//!
//! ```rust,no_run
//! use reinhardt::apps::AppConfig;
//!
//! let app_config = AppConfig::new("myapp");
//! ```

// Re-export from reinhardt-apps crate
pub use reinhardt_apps::*;
