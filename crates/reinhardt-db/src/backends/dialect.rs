//! Database dialect implementations
//!
//! This module provides concrete implementations of the `DatabaseBackend` trait
//! for different database systems.

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "mysql")]
pub mod mysql;

#[cfg(feature = "mysql")]
pub mod mysql_dcl;

#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteBackend;

#[cfg(feature = "mysql")]
pub use mysql::MySqlBackend;

#[cfg(feature = "mysql")]
pub use mysql_dcl::MySqlUser;
