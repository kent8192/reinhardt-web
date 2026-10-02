//! Errors produced by the Manouche v2 migration codemod.

use std::path::PathBuf;

use thiserror::Error;

/// An error produced while validating or applying the migration codemod.
#[derive(Debug, Error)]
pub enum MigrateV2Error {
	/// One or more requested skip rules are not supported.
	#[error("unknown --skip rule(s): {0}")]
	UnknownSkipRules(String),
	/// The source tree could not be walked.
	#[error("failed to walk source tree: {0}")]
	Walk(#[from] walkdir::Error),
	/// A source file could not be read or written.
	#[error("I/O operation failed: {0}")]
	Io(#[from] std::io::Error),
	/// A file path does not have a parent directory for an atomic rewrite.
	#[error("path has no parent directory: {}", .0.display())]
	MissingParent(PathBuf),
}

/// Result type for the Manouche v2 migration codemod.
pub type Result<T> = std::result::Result<T, MigrateV2Error>;
