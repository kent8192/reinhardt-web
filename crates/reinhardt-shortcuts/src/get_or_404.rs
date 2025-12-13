//! Database query shortcuts with 404 error handling
//!
//! Provides convenient functions for database queries that return 404 errors
//! when objects are not found, similar to Django's get_object_or_404.

use reinhardt_core::http::Response;

/// Error type for get_or_404 operations
#[derive(Debug, thiserror::Error)]
pub enum GetError {
	#[error("Object not found")]
	NotFound,
	#[error("Multiple objects returned")]
	MultipleObjectsReturned,
	#[error("Database error: {0}")]
	DatabaseError(String),
}

impl From<GetError> for Response {
	fn from(error: GetError) -> Self {
		match error {
			GetError::NotFound => Response::not_found(),
			GetError::MultipleObjectsReturned => {
				let mut response = Response::bad_request();
				response.body = bytes::Bytes::from("Multiple objects returned");
				response
			}
			GetError::DatabaseError(msg) => {
				let mut response = Response::internal_server_error();
				response.body = bytes::Bytes::from(format!("Database error: {}", msg));
				response
			}
		}
	}
}

/// Get a single object or return a 404 error
///
/// This is a simplified version that works with any query result.
/// In a full implementation, this would integrate with the ORM QuerySet.
///
/// The error can be automatically converted to a Response using the `?` operator
/// or explicitly via `.into()`.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_shortcuts::get_or_404_response;
///
/// // In a view handler:
/// let result = query_database(id);
/// let object = get_or_404_response(result)?;  // GetError converts to Response automatically
/// ```
///
/// # Arguments
///
/// * `result` - The result of a database query
///
/// # Returns
///
/// Either the queried object or a GetError (NotFound or DatabaseError)
pub fn get_or_404_response<T>(result: Result<Option<T>, String>) -> Result<T, GetError> {
	match result {
		Ok(Some(obj)) => Ok(obj),
		Ok(None) => Err(GetError::NotFound),
		Err(e) => Err(GetError::DatabaseError(e)),
	}
}

/// Get a list of objects or return a 404 error if empty
///
/// The error can be automatically converted to a Response using the `?` operator
/// or explicitly via `.into()`.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_shortcuts::get_list_or_404_response;
///
/// let results = query_database_list(filters);
/// let list = get_list_or_404_response(results)?;  // GetError converts to Response automatically
/// ```
///
/// # Arguments
///
/// * `result` - The result of a database query returning a list
///
/// # Returns
///
/// Either the list of objects or a GetError (NotFound if empty, or DatabaseError)
pub fn get_list_or_404_response<T>(result: Result<Vec<T>, String>) -> Result<Vec<T>, GetError> {
	match result {
		Ok(list) if !list.is_empty() => Ok(list),
		Ok(_) => Err(GetError::NotFound),
		Err(e) => Err(GetError::DatabaseError(e)),
	}
}

/// Check if a query result exists, returning a 404 error if not
///
/// The error can be automatically converted to a Response using the `?` operator
/// or explicitly via `.into()`.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::exists_or_404_response;
///
/// // Simulate a database query
/// let exists = Some(true);
/// let result = exists_or_404_response(exists);
/// assert!(result.is_ok());
///
/// let not_exists: Option<bool> = None;
/// let result = exists_or_404_response(not_exists);
/// assert!(result.is_err());
/// ```
pub fn exists_or_404_response(exists: Option<bool>) -> Result<(), GetError> {
	match exists {
		Some(true) => Ok(()),
		_ => Err(GetError::NotFound),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hyper::StatusCode;

	#[derive(Debug, Clone, PartialEq)]
	struct User {
		id: i64,
		name: String,
	}

	#[test]
	fn test_get_or_404_success() {
		let user = User {
			id: 1,
			name: "Alice".to_string(),
		};
		let result = Ok(Some(user.clone()));

		let response = get_or_404_response(result);
		assert!(response.is_ok());
		match response {
			Ok(returned_user) => assert_eq!(returned_user, user),
			Err(_) => panic!("Expected Ok result"),
		}
	}

	#[test]
	fn test_get_or_404_not_found() {
		let result: Result<Option<User>, String> = Ok(None);

		let response = get_or_404_response(result);
		assert!(response.is_err());

		let error = response.unwrap_err();
		let error_response: Response = error.into();
		assert_eq!(error_response.status, StatusCode::NOT_FOUND);
	}

	#[test]
	fn test_get_or_404_database_error() {
		let result: Result<Option<User>, String> = Err("Connection failed".to_string());

		let response = get_or_404_response(result);
		assert!(response.is_err());

		let error = response.unwrap_err();
		let error_response: Response = error.into();
		assert_eq!(error_response.status, StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[test]
	fn test_get_list_or_404_success() {
		let users = vec![
			User {
				id: 1,
				name: "Alice".to_string(),
			},
			User {
				id: 2,
				name: "Bob".to_string(),
			},
		];
		let result = Ok(users.clone());

		let response = get_list_or_404_response(result);
		assert!(response.is_ok());
		match response {
			Ok(returned_users) => assert_eq!(returned_users, users),
			Err(_) => panic!("Expected Ok result"),
		}
	}

	#[test]
	fn test_get_list_or_404_empty() {
		let result: Result<Vec<User>, String> = Ok(vec![]);

		let response = get_list_or_404_response(result);
		assert!(response.is_err());

		let error = response.unwrap_err();
		let error_response: Response = error.into();
		assert_eq!(error_response.status, StatusCode::NOT_FOUND);
	}

	#[test]
	fn test_get_list_or_404_database_error() {
		let result: Result<Vec<User>, String> = Err("Query failed".to_string());

		let response = get_list_or_404_response(result);
		assert!(response.is_err());

		let error = response.unwrap_err();
		let error_response: Response = error.into();
		assert_eq!(error_response.status, StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[test]
	fn test_exists_or_404_exists() {
		let result = exists_or_404_response(Some(true));
		assert!(result.is_ok());
	}

	#[test]
	fn test_exists_or_404_not_exists() {
		let result = exists_or_404_response(Some(false));
		assert!(result.is_err());

		let error = result.unwrap_err();
		let error_response: Response = error.into();
		assert_eq!(error_response.status, StatusCode::NOT_FOUND);
	}

	#[test]
	fn test_exists_or_404_none() {
		let result = exists_or_404_response(None);
		assert!(result.is_err());

		let error = result.unwrap_err();
		let error_response: Response = error.into();
		assert_eq!(error_response.status, StatusCode::NOT_FOUND);
	}
}
