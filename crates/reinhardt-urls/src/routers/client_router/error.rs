//! Error types for client-side routing.

/// Error type for path parameter extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
	/// Failed to parse a parameter value.
	ParseError {
		/// Index of the parameter that failed to parse.
		param_index: Option<usize>,
		/// Expected type name.
		param_type: &'static str,
		/// Raw string value that failed to parse.
		raw_value: String,
		/// Error message from parsing.
		source: String,
	},
	/// Parameter count mismatch.
	CountMismatch {
		/// Expected number of parameters.
		expected: usize,
		/// Actual number of parameters.
		actual: usize,
	},
	/// Custom error message.
	Custom(String),
}

impl std::fmt::Display for PathError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ParseError {
				param_index,
				param_type,
				raw_value,
				source,
			} => {
				if let Some(idx) = param_index {
					write!(
						f,
						"Failed to parse parameter[{}] '{}' as {}: {}",
						idx, raw_value, param_type, source
					)
				} else {
					write!(
						f,
						"Failed to parse parameter '{}' as {}: {}",
						raw_value, param_type, source
					)
				}
			}
			Self::CountMismatch { expected, actual } => {
				write!(
					f,
					"Parameter count mismatch: expected {}, got {}",
					expected, actual
				)
			}
			Self::Custom(msg) => write!(f, "{}", msg),
		}
	}
}

impl std::error::Error for PathError {}

/// Error type for router operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
	/// Route not found.
	NotFound(String),
	/// Invalid route name.
	InvalidRouteName(String),
	/// Missing parameter for reverse URL.
	MissingParameter(String),
	/// Navigation failed.
	NavigationFailed(String),
	/// Path parameter extraction failed.
	PathExtraction(PathError),
}

impl std::fmt::Display for RouterError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NotFound(path) => write!(f, "Route not found: {}", path),
			Self::InvalidRouteName(name) => write!(f, "Invalid route name: {}", name),
			Self::MissingParameter(param) => write!(f, "Missing parameter: {}", param),
			Self::NavigationFailed(msg) => write!(f, "Navigation failed: {}", msg),
			Self::PathExtraction(err) => write!(f, "Path extraction error: {}", err),
		}
	}
}

impl std::error::Error for RouterError {}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_path_error_display() {
		let err = PathError::ParseError {
			param_index: Some(0),
			param_type: "i32",
			raw_value: "abc".to_string(),
			source: "invalid digit".to_string(),
		};
		assert!(err.to_string().contains("parameter[0]"));
		assert!(err.to_string().contains("abc"));
		assert!(err.to_string().contains("i32"));
	}

	#[rstest]
	fn test_path_error_count_mismatch() {
		let err = PathError::CountMismatch {
			expected: 2,
			actual: 1,
		};
		assert!(err.to_string().contains("expected 2"));
		assert!(err.to_string().contains("got 1"));
	}

	#[rstest]
	fn test_router_error_display() {
		assert_eq!(
			RouterError::NotFound("/test/".to_string()).to_string(),
			"Route not found: /test/"
		);
		assert_eq!(
			RouterError::InvalidRouteName("test".to_string()).to_string(),
			"Invalid route name: test"
		);
	}
}
