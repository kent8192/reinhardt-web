//! # Reinhardt Configuration Framework
//!
//! Django-inspired settings management for Rust with secrets, encryption, and audit logging.
//!
//! This crate provides a comprehensive configuration management framework for Reinhardt applications,
//! inspired by Django's settings system with additional security features.
//!
//! ## Features
//!
//! - **Multiple configuration sources**: Files, environment variables, command-line arguments
//! - **Type-safe settings**: Strong type validation with custom validators
//! - **Secrets management**: Integration with HashiCorp Vault, AWS Secrets Manager, Azure Key Vault
//! - **Encryption**: Built-in encryption for sensitive settings
//! - **Dynamic backends**: Redis and database-backed dynamic settings
//! - **Secret rotation**: Automatic secret rotation support
//! - **Audit logging**: Track all setting changes
//!
//! ## Quick Start
//!
//! ```rust
//! # // This documentation test is skipped because it does not use the actual filesystem
//! # fn main() {}
//! ```
//!
//! ## Module Organization
//!
//! - [`settings`]: Core settings management functionality
//! - [`settings_cli`]: CLI tool for managing settings

#![cfg_attr(not(feature = "settings"), allow(unused_imports))]

// Re-export internal crates
#[cfg(feature = "settings")]
pub use reinhardt_settings as settings;

#[cfg(feature = "settings-cli")]
pub use reinhardt_settings_cli as settings_cli;

// Re-export commonly used types at the crate root for convenience
#[cfg(feature = "settings")]
pub use reinhardt_settings::Settings;
