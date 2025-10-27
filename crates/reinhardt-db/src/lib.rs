//! # Reinhardt Database
//!
//! Django-style database layer for Reinhardt framework.
//!
//! This crate provides a unified database abstraction that combines:
//! - **Database Backends**: Low-level database operations
//! - **Connection Pooling**: Advanced connection pool management
//! - **ORM**: Django-style ORM for database queries
//! - **Migrations**: Database schema migration system
//! - **Hybrid Types**: Common database type abstractions
//! - **Associations**: Relationship management between models
//!
//! Equivalent to Django's `django.db` package.
//!
//! ## Features
//!
//! ### Database Backends (`backends` module)
//!
//! - **Schema Editor Abstraction**: Unified [`BaseDatabaseSchemaEditor`] trait
//! - **Database-Specific Implementations**: PostgreSQL, MySQL, SQLite support
//! - **DDL Operations**: CREATE TABLE, ALTER TABLE, CREATE INDEX, etc.
//! - **Query Builder**: Type-safe query construction
//!
//! ### Connection Pooling (`pool` module)
//!
//! - **Advanced Pooling**: SQLAlchemy-inspired connection pool management
//! - **Dependency Injection**: Integration with Reinhardt DI system
//! - **Event Listeners**: Connection lifecycle hooks
//! - **Pool Configuration**: Fine-grained control over pool behavior
//!
//! ### ORM (`orm` module)
//!
//! - **Django-style Models**: Define database models with structs
//! - **QuerySet API**: Chainable query builder
//! - **Field Types**: Rich set of field types with validation
//! - **Relationships**: ForeignKey, ManyToMany, OneToOne
//!
//! ### Migrations (`migrations` module)
//!
//! - **Schema Migrations**: Track and apply database schema changes
//! - **Auto-detection**: Automatically detect model changes
//! - **Migration Files**: Generate migration files from model changes
//! - **Rollback Support**: Reverse migrations when needed
//!
//! ## Available Database Backends
//!
//! The backends crate provides multiple database backend implementations:
//! - **PostgreSQL**: Full support with connection pooling
//! - **MySQL**: Full support with connection pooling
//! - **SQLite**: Full support with connection pooling
//! - **MongoDB**: Full support with connection pooling (✅)
//! - **CockroachDB**: Distributed transaction support (✅)
//!
//! ## Planned Features
//!
//! - Additional database backend optimizations
//! - Enhanced schema migration tools
//!
//! ## Quick Start
//!
//! ### Using Schema Editor
//!
//! ```rust
//! use reinhardt_db::backends::schema::factory::{SchemaEditorFactory, DatabaseType};
//!
//! let factory = SchemaEditorFactory::new();
//! let editor = factory.create_for_database(DatabaseType::PostgreSQL);
//!
//! let sql = editor.create_table_sql("users", &[
//!     ("id", "INTEGER PRIMARY KEY"),
//!     ("name", "VARCHAR(100)"),
//! ]);
//! ```
//!
//! ### Using Connection Pool
//!
//! ```rust,no_run
//! use reinhardt_db::pool::{ConnectionPool, PoolConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = ConnectionPool::new("postgres://localhost/mydb", PoolConfig::default()).await?;
//! let conn = pool.acquire().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! - `backends` (default): Database backend abstractions
//! - `pool` (default): Connection pooling support
//! - `orm` (default): ORM functionality
//! - `migrations` (default): Migration system
//! - `hybrid` (default): Hybrid type system
//! - `associations` (default): Association management
//! - `postgres` (default): PostgreSQL support
//! - `sqlite`: SQLite support
//! - `mysql`: MySQL support
//! - `all-databases`: Enable all database backends

// Re-export backends with convenient module structure
#[cfg(feature = "backends")]
pub mod backends {
    //! Database backend abstractions and schema editors
    //!
    //! This module provides low-level database operations, schema editing,
    //! and query building capabilities.

    pub use ::backends::*;
}

// Re-export pool with convenient module structure
#[cfg(feature = "pool")]
pub mod pool {
    //! Connection pooling with advanced lifecycle management
    //!
    //! This module provides SQLAlchemy-inspired connection pooling with
    //! dependency injection support and event-driven lifecycle hooks.

    // Re-export all backends-pool types (includes DI support)
    pub use ::backends_pool::*;

    // Also re-export pool-specific types
    pub use reinhardt_pool::{
        config::{PoolConfig, PoolOptions},
        errors::{PoolError, PoolResult},
        events::{PoolEvent, PoolEventListener},
        manager::PoolManager,
        pool::{ConnectionPool, PooledConnection},
    };
}

// Re-export internal crates
#[cfg(feature = "orm")]
pub use reinhardt_orm as orm;

#[cfg(feature = "migrations")]
pub use reinhardt_migrations as migrations;

#[cfg(feature = "hybrid")]
pub use reinhardt_hybrid as hybrid;

#[cfg(feature = "associations")]
pub use reinhardt_associations as associations;

/// Prelude module for convenient imports
///
/// Imports commonly used types from all modules.
#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    #[cfg(feature = "backends")]
    pub use crate::backends::*;

    #[cfg(feature = "pool")]
    pub use crate::pool::*;

    #[cfg(feature = "orm")]
    pub use crate::orm::*;

    #[cfg(feature = "migrations")]
    pub use crate::migrations::*;

    #[cfg(feature = "hybrid")]
    pub use crate::hybrid::*;

    #[cfg(feature = "associations")]
    pub use crate::associations::*;
}

// Re-export top-level commonly used types
#[cfg(feature = "backends")]
pub use backends::{DatabaseBackend, DatabaseConnection, DatabaseError};

#[cfg(feature = "pool")]
pub use pool::{ConnectionPool, PoolConfig, PoolError};
