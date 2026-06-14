//! # reinhardt-providers
//!
//! Cloud provider integrations for Reinhardt.
//!
//! This crate hosts narrow provider utilities used by higher-level Reinhardt
//! crates. It is not a full cloud SDK; each module should stay focused on the
//! operations Reinhardt directly needs.

#![warn(missing_docs)]

#[cfg(feature = "aws")]
pub mod aws;
pub mod error;
#[cfg(feature = "gcp")]
pub mod gcp;

pub use error::{ProviderError, Result};
