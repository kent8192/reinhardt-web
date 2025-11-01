//! File storage system for Reinhardt
//!
//! This crate provides Django-style file storage with support for multiple
//! storage backends (local, S3-compatible, etc.).

pub mod backend;
pub mod errors;
pub mod file;
pub mod local;
pub mod memory;

pub use backend::Storage;
pub use errors::{StorageError, StorageResult};
pub use file::{FileMetadata, StoredFile};
pub use local::LocalStorage;
pub use memory::InMemoryStorage;

/// Re-export commonly used types
pub mod prelude {
	pub use crate::backend::*;
	pub use crate::errors::*;
	pub use crate::file::*;
	pub use crate::local::*;
	pub use crate::memory::*;
}
