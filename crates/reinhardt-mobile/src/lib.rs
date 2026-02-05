//! # reinhardt-mobile
//!
//! Mobile application support for reinhardt-manouche DSL.
//!
//! This crate provides Android and iOS support by consuming
//! reinhardt-manouche IR and generating mobile applications
//! using wry/tao for WebView embedding.
//!
//! ## Features
//!
//! - `experimental` - Enable experimental mobile features
//!
//! ## Platform Support
//!
//! - Android: API 26+ (Android 8.0 Oreo)
//! - iOS: iOS 13.0+

// Core modules
mod core;
mod error;

// Platform-specific modules (Phase 2+)
mod codegen;
mod platform;
mod runtime;

// Public API
pub use codegen::*;
pub use core::*;
pub use error::*;
pub use platform::*;
pub use runtime::*;
