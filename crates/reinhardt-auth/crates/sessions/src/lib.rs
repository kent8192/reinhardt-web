//! # Reinhardt Sessions
//!
//! Django-inspired session management for Reinhardt.
//!
//! This crate provides session storage and management similar to Django's session framework,
//! supporting multiple backends including cache, database, file, and cookie-based sessions.
//!
//! ## Features
//!
//! - **Cache Backend** (always available): Store sessions in memory or cache systems
//! - **Database Backend** (feature: `database`): Persist sessions in a database
//! - **File Backend** (feature: `file`): Store sessions as files on disk
//! - **Cookie Backend** (feature: `cookie`): Store encrypted sessions in cookies
//! - **JWT Backend** (feature: `jwt`): Store sessions as JSON Web Tokens
//! - **Automatic Cleanup**: Remove expired sessions automatically
//! - **Session Key Rotation**: Rotate session keys for enhanced security
//! - **CSRF Protection**: Integration with reinhardt-forms CSRF tokens
//! - **Multiple Serialization Formats**: JSON, MessagePack, CBOR, and Bincode support
//! - **Session Compression**: Zstd, Gzip, and Brotli compression algorithms
//! - **Session Analytics**: Track and monitor session operations (Logger, Prometheus)
//! - **Session Replication**: High availability with multi-backend replication
//! - **Multi-Tenant Session Isolation**: Tenant-specific session namespaces
//! - **Storage Migration**: Tools for migrating sessions between backends
//!
//! ## Quick Start
//!
//! ```rust
//! use reinhardt_sessions::backends::{InMemorySessionBackend, SessionBackend};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a session backend
//! let backend = InMemorySessionBackend::new();
//!
//! // Store session data
//! let session_data = json!({
//!     "user_id": 42,
//!     "username": "alice",
//!     "authenticated": true,
//! });
//!
//! backend.save("session_key_123", &session_data, Some(3600)).await?;
//!
//! // Retrieve session data
//! let retrieved: Option<serde_json::Value> = backend.load("session_key_123").await?;
//! assert!(retrieved.is_some());
//! # Ok(())
//! # }
//! ```

pub mod analytics;
pub mod backends;
pub mod cleanup;
pub mod compression;
pub mod config;
pub mod csrf;
pub mod middleware;
pub mod migration;
pub mod models;
#[cfg(feature = "replication")]
pub mod replication;
pub mod rotation;
pub mod serialization;
pub mod session;
pub mod tenant;

// Re-export common types
pub use backends::cache::{SessionBackend, SessionError};
pub use backends::{CacheSessionBackend, InMemorySessionBackend};

#[cfg(feature = "database")]
pub use backends::DatabaseSessionBackend;

#[cfg(feature = "file")]
pub use backends::FileSessionBackend;

#[cfg(feature = "jwt")]
pub use backends::{JwtConfig, JwtSessionBackend};

pub use cleanup::{CleanupConfig, CleanupableBackend, SessionCleanupTask, SessionMetadata};
pub use compression::{CompressedSessionBackend, CompressionError, Compressor};
pub use csrf::{CsrfSessionManager, CsrfTokenData};
pub use migration::{MigrationConfig, MigrationResult, Migrator, SessionMigrator};
pub use rotation::{RotationMetadata, RotationPolicy, SessionRotator};
pub use serialization::{JsonSerializer, SerializationFormat, Serializer};

#[cfg(feature = "messagepack")]
pub use serialization::MessagePackSerializer;

#[cfg(feature = "cbor")]
pub use serialization::CborSerializer;

#[cfg(feature = "bincode")]
pub use serialization::BincodeSerializer;

#[cfg(feature = "compression-zstd")]
pub use compression::ZstdCompressor;

#[cfg(feature = "compression-gzip")]
pub use compression::GzipCompressor;

#[cfg(feature = "compression-brotli")]
pub use compression::BrotliCompressor;

pub use analytics::{
	CompositeAnalytics, DeletionReason, InstrumentedSessionBackend, LoggerAnalytics,
	SessionAnalytics, SessionEvent,
};

#[cfg(feature = "analytics-prometheus")]
pub use analytics::PrometheusAnalytics;

#[cfg(feature = "replication")]
pub use replication::{ReplicatedSessionBackend, ReplicationConfig, ReplicationStrategy};
pub use session::Session;
pub use tenant::{TenantConfig, TenantSessionBackend, TenantSessionOperations};

#[cfg(feature = "middleware")]
pub use middleware::{HttpSessionConfig, SameSite, SessionMiddleware};
