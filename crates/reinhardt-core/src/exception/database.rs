use thiserror::Error;

/// Driver-independent classification for database failures.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseErrorKind {
	/// A database connection could not be established or was lost.
	Connection,
	/// The injected database handle outlived its owning DI scope.
	ConnectionHandleExpired,
	/// A database operation or connection-pool acquisition timed out.
	Timeout,
	/// A unique constraint was violated.
	UniqueViolation,
	/// A foreign key constraint was violated.
	ForeignKeyViolation,
	/// A non-null constraint was violated.
	NotNullViolation,
	/// A check constraint was violated.
	CheckViolation,
	/// A query contained invalid database syntax.
	Syntax,
	/// A value or expression had an incompatible database type.
	Type,
	/// A referenced database column was not found.
	ColumnNotFound,
	/// A database transaction failed.
	Transaction,
	/// Database configuration was invalid or incomplete.
	Configuration,
	/// Database serialization or deserialization failed.
	Serialization,
	/// The requested database operation is not supported.
	Unsupported,
	/// A database query failed for a reason not covered by a more specific kind.
	Query,
}

/// Structured database failure retained by the framework error boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct DatabaseError {
	kind: DatabaseErrorKind,
	message: String,
	code: Option<String>,
}

impl DatabaseError {
	/// Creates a database error with the specified classification and message.
	pub fn new(kind: DatabaseErrorKind, message: impl Into<String>) -> Self {
		Self {
			kind,
			message: message.into(),
			code: None,
		}
	}

	/// Associates a driver- or database-specific error code with this error.
	pub fn with_code(mut self, code: impl Into<String>) -> Self {
		self.code = Some(code.into());
		self
	}

	/// Returns the driver-independent classification of this error.
	pub fn kind(&self) -> DatabaseErrorKind {
		self.kind
	}

	/// Returns the diagnostic message retained for this error.
	pub fn message(&self) -> &str {
		&self.message
	}

	/// Returns the driver- or database-specific error code, if available.
	pub fn code(&self) -> Option<&str> {
		self.code.as_deref()
	}
}

#[cfg(test)]
mod tests {
	use super::{DatabaseError, DatabaseErrorKind};

	#[test]
	fn connection_handle_expired_has_stable_display_and_server_status() {
		let error = DatabaseError::new(
			DatabaseErrorKind::ConnectionHandleExpired,
			"The injected database connection is no longer available because its DI scope has ended",
		);

		assert_eq!(
			error.to_string(),
			"The injected database connection is no longer available because its DI scope has ended"
		);
		assert_eq!(crate::exception::Error::from(error).status_code(), 500);
	}
}
