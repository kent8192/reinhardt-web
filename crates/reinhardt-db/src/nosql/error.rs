//! NoSQL database error types
//!
//! This module provides a unified error type for all NoSQL database operations.

use std::fmt;

/// Result type for NoSQL operations
pub type Result<T> = std::result::Result<T, NoSQLError>;

/// Unified error type for NoSQL operations
#[derive(Debug)]
pub enum NoSQLError {
	/// Connection error
	ConnectionError(String),

	/// Query/operation execution error
	ExecutionError(String),

	/// Document/data not found
	NotFound(String),

	/// Serialization/deserialization error
	SerializationError(String),

	/// Invalid operation for the current backend
	InvalidOperation(String),

	/// Configuration error
	ConfigError(String),

	/// Timeout error
	Timeout(String),

	/// Authentication error
	AuthenticationError(String),

	/// Permission denied error
	PermissionDenied(String),

	/// Database-specific error (contains the original error message)
	DatabaseError(String),

	/// Feature not supported by this backend
	UnsupportedFeature(String),
}

impl fmt::Display for NoSQLError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			NoSQLError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
			NoSQLError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
			NoSQLError::NotFound(msg) => write!(f, "Not found: {}", msg),
			NoSQLError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
			NoSQLError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
			NoSQLError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
			NoSQLError::Timeout(msg) => write!(f, "Timeout: {}", msg),
			NoSQLError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
			NoSQLError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
			NoSQLError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
			NoSQLError::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
		}
	}
}

impl std::error::Error for NoSQLError {}

// Convenience conversion implementations for common error types
impl From<serde_json::Error> for NoSQLError {
	fn from(err: serde_json::Error) -> Self {
		NoSQLError::SerializationError(err.to_string())
	}
}

#[cfg(feature = "mongodb")]
impl From<mongodb::error::Error> for NoSQLError {
	fn from(err: mongodb::error::Error) -> Self {
		use mongodb::error::ErrorKind;

		match *err.kind {
			ErrorKind::Authentication { .. } => NoSQLError::AuthenticationError(err.to_string()),
			ErrorKind::InvalidArgument { .. } => NoSQLError::InvalidOperation(err.to_string()),
			ErrorKind::Io(_) => NoSQLError::ConnectionError(err.to_string()),
			_ => NoSQLError::DatabaseError(err.to_string()),
		}
	}
}

// In bson v3.x, both ser::Error and de::Error are type aliases for bson::error::Error
#[cfg(feature = "mongodb")]
impl From<bson::error::Error> for NoSQLError {
	fn from(err: bson::error::Error) -> Self {
		NoSQLError::SerializationError(err.to_string())
	}
}

#[cfg(feature = "redis")]
impl From<redis::RedisError> for NoSQLError {
	fn from(err: redis::RedisError) -> Self {
		if err.is_timeout() {
			NoSQLError::Timeout(err.to_string())
		} else if err.is_connection_refusal() || err.is_connection_dropped() {
			NoSQLError::ConnectionError(err.to_string())
		} else {
			NoSQLError::DatabaseError(err.to_string())
		}
	}
}

// ============================================================================
// ODM-specific error types
// ============================================================================

/// Result type for ODM operations.
pub type OdmResult<T> = std::result::Result<T, OdmError>;

/// Error type for ODM operations.
#[derive(Debug)]
pub enum OdmError {
	/// Validation failed.
	Validation(ValidationError),

	/// MongoDB error.
	#[cfg(feature = "mongodb")]
	Mongo(mongodb::error::Error),

	/// Document not found.
	// TODO: [PR#31] Currently unused — Repository will use this for find/update/delete miss
	NotFound,

	/// Duplicate key error.
	// TODO: [PR#31] Currently unused — add MongoDB error code 11000 detection helper
	DuplicateKey { field: String },

	/// Serialization error.
	// TODO: [PR#31] Currently unused — Repository serialization will use this
	Serialization(String),

	/// Deserialization error.
	#[cfg(feature = "mongodb")]
	Deserialization(bson::error::Error),
}

impl fmt::Display for OdmError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			OdmError::Validation(err) => write!(f, "Validation failed: {}", err),
			#[cfg(feature = "mongodb")]
			OdmError::Mongo(err) => write!(f, "MongoDB error: {}", err),
			OdmError::NotFound => write!(f, "Document not found"),
			OdmError::DuplicateKey { field } => write!(f, "Duplicate key: {}", field),
			OdmError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
			#[cfg(feature = "mongodb")]
			OdmError::Deserialization(err) => write!(f, "Deserialization error: {}", err),
		}
	}
}

impl std::error::Error for OdmError {}

impl From<ValidationError> for OdmError {
	fn from(err: ValidationError) -> Self {
		OdmError::Validation(err)
	}
}

#[cfg(feature = "mongodb")]
impl From<mongodb::error::Error> for OdmError {
	fn from(err: mongodb::error::Error) -> Self {
		OdmError::Mongo(err)
	}
}

#[cfg(feature = "mongodb")]
impl From<bson::error::Error> for OdmError {
	fn from(err: bson::error::Error) -> Self {
		OdmError::Deserialization(err)
	}
}

/// Validation error type.
// TODO: [PR#31] All variants currently unused — macro-generated validate() will use them
#[derive(Debug)]
pub enum ValidationError {
	/// Required field is missing or empty.
	Required(&'static str),

	/// Invalid email format.
	InvalidEmail,

	/// Invalid URL format.
	InvalidUrl,

	/// Value out of range.
	OutOfRange {
		field: &'static str,
		min: i64,
		max: i64,
	},

	/// Custom validation error.
	Custom(String),
}

impl fmt::Display for ValidationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ValidationError::Required(field) => write!(f, "Field required: {}", field),
			ValidationError::InvalidEmail => write!(f, "Invalid email format"),
			ValidationError::InvalidUrl => write!(f, "Invalid URL format"),
			ValidationError::OutOfRange { field, min, max } => {
				write!(
					f,
					"Value out of range: {} must be between {} and {}",
					field, min, max
				)
			}
			ValidationError::Custom(msg) => write!(f, "Custom validation failed: {}", msg),
		}
	}
}

impl std::error::Error for ValidationError {}
