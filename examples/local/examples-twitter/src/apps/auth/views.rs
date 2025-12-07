//! Views for auth app
//!
//! This module contains all view handlers for the auth application,
//! organized by functionality

pub mod register;

// Re-export commonly used view handlers
pub use register::register;
