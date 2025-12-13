//! Core framework types and utilities module.
//!
//! This module provides access to core types, exception handling,
//! signals, macros, security utilities, and validators.
//!
//! # Examples
//!
//! ```rust,no_run
//! # #[cfg(feature = "core")]
//! use reinhardt::core::exception::Error;
//! # #[cfg(feature = "core")]
//! use reinhardt::core::signals::{Signal, pre_save};
//! ```

#[cfg(feature = "core")]
pub use reinhardt_core::*;
