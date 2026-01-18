//! Storage error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
	#[error("File not found: {0}")]
	NotFound(String),

	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	#[error("Invalid path: {0}")]
	InvalidPath(String),

	#[error("Storage full")]
	StorageFull,

	#[error("Permission denied: {0}")]
	PermissionDenied(String),

	#[error("File already exists: {0}")]
	AlreadyExists(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
