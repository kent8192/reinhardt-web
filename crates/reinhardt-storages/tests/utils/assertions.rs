//! Custom assertion utilities for storage tests.

use reinhardt_storages::{StorageBackend, StorageError};
use std::fmt;

/// Custom assertion error.
#[derive(Debug)]
pub struct AssertionError {
	pub message: String,
}

impl fmt::Display for AssertionError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl std::error::Error for AssertionError {}

impl AssertionError {
	/// Create a new assertion error.
	pub fn new(message: String) -> Self {
		Self { message }
	}
}

/// Assert that a file exists in storage.
pub async fn assert_storage_exists(
	storage: &dyn StorageBackend,
	name: &str,
) -> Result<(), AssertionError> {
	let exists = storage
		.exists(name)
		.await
		.map_err(|e| AssertionError::new(format!("Failed to check existence: {}", e)))?;

	if !exists {
		return Err(AssertionError::new(format!("File should exist: {}", name)));
	}

	Ok(())
}

/// Assert that a file does NOT exist in storage.
pub async fn assert_storage_not_exists(
	storage: &dyn StorageBackend,
	name: &str,
) -> Result<(), AssertionError> {
	let exists = storage
		.exists(name)
		.await
		.map_err(|e| AssertionError::new(format!("Failed to check existence: {}", e)))?;

	if exists {
		return Err(AssertionError::new(format!(
			"File should not exist: {}",
			name
		)));
	}

	Ok(())
}

/// Assert that file content matches expected value.
pub async fn assert_content_matches(
	storage: &dyn StorageBackend,
	name: &str,
	expected: &[u8],
) -> Result<(), AssertionError> {
	let actual = storage
		.open(name)
		.await
		.map_err(|e| AssertionError::new(format!("Failed to open file: {}", e)))?;

	if actual != expected {
		return Err(AssertionError::new(format!(
			"Content mismatch for file: {} (expected {} bytes, got {} bytes)",
			name,
			expected.len(),
			actual.len()
		)));
	}

	Ok(())
}

/// Assert that presigned URL is valid.
pub fn assert_presigned_url(url: &str) -> Result<(), AssertionError> {
	if !url.starts_with("http://") && !url.starts_with("https://") {
		return Err(AssertionError::new(format!(
			"Presigned URL should start with http:// or https://: {}",
			url
		)));
	}

	// Check for AWS signature query params
	if !url.contains("X-Amz") {
		return Err(AssertionError::new(format!(
			"Presigned URL should contain AWS signature params: {}",
			url
		)));
	}

	Ok(())
}

/// Assert that local file URL is valid.
pub fn assert_file_url(url: &str) -> Result<(), AssertionError> {
	if !url.starts_with("file://") {
		return Err(AssertionError::new(format!(
			"Local URL should start with file://: {}",
			url
		)));
	}

	Ok(())
}

/// Assert that file size matches expected value.
pub async fn assert_file_size(
	storage: &dyn StorageBackend,
	name: &str,
	expected: u64,
) -> Result<(), AssertionError> {
	let actual = storage
		.size(name)
		.await
		.map_err(|e| AssertionError::new(format!("Failed to get file size: {}", e)))?;

	if actual != expected {
		return Err(AssertionError::new(format!(
			"Size mismatch for file: {} (expected {} bytes, got {} bytes)",
			name, expected, actual
		)));
	}

	Ok(())
}

/// Assert that operation returns NotFound error.
pub async fn assert_not_found<F, Fut>(f: F) -> Result<(), AssertionError>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<(), StorageError>>,
{
	match f().await {
		Err(StorageError::NotFound(msg)) => {
			if !msg.is_empty() {
				Ok(())
			} else {
				Err(AssertionError::new(
					"NotFound error should have a message".to_string(),
				))
			}
		}
		Ok(_) => Err(AssertionError::new(
			"Expected NotFound error, but operation succeeded".to_string(),
		)),
		Err(e) => Err(AssertionError::new(format!(
			"Expected NotFound error, got: {:?}",
			e
		))),
	}
}

/// Assert that operation returns PermissionDenied error.
pub async fn assert_permission_denied<F, Fut>(f: F) -> Result<(), AssertionError>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<(), StorageError>>,
{
	match f().await {
		Err(StorageError::PermissionDenied(msg)) => {
			if !msg.is_empty() {
				Ok(())
			} else {
				Err(AssertionError::new(
					"PermissionDenied error should have a message".to_string(),
				))
			}
		}
		Ok(_) => Err(AssertionError::new(
			"Expected PermissionDenied error, but operation succeeded".to_string(),
		)),
		Err(e) => Err(AssertionError::new(format!(
			"Expected PermissionDenied error, got: {:?}",
			e
		))),
	}
}

/// Assert that operation returns ConfigError error.
pub async fn assert_config_error<F, Fut>(f: F) -> Result<(), AssertionError>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<(), StorageError>>,
{
	match f().await {
		Err(StorageError::ConfigError(msg)) => {
			if !msg.is_empty() {
				Ok(())
			} else {
				Err(AssertionError::new(
					"ConfigError should have a message".to_string(),
				))
			}
		}
		Ok(_) => Err(AssertionError::new(
			"Expected ConfigError, but operation succeeded".to_string(),
		)),
		Err(e) => Err(AssertionError::new(format!(
			"Expected ConfigError, got: {:?}",
			e
		))),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_assert_presigned_url_valid() {
		let url = "https://bucket.s3.amazonaws.com/key?X-Amz-Algorithm=AWS4-HMAC-SHA256";
		assert!(assert_presigned_url(url).is_ok());
	}

	#[test]
	fn test_assert_presigned_url_invalid() {
		let url = "ftp://invalid.com";
		assert!(assert_presigned_url(url).is_err());
	}

	#[test]
	fn test_assert_file_url_valid() {
		let url = "file:///path/to/file.txt";
		assert!(assert_file_url(url).is_ok());
	}

	#[test]
	fn test_assert_file_url_invalid() {
		let url = "http://example.com";
		assert!(assert_file_url(url).is_err());
	}

	#[tokio::test]
	async fn test_assert_config_error_message() {
		// Test with a function that returns ConfigError
		async fn returns_config_error() -> Result<(), StorageError> {
			Err(StorageError::ConfigError("test error".to_string()))
		}

		let result = assert_config_error(returns_config_error).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_assert_config_error_empty_message() {
		async fn returns_empty_config_error() -> Result<(), StorageError> {
			Err(StorageError::ConfigError(String::new()))
		}

		let result = assert_config_error(returns_empty_config_error).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_assert_not_found_error() {
		async fn returns_not_found() -> Result<(), StorageError> {
			Err(StorageError::NotFound("test.txt".to_string()))
		}

		let result = assert_not_found(returns_not_found).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_assert_permission_denied_error() {
		async fn returns_permission_denied() -> Result<(), StorageError> {
			Err(StorageError::PermissionDenied("access denied".to_string()))
		}

		let result = assert_permission_denied(returns_permission_denied).await;
		assert!(result.is_ok());
	}
}
