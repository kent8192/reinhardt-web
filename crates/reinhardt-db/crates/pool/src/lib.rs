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
pub mod pool;

pub use config::{PoolConfig, PoolOptions};
pub use errors::{PoolError, PoolResult};
pub use events::{PoolEvent, PoolEventListener};
pub use manager::PoolManager;
pub use pool::{ConnectionPool, PooledConnection};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::config::*;
    pub use crate::errors::*;
    pub use crate::events::*;
    pub use crate::manager::*;
    pub use crate::pool::*;
}
