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
//! - **Automatic Cleanup**: Remove expired sessions automatically
//! - **Session Key Rotation**: Rotate session keys for enhanced security
//! - **CSRF Protection**: Integration with reinhardt-forms CSRF tokens
//! - **Multiple Serialization Formats**: JSON and MessagePack support
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

pub mod backends;
pub mod cleanup;
pub mod config;
pub mod csrf;
pub mod di_support;
pub mod middleware;
pub mod migration;
pub mod models;
pub mod rotation;
pub mod serialization;
pub mod session;

// Re-export common types
pub use backends::cache::{SessionBackend, SessionError};
pub use backends::{CacheSessionBackend, InMemorySessionBackend};

#[cfg(feature = "database")]
pub use backends::DatabaseSessionBackend;

#[cfg(feature = "file")]
pub use backends::FileSessionBackend;

pub use cleanup::{CleanupConfig, CleanupableBackend, SessionCleanupTask, SessionMetadata};
pub use csrf::{CsrfSessionManager, CsrfTokenData};
pub use migration::{MigrationConfig, MigrationResult, Migrator, SessionMigrator};
pub use rotation::{RotationMetadata, RotationPolicy, SessionRotator};
pub use serialization::{JsonSerializer, SerializationFormat, Serializer};

#[cfg(feature = "messagepack")]
pub use serialization::MessagePackSerializer;

pub use session::Session;

#[cfg(feature = "middleware")]
pub use middleware::{HttpSessionConfig, SameSite, SessionMiddleware};
