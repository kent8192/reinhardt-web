//! Error type used by the model-based ViewSet handler.

/// Error type for `ModelViewSetHandler`.
#[derive(Debug)]
pub enum ViewError {
	/// Serialization or deserialization failure.
	Serialization(String),
	/// Permission denied for the requested action.
	Permission(String),
	/// The requested resource was not found.
	NotFound(String),
	/// The request was malformed or invalid.
	BadRequest(String),
	/// An internal server error occurred.
	Internal(String),
	/// A database operation failed.
	DatabaseError(String),
}

impl std::fmt::Display for ViewError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ViewError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
			ViewError::Permission(msg) => write!(f, "Permission denied: {}", msg),
			ViewError::NotFound(msg) => write!(f, "Not found: {}", msg),
			ViewError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
			ViewError::Internal(msg) => write!(f, "Internal error: {}", msg),
			ViewError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
		}
	}
}

impl std::error::Error for ViewError {}

/// Convert `ViewError` into the framework-wide `reinhardt_core::exception::Error`.
///
/// Mapping preserves HTTP status codes via `Error::status_code()`:
///
/// | `ViewError`        | `Error`         | Status |
/// |--------------------|-----------------|--------|
/// | `Serialization`    | `Serialization` | 400    |
/// | `Permission`       | `Authorization` | 403    |
/// | `NotFound`         | `NotFound`      | 404    |
/// | `BadRequest`       | `Http`          | 400    |
/// | `Internal`         | `Internal`      | 500    |
/// | `DatabaseError`    | `Database`      | 500    |
impl From<ViewError> for reinhardt_core::exception::Error {
	fn from(value: ViewError) -> Self {
		match value {
			ViewError::Serialization(m) => Self::Serialization(m),
			ViewError::Permission(m) => Self::Authorization(m),
			ViewError::NotFound(m) => Self::NotFound(m),
			ViewError::BadRequest(m) => Self::Http(m),
			ViewError::Internal(m) => Self::Internal(m),
			ViewError::DatabaseError(m) => Self::Database(m),
		}
	}
}
