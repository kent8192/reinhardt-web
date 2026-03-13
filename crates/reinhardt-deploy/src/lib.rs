#![warn(missing_docs)]

//! # Reinhardt Deploy
//!
//! Deployment utilities and Dockerfile generation for Reinhardt applications.
//!
//! This crate provides tools for generating deployment configurations,
//! including Dockerfile templates for various deployment targets.
//!
//! ## Features
//!
//! - `wasm-deploy`: Enable WASM frontend deployment support with
//!   trunk-based builds and nginx serving

/// Frontend deployment configuration types
pub mod config;
mod error;
mod template;

#[cfg(feature = "wasm-deploy")]
pub mod wasm;

pub use config::FrontendConfig;
pub use error::DeployError;
pub use template::DockerfileGenerator;
