//! Connection pooling with advanced lifecycle management
//!
//! This module provides SQLAlchemy-inspired connection pooling with
//! dependency injection support and event-driven lifecycle hooks.

pub mod config;
pub mod di_support;
pub mod errors;
pub mod events;
pub mod manager;
pub mod pool;

pub use config::{PoolConfig, PoolOptions};
pub use di_support::{DatabaseService, DatabaseUrl, MySqlPool, PostgresPool, SqlitePool};
pub use errors::{PoolError, PoolResult};
pub use events::{PoolEvent, PoolEventListener};
pub use manager::PoolManager;
pub use pool::{ConnectionPool, PooledConnection};
