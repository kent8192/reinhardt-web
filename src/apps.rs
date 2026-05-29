//! Application configuration and registry module.
//!
//! This module provides access to the application configuration and
//! registry system in Reinhardt.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::apps::AppConfig;
//!
//! let app_config = AppConfig::new("myapp", "myapp");
//! ```

// Re-export from reinhardt-apps crate
pub use reinhardt_apps::*;
