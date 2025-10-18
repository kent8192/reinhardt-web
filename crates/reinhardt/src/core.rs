//! Core framework types and utilities module.
//!
//! This module provides access to core types, exception handling,
//! signals, macros, security utilities, and validators.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::core::exception::Error;
//! use reinhardt::core::signals::{Signal, pre_save};
//! use reinhardt::core::validators::EmailValidator;
//! ```

#[cfg(feature = "core")]
pub use reinhardt_core::*;
