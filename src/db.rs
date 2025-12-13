//! Database and ORM module.
//!
//! This module provides access to the database layer, ORM, migrations,
//! hybrid properties, and associations.
//!
//! # Examples
//!
//! ```rust,no_run
//! # #[cfg(feature = "database")]
//! use reinhardt::db::orm::{Model, QuerySet};
//! # #[cfg(feature = "database")]
//! # use reinhardt::db::migrations::Migration;
//! ```

#[cfg(feature = "database")]
pub use reinhardt_db::*;
