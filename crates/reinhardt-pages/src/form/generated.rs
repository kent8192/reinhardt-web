//! Static Metadata Types for form! Macro Generated Code
//!
//! This module re-exports types from `form_generated` for backwards compatibility.
//! The actual implementation is in `form_generated` module which is always available
//! (on both WASM and server).
//!
//! For new code, prefer importing directly from `form_generated` or the crate root.

// Re-export from form_generated for backwards compatibility
pub use crate::form_generated::{StaticFieldMetadata, StaticFormMetadata};
