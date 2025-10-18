//! Connection pooling with advanced lifecycle management
//!
//! This module provides SQLAlchemy-inspired connection pooling with
//! dependency injection support and event-driven lifecycle hooks.

pub mod config;
pub mod errors;
pub mod events;
pub mod manager;
pub mod pool;

// DI support is only available when both pool and DI features are enabled
#[cfg(feature = "reinhardt-di")]
pub mod di_support;

pub use config::{PoolConfig, PoolOptions};
pub use errors::{PoolError, PoolResult};
pub use events::{PoolEvent, PoolEventListener};
pub use manager::PoolManager;
pub use pool::{ConnectionPool, PooledConnection};

#[cfg(feature = "reinhardt-di")]
pub use di_support::{DatabaseService, DatabaseUrl, MySqlPool, PostgresPool, SqlitePool};
