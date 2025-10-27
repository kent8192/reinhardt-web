//! Session backend implementations
//!
//! This module provides different backend implementations for session storage:
//!
//! - **Cache Backend** (always available): Stores sessions in memory or cache systems
//! - **Database Backend** (feature: `database`): Stores sessions in a database
//! - **File Backend** (feature: `file`): Stores sessions as files on disk
//! - **Cookie Backend** (feature: `cookie`): Stores encrypted sessions in cookies
//! - **JWT Backend** (feature: `jwt`): Stores sessions as JSON Web Tokens
//!
//! ## Feature Flags
//!
//! - `database`: Enable database-backed sessions (requires reinhardt-orm)
//! - `file`: Enable file-backed sessions
//! - `cookie`: Enable cookie-backed sessions
//! - `jwt`: Enable JWT-backed sessions
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_sessions::backends::{InMemorySessionBackend, SessionBackend};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create an in-memory session backend
//! let backend = InMemorySessionBackend::new();
//!
//! // Store session data
//! let session_key = "session_abc123";
//! let session_data = json!({
//!     "user_id": 42,
//!     "username": "alice",
//!     "is_authenticated": true,
//! });
//!
//! backend.save(session_key, &session_data, None).await?;
//!
//! // Retrieve session data
//! let retrieved: Option<serde_json::Value> = backend.load(session_key).await?;
//! assert_eq!(retrieved, Some(session_data));
//!
//! // Delete session
//! backend.delete(session_key).await?;
//!
//! // Verify deletion
//! let deleted: Option<serde_json::Value> = backend.load(session_key).await?;
//! assert_eq!(deleted, None);
//! # Ok(())
//! # }
//! ```

pub mod cache;

#[cfg(feature = "database")]
pub mod database;

#[cfg(feature = "file")]
pub mod file;

#[cfg(feature = "cookie")]
pub mod cookie;

#[cfg(feature = "jwt")]
pub mod jwt;

// Re-export commonly used backends
pub use cache::{CacheSessionBackend, InMemorySessionBackend, SessionBackend, SessionError};

#[cfg(feature = "database")]
pub use database::DatabaseSessionBackend;

#[cfg(feature = "file")]
pub use file::FileSessionBackend;

#[cfg(feature = "cookie")]
pub use cookie::CookieSessionBackend;

#[cfg(feature = "jwt")]
pub use jwt::{JwtConfig, JwtSessionBackend};
