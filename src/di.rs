//! Dependency injection module.
//!
//! This module provides FastAPI-style dependency injection system
//! and parameter extraction.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::di::{Depends, Injectable};
//! use reinhardt::di::params::{Path, Query, Json};
//! ```

#[cfg(feature = "di")]
pub use reinhardt_core::di::*;
