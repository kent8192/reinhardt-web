//! Plugin system module.
//!
//! This module provides the Dentdelion plugin system for extending
//! Reinhardt applications with static, WASM, and TypeScript plugins.
//!
//! # Examples
//!
//! ```rust,no_run
//! use reinhardt::dentdelion::{Plugin, PluginRegistry};
//! ```

#[cfg(feature = "dentdelion")]
pub use reinhardt_dentdelion::*;
