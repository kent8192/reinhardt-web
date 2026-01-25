//! Database DDL statement builders
//!
//! This module provides builders for database management operations:
//! - ALTER DATABASE

mod alter_database;

pub use alter_database::AlterDatabaseStatement;
