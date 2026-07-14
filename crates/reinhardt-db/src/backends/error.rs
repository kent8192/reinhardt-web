//! Error types for database operations

pub use reinhardt_core::exception::{DatabaseError, DatabaseErrorKind};

/// Result type for database operations
pub type Result<T> = reinhardt_core::exception::Result<T>;

#[cfg(any(
	feature = "orm",
	feature = "postgres",
	feature = "sqlite",
	feature = "mysql",
	test
))]
pub(crate) fn map_sqlx_error(error: sqlx::Error) -> DatabaseError {
	use sqlx::error::ErrorKind;

	match error {
		sqlx::Error::Database(error) => {
			let kind = match error.kind() {
				ErrorKind::UniqueViolation => DatabaseErrorKind::UniqueViolation,
				ErrorKind::ForeignKeyViolation => DatabaseErrorKind::ForeignKeyViolation,
				ErrorKind::NotNullViolation => DatabaseErrorKind::NotNullViolation,
				ErrorKind::CheckViolation => DatabaseErrorKind::CheckViolation,
				ErrorKind::Other => DatabaseErrorKind::Query,
				_ => DatabaseErrorKind::Query,
			};
			let database_error = DatabaseError::new(kind, error.message());
			match error.code() {
				Some(code) => database_error.with_code(code.into_owned()),
				None => database_error,
			}
		}
		sqlx::Error::PoolTimedOut => {
			DatabaseError::new(DatabaseErrorKind::Timeout, "Pool timed out")
		}
		sqlx::Error::Io(error) => {
			DatabaseError::new(DatabaseErrorKind::Connection, error.to_string())
		}
		sqlx::Error::Tls(error) => {
			DatabaseError::new(DatabaseErrorKind::Connection, error.to_string())
		}
		sqlx::Error::Protocol(message) => {
			DatabaseError::new(DatabaseErrorKind::Connection, message)
		}
		sqlx::Error::PoolClosed => DatabaseError::new(DatabaseErrorKind::Connection, "Pool closed"),
		sqlx::Error::WorkerCrashed => {
			DatabaseError::new(DatabaseErrorKind::Connection, "Worker crashed")
		}
		sqlx::Error::Configuration(error) => {
			DatabaseError::new(DatabaseErrorKind::Configuration, error.to_string())
		}
		sqlx::Error::TypeNotFound { type_name } => DatabaseError::new(
			DatabaseErrorKind::Type,
			format!("Type not found: {type_name}"),
		),
		sqlx::Error::ColumnIndexOutOfBounds { index, len } => DatabaseError::new(
			DatabaseErrorKind::ColumnNotFound,
			format!("Column index {index} out of bounds (len: {len})"),
		),
		sqlx::Error::ColumnNotFound(name) => DatabaseError::new(
			DatabaseErrorKind::ColumnNotFound,
			format!("Column not found: {name}"),
		),
		sqlx::Error::ColumnDecode { index, source } => DatabaseError::new(
			DatabaseErrorKind::Type,
			format!("Failed to decode column {index}: {source}"),
		),
		sqlx::Error::Decode(error) => {
			DatabaseError::new(DatabaseErrorKind::Type, error.to_string())
		}
		sqlx::Error::RowNotFound => DatabaseError::new(DatabaseErrorKind::Query, "Row not found"),
		error @ sqlx::Error::InvalidSavePointStatement => {
			DatabaseError::new(DatabaseErrorKind::Transaction, error.to_string())
		}
		error @ sqlx::Error::BeginFailed => {
			DatabaseError::new(DatabaseErrorKind::Transaction, error.to_string())
		}
		sqlx::Error::Migrate(error) => DatabaseError::new(
			DatabaseErrorKind::Query,
			format!("Migration error: {error}"),
		),
		error => DatabaseError::new(DatabaseErrorKind::Query, error.to_string()),
	}
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;
	use std::fmt;
	use std::io;

	use rstest::rstest;
	use sqlx::error::{DatabaseError as SqlxDatabaseError, ErrorKind};

	use super::{DatabaseError, DatabaseErrorKind, map_sqlx_error};

	const DATABASE_MESSAGE: &str = "database operation failed";
	const DATABASE_CODE: &str = "VENDOR-CODE";
	const CONSTRAINT_NAME: &str = "private_constraint";
	const TABLE_NAME: &str = "private_table";

	#[derive(Debug)]
	struct TestSqlxDatabaseError {
		kind: fn() -> ErrorKind,
	}

	fn unique_violation() -> ErrorKind {
		ErrorKind::UniqueViolation
	}

	fn foreign_key_violation() -> ErrorKind {
		ErrorKind::ForeignKeyViolation
	}

	fn not_null_violation() -> ErrorKind {
		ErrorKind::NotNullViolation
	}

	fn check_violation() -> ErrorKind {
		ErrorKind::CheckViolation
	}

	fn other_database_error() -> ErrorKind {
		ErrorKind::Other
	}

	impl fmt::Display for TestSqlxDatabaseError {
		fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			formatter.write_str(DATABASE_MESSAGE)
		}
	}

	impl std::error::Error for TestSqlxDatabaseError {}

	impl SqlxDatabaseError for TestSqlxDatabaseError {
		fn message(&self) -> &str {
			DATABASE_MESSAGE
		}

		fn code(&self) -> Option<Cow<'_, str>> {
			Some(Cow::Borrowed(DATABASE_CODE))
		}

		fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
			self
		}

		fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
			self
		}

		fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
			self
		}

		fn constraint(&self) -> Option<&str> {
			Some(CONSTRAINT_NAME)
		}

		fn table(&self) -> Option<&str> {
			Some(TABLE_NAME)
		}

		fn kind(&self) -> ErrorKind {
			(self.kind)()
		}
	}

	#[rstest]
	#[case(unique_violation, DatabaseErrorKind::UniqueViolation)]
	#[case(foreign_key_violation, DatabaseErrorKind::ForeignKeyViolation)]
	#[case(not_null_violation, DatabaseErrorKind::NotNullViolation)]
	#[case(check_violation, DatabaseErrorKind::CheckViolation)]
	#[case(other_database_error, DatabaseErrorKind::Query)]
	fn map_sqlx_error_classifies_database_errors(
		#[case] sqlx_kind: fn() -> ErrorKind,
		#[case] expected_kind: DatabaseErrorKind,
	) {
		// Arrange
		let error = sqlx::Error::Database(Box::new(TestSqlxDatabaseError { kind: sqlx_kind }));

		// Act
		let error = map_sqlx_error(error);

		// Assert
		assert_eq!(
			error,
			DatabaseError::new(expected_kind, DATABASE_MESSAGE).with_code(DATABASE_CODE)
		);
		assert_eq!(error.message(), DATABASE_MESSAGE);
		assert_eq!(error.code(), Some(DATABASE_CODE));
		assert_eq!(error.to_string(), DATABASE_MESSAGE);
	}

	#[rstest]
	#[case(sqlx::Error::PoolTimedOut, DatabaseErrorKind::Timeout)]
	#[case(sqlx::Error::PoolClosed, DatabaseErrorKind::Connection)]
	#[case(sqlx::Error::WorkerCrashed, DatabaseErrorKind::Connection)]
	#[case(
		sqlx::Error::Protocol("wire failure".to_string()),
		DatabaseErrorKind::Connection
	)]
	#[case(
		sqlx::Error::Configuration(Box::new(io::Error::other("invalid configuration"))),
		DatabaseErrorKind::Configuration
	)]
	#[case(
		sqlx::Error::TypeNotFound {
			type_name: "missing_type".to_string(),
		},
		DatabaseErrorKind::Type
	)]
	#[case(
		sqlx::Error::ColumnIndexOutOfBounds { index: 2, len: 1 },
		DatabaseErrorKind::ColumnNotFound
	)]
	#[case(
		sqlx::Error::ColumnNotFound("missing_column".to_string()),
		DatabaseErrorKind::ColumnNotFound
	)]
	#[case(
		sqlx::Error::Decode(Box::new(io::Error::other("decode failed"))),
		DatabaseErrorKind::Type
	)]
	#[case(sqlx::Error::RowNotFound, DatabaseErrorKind::Query)]
	#[case(sqlx::Error::InvalidSavePointStatement, DatabaseErrorKind::Transaction)]
	#[case(sqlx::Error::BeginFailed, DatabaseErrorKind::Transaction)]
	fn map_sqlx_error_classifies_non_database_errors(
		#[case] sqlx_error: sqlx::Error,
		#[case] expected_kind: DatabaseErrorKind,
	) {
		// Arrange

		// Act
		let error = map_sqlx_error(sqlx_error);

		// Assert
		assert_eq!(error.kind(), expected_kind);
		assert_eq!(error.code(), None);
	}
}
