//! Serializers for auth app
//!
//! This module contains all serializers for the auth application,
//! organized by functionality

pub mod register;

// Re-export commonly used serializers
pub use register::{RegisterRequest, RegisterResponse};
