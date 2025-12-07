//! Dependency injection module.
//!
//! This module provides FastAPI-style dependency injection system
//! and parameter extraction.
//!
//! # Examples
//!
//! ```rust,no_run
//! use reinhardt::di::{Injected, Injectable};
//! ```

#[cfg(feature = "di")]
pub use reinhardt_core::di::*;

// Re-export reinhardt-di types for macro compatibility
#[cfg(feature = "di")]
pub use reinhardt_di::{
	DiError, DiResult, Injectable, Injected, InjectionContext, InjectionMetadata, OptionalInjected,
};
