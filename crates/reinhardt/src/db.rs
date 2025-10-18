//! Database and ORM module.
//!
//! This module provides access to the database layer, ORM, migrations,
//! hybrid properties, and associations.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::db::orm::{Model, QuerySet};
//! use reinhardt::db::migrations::Migration;
//! use reinhardt::db::hybrid::HybridProperty;
//! ```

#[cfg(feature = "database")]
pub use reinhardt_db::*;
