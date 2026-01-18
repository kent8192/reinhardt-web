//! Database connection pooling for Reinhardt
//!
//! This crate provides advanced connection pooling features similar to SQLAlchemy,
//! with support for multiple database backends and connection lifecycle management.
//!
//! Note: DI support (DatabaseService, DatabaseUrl) is provided by the backends-pool crate.

pub mod config;
pub mod errors;
pub mod events;
pub mod manager;
// Allow module_inception: Re-exporting pool submodule from pool.rs
// is intentional for compatibility with existing imports (`reinhardt_db::pool::DatabasePool`)
#[allow(clippy::module_inception)]
pub mod pool;

pub use config::{PoolConfig, PoolOptions};
pub use errors::{PoolError, PoolResult};
pub use events::{PoolEvent, PoolEventListener};
pub use manager::PoolManager;
pub use pool::{ConnectionPool, PooledConnection};

/// Re-export commonly used types
pub mod prelude {
	pub use super::config::*;
	pub use super::errors::*;
	pub use super::events::*;
	pub use super::manager::*;
}
