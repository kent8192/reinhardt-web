//! Database backend optimization utilities
//!
//! This module provides optimization features for database backends:
//! - Connection pooling configuration
//! - Query caching
//! - Batch operations

pub mod connection_pool;
pub mod query_cache;
pub mod batch_ops;

pub use connection_pool::{PoolOptimizationConfig, OptimizedPoolBuilder};
pub use query_cache::{QueryCache, QueryCacheConfig, CachedQuery};
pub use batch_ops::{BatchOperations, BatchInsertBuilder, BatchUpdateBuilder};
