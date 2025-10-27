//! Response builder functions for common HTTP responses.
//!
//! This module provides convenient functions for building HTTP responses with
//! appropriate status codes and JSON bodies. All functions are designed to be
//! ergonomic and type-safe, leveraging Rust's type system to ensure correctness.

use crate::{Error, Response, Result};
use serde::Serialize;

/// Creates a 200 OK response with JSON body.
///
/// # Arguments
///
/// * `data` - The data to serialize as JSON in the response body
///
/// # Returns
///
/// A `Result<Response>` containing the 200 OK response with JSON body
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::ok_json;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let user = User {
///     id: 1,
///     name: "Alice".to_string(),
/// };
///
/// let response = ok_json(user).unwrap();
/// assert_eq!(response.status(), 200);
/// ```
pub fn ok_json<T: Serialize>(data: T) -> Result<Response> {
    let json = serde_json::to_string(&data)
        .map_err(|e| Error::Serialization(format!("Failed to serialize data: {}", e)))?;

    Ok(Response::ok()
        .with_header("Content-Type", "application/json")
        .with_body(json))
}

/// Creates a 201 Created response with JSON body.
///
/// # Arguments
///
/// * `data` - The data to serialize as JSON in the response body
///
/// # Returns
///
/// A `Result<Response>` containing the 201 Created response with JSON body
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::created_json;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let user = User {
///     id: 1,
///     name: "Bob".to_string(),
/// };
///
/// let response = created_json(user).unwrap();
/// assert_eq!(response.status(), 201);
/// ```
pub fn created_json<T: Serialize>(data: T) -> Result<Response> {
    let json = serde_json::to_string(&data)
        .map_err(|e| Error::Serialization(format!("Failed to serialize data: {}", e)))?;

    Ok(Response::created()
        .with_header("Content-Type", "application/json")
        .with_body(json))
}

/// Creates a 204 No Content response.
///
/// # Returns
///
/// A `Response` with status code 204 and no body
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::no_content;
///
/// let response = no_content();
/// assert_eq!(response.status(), 204);
/// ```
pub fn no_content() -> Response {
    Response::no_content()
}

/// Creates a 400 Bad Request response with an error message.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 400 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::bad_request;
///
/// let response = bad_request("Invalid email format");
/// assert_eq!(response.status(), 400);
/// ```
pub fn bad_request(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::bad_request()
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

/// Creates a 401 Unauthorized response with an error message.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 401 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::unauthorized;
///
/// let response = unauthorized("Missing authentication token");
/// assert_eq!(response.status(), 401);
/// ```
pub fn unauthorized(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::unauthorized()
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

/// Creates a 404 Not Found response with an error message.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 404 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::not_found;
///
/// let response = not_found("User not found");
/// assert_eq!(response.status(), 404);
/// ```
pub fn not_found(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::not_found()
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

/// Creates a 500 Internal Server Error response with an error message.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 500 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::internal_error;
///
/// let response = internal_error("Database connection failed");
/// assert_eq!(response.status(), 500);
/// ```
pub fn internal_error(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::internal_server_error()
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

/// Creates a 202 Accepted response.
///
/// Indicates that the request has been accepted for processing, but processing
/// has not been completed. Typically used for asynchronous operations.
///
/// # Returns
///
/// A `Response` with status code 202 and no body
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::accepted;
///
/// let response = accepted();
/// assert_eq!(response.status(), 202);
/// ```
pub fn accepted() -> Response {
    Response::new(hyper::StatusCode::ACCEPTED)
}

/// Creates a 403 Forbidden response with an error message.
///
/// Indicates that the server understood the request but refuses to authorize it.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 403 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::forbidden;
///
/// let response = forbidden("Access denied to this resource");
/// assert_eq!(response.status(), 403);
/// ```
pub fn forbidden(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::forbidden()
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

/// Creates a 409 Conflict response with an error message.
///
/// Indicates that the request could not be completed due to a conflict
/// with the current state of the target resource.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 409 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::conflict;
///
/// let response = conflict("Email already exists");
/// assert_eq!(response.status(), 409);
/// ```
pub fn conflict(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::new(hyper::StatusCode::CONFLICT)
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

/// Creates a 422 Unprocessable Entity response with an error message.
///
/// Indicates that the server understands the content type of the request entity,
/// and the syntax is correct, but it was unable to process the contained instructions.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 422 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::unprocessable_entity;
///
/// let response = unprocessable_entity("Validation failed: age must be positive");
/// assert_eq!(response.status(), 422);
/// ```
pub fn unprocessable_entity(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::new(hyper::StatusCode::UNPROCESSABLE_ENTITY)
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

/// Creates a 503 Service Unavailable response with an error message.
///
/// Indicates that the server is not ready to handle the request, usually
/// because it is temporarily overloaded or down for maintenance.
///
/// # Arguments
///
/// * `message` - The error message to include in the response body
///
/// # Returns
///
/// A `Response` with status code 503 and the error message as JSON
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::service_unavailable;
///
/// let response = service_unavailable("Service is under maintenance");
/// assert_eq!(response.status(), 503);
/// ```
pub fn service_unavailable(message: impl Into<String>) -> Response {
    let error_body = format!(r#"{{"error":"{}"}}"#, message.into());
    Response::new(hyper::StatusCode::SERVICE_UNAVAILABLE)
        .with_header("Content-Type", "application/json")
        .with_body(error_body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestData {
        id: i64,
        name: String,
    }

    #[test]
    fn test_ok_json() {
        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };
        let response = ok_json(data).unwrap();
        assert_eq!(response.status(), 200);
        assert!(response.body().contains(r#""id":1"#));
        assert!(response.body().contains(r#""name":"test""#));
    }

    #[test]
    fn test_created_json() {
        let data = TestData {
            id: 2,
            name: "created".to_string(),
        };
        let response = created_json(data).unwrap();
        assert_eq!(response.status(), 201);
        assert!(response.body().contains(r#""id":2"#));
        assert!(response.body().contains(r#""name":"created""#));
    }

    #[test]
    fn test_no_content() {
        let response = no_content();
        assert_eq!(response.status(), 204);
        assert!(response.body().is_empty());
    }

    #[test]
    fn test_bad_request() {
        let response = bad_request("Invalid input");
        assert_eq!(response.status(), 400);
        assert!(response.body().contains("Invalid input"));
    }

    #[test]
    fn test_unauthorized() {
        let response = unauthorized("Token expired");
        assert_eq!(response.status(), 401);
        assert!(response.body().contains("Token expired"));
    }

    #[test]
    fn test_not_found() {
        let response = not_found("Resource not found");
        assert_eq!(response.status(), 404);
        assert!(response.body().contains("Resource not found"));
    }

    #[test]
    fn test_internal_error() {
        let response = internal_error("Database error");
        assert_eq!(response.status(), 500);
        assert!(response.body().contains("Database error"));
    }

    #[test]
    fn test_error_message_into_string() {
        let response = bad_request("test");
        assert!(response.body().contains("test"));

        let response = unauthorized(String::from("auth failed"));
        assert!(response.body().contains("auth failed"));
    }

    #[test]
    fn test_accepted() {
        let response = accepted();
        assert_eq!(response.status(), 202);
        assert!(response.body().is_empty());
    }

    #[test]
    fn test_forbidden() {
        let response = forbidden("Access denied");
        assert_eq!(response.status(), 403);
        assert!(response.body().contains("Access denied"));
    }

    #[test]
    fn test_conflict() {
        let response = conflict("Resource already exists");
        assert_eq!(response.status(), 409);
        assert!(response.body().contains("Resource already exists"));
    }

    #[test]
    fn test_unprocessable_entity() {
        let response = unprocessable_entity("Validation failed");
        assert_eq!(response.status(), 422);
        assert!(response.body().contains("Validation failed"));
    }

    #[test]
    fn test_service_unavailable() {
        let response = service_unavailable("Under maintenance");
        assert_eq!(response.status(), 503);
        assert!(response.body().contains("Under maintenance"));
    }
}
