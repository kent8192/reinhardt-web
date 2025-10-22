//! Database query shortcuts with 404 error handling
//!
//! Provides convenient functions for database queries that return 404 errors
//! when objects are not found, similar to Django's get_object_or_404.

use reinhardt_http::Response;

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

/// Get a single object or return a 404 response
///
/// This is a simplified version that works with any query result.
/// In a full implementation, this would integrate with the ORM QuerySet.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::get_or_404_response;
///
// In a view handler:
/// let result = query_database(id);
/// let response = get_or_404_response(result)?;
/// ```
///
/// # Arguments
///
/// * `result` - The result of a database query
///
/// # Returns
///
/// Either the queried object or a 404 Response
pub fn get_or_404_response<T>(result: Result<Option<T>, String>) -> Result<T, Response> {
    match result {
        Ok(Some(obj)) => Ok(obj),
        Ok(None) => Err(Response::not_found()),
        Err(e) => {
            let mut response = Response::internal_server_error();
            response.body = bytes::Bytes::from(format!("Database error: {}", e));
            Err(response)
        }
    }
}

/// Get a list of objects or return a 404 response if empty
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::get_list_or_404_response;
///
/// let results = query_database_list(filters);
/// let list = get_list_or_404_response(results)?;
/// ```
///
/// # Arguments
///
/// * `result` - The result of a database query returning a list
///
/// # Returns
///
/// Either the list of objects or a 404 Response if the list is empty
pub fn get_list_or_404_response<T>(result: Result<Vec<T>, String>) -> Result<Vec<T>, Response> {
    match result {
        Ok(list) if !list.is_empty() => Ok(list),
        Ok(_) => Err(Response::not_found()),
        Err(e) => {
            let mut response = Response::internal_server_error();
            response.body = bytes::Bytes::from(format!("Database error: {}", e));
            Err(response)
        }
    }
}

/// Check if a query result exists, returning a 404 response if not
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::exists_or_404_response;
///
// Simulate a database query
/// let exists = Some(true);
/// let result = exists_or_404_response(exists);
/// assert!(result.is_ok());
///
/// let not_exists: Option<bool> = None;
/// let result = exists_or_404_response(not_exists);
/// assert!(result.is_err());
/// ```
pub fn exists_or_404_response(exists: Option<bool>) -> Result<(), Response> {
    match exists {
        Some(true) => Ok(()),
        _ => Err(Response::not_found()),
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

        let error_response = response.unwrap_err();
        assert_eq!(error_response.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_get_or_404_database_error() {
        let result: Result<Option<User>, String> = Err("Connection failed".to_string());

        let response = get_or_404_response(result);
        assert!(response.is_err());

        let error_response = response.unwrap_err();
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

        let error_response = response.unwrap_err();
        assert_eq!(error_response.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_get_list_or_404_database_error() {
        let result: Result<Vec<User>, String> = Err("Query failed".to_string());

        let response = get_list_or_404_response(result);
        assert!(response.is_err());

        let error_response = response.unwrap_err();
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

        let error_response = result.unwrap_err();
        assert_eq!(error_response.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_exists_or_404_none() {
        let result = exists_or_404_response(None);
        assert!(result.is_err());
    }
}
