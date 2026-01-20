//! Custom validation hooks for parsers.
//!
//! This module provides traits and utilities for adding custom validation logic
//! before and after parsing operations.

use async_trait::async_trait;
use bytes::Bytes;

use super::parser::{ParseResult, ParsedData};

/// Trait for custom parser validation hooks.
///
/// Implement this trait to add custom validation logic that runs before or after
/// parsing operations. This allows you to enforce business rules, size limits,
/// or other constraints on parsed data.
///
/// # Examples
///
/// ```
/// use async_trait::async_trait;
/// use bytes::Bytes;
/// use reinhardt_core::parsers::validator::ParserValidator;
/// use reinhardt_core::parsers::parser::{ParseResult, ParsedData};
/// use reinhardt_core::exception::Error;
///
/// struct SizeLimitValidator {
///     max_size: usize,
/// }
///
/// #[async_trait]
/// impl ParserValidator for SizeLimitValidator {
///     async fn before_parse(&self, _content_type: Option<&str>, body: &Bytes) -> ParseResult<()> {
///         if body.len() > self.max_size {
///             return Err(Error::Validation(format!(
///                 "Body size {} exceeds maximum {}",
///                 body.len(),
///                 self.max_size
///             )));
///         }
///         Ok(())
///     }
///
///     async fn after_parse(&self, _data: &ParsedData) -> ParseResult<()> {
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait ParserValidator: Send + Sync {
	/// Validate before parsing.
	///
	/// This hook is called before the parser processes the request body.
	/// Use it to validate content type, body size, or other pre-conditions.
	///
	/// # Arguments
	///
	/// * `content_type` - The Content-Type header value, if present
	/// * `body` - The raw request body bytes
	///
	/// # Returns
	///
	/// `Ok(())` if validation passes, `Err` otherwise
	async fn before_parse(&self, content_type: Option<&str>, body: &Bytes) -> ParseResult<()>;

	/// Validate after parsing.
	///
	/// This hook is called after the parser successfully processes the request body.
	/// Use it to validate the structure or content of the parsed data.
	///
	/// # Arguments
	///
	/// * `data` - The parsed data structure
	///
	/// # Returns
	///
	/// `Ok(())` if validation passes, `Err` otherwise
	async fn after_parse(&self, data: &ParsedData) -> ParseResult<()>;
}

/// Validator that enforces a maximum body size limit.
///
/// # Examples
///
/// ```
/// use reinhardt_core::parsers::validator::SizeLimitValidator;
///
/// // Limit requests to 1MB
/// let validator = SizeLimitValidator::new(1024 * 1024);
/// ```
#[derive(Debug, Clone)]
pub struct SizeLimitValidator {
	max_size: usize,
}

impl SizeLimitValidator {
	/// Create a new SizeLimitValidator with the specified maximum size in bytes.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::validator::SizeLimitValidator;
	///
	/// // Limit to 10KB
	/// let validator = SizeLimitValidator::new(10 * 1024);
	/// ```
	pub fn new(max_size: usize) -> Self {
		Self { max_size }
	}
}

#[async_trait]
impl ParserValidator for SizeLimitValidator {
	async fn before_parse(&self, _content_type: Option<&str>, body: &Bytes) -> ParseResult<()> {
		use crate::exception::Error;

		if body.len() > self.max_size {
			return Err(Error::Validation(format!(
				"Request body size {} exceeds maximum allowed size {}",
				body.len(),
				self.max_size
			)));
		}
		Ok(())
	}

	async fn after_parse(&self, _data: &ParsedData) -> ParseResult<()> {
		Ok(())
	}
}

/// Validator that checks for required content type.
///
/// # Examples
///
/// ```
/// use reinhardt_core::parsers::validator::ContentTypeValidator;
///
/// // Require application/json
/// let validator = ContentTypeValidator::new(vec!["application/json".to_string()]);
/// ```
#[derive(Debug, Clone)]
pub struct ContentTypeValidator {
	allowed_types: Vec<String>,
}

impl ContentTypeValidator {
	/// Create a new ContentTypeValidator with allowed content types.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::validator::ContentTypeValidator;
	///
	/// let validator = ContentTypeValidator::new(vec![
	///     "application/json".to_string(),
	///     "application/xml".to_string(),
	/// ]);
	/// ```
	pub fn new(allowed_types: Vec<String>) -> Self {
		Self { allowed_types }
	}
}

#[async_trait]
impl ParserValidator for ContentTypeValidator {
	async fn before_parse(&self, content_type: Option<&str>, _body: &Bytes) -> ParseResult<()> {
		use crate::exception::Error;

		if let Some(ct) = content_type {
			// Check if content type matches any allowed type
			let ct_lower = ct.to_lowercase();
			for allowed in &self.allowed_types {
				if ct_lower.contains(&allowed.to_lowercase()) {
					return Ok(());
				}
			}
			return Err(Error::Validation(format!(
				"Content-Type '{}' is not allowed. Allowed types: {:?}",
				ct, self.allowed_types
			)));
		}

		Err(Error::Validation(
			"Content-Type header is required".to_string(),
		))
	}

	async fn after_parse(&self, _data: &ParsedData) -> ParseResult<()> {
		Ok(())
	}
}

/// Composite validator that runs multiple validators in sequence.
///
/// # Examples
///
/// ```
/// use reinhardt_core::parsers::validator::{CompositeValidator, SizeLimitValidator, ContentTypeValidator};
///
/// let validator = CompositeValidator::new()
///     .add(SizeLimitValidator::new(1024 * 1024))
///     .add(ContentTypeValidator::new(vec!["application/json".to_string()]));
/// ```
#[derive(Default)]
pub struct CompositeValidator {
	validators: Vec<Box<dyn ParserValidator>>,
}

impl CompositeValidator {
	/// Create a new empty CompositeValidator.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::validator::CompositeValidator;
	///
	/// let validator = CompositeValidator::new();
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Add a validator to the composite.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::validator::{CompositeValidator, SizeLimitValidator};
	///
	/// let validator = CompositeValidator::new()
	///     .add(SizeLimitValidator::new(1024));
	/// ```
	#[allow(clippy::should_implement_trait)]
	pub fn add<V: ParserValidator + 'static>(mut self, validator: V) -> Self {
		self.validators.push(Box::new(validator));
		self
	}
}

#[async_trait]
impl ParserValidator for CompositeValidator {
	async fn before_parse(&self, content_type: Option<&str>, body: &Bytes) -> ParseResult<()> {
		for validator in &self.validators {
			validator.before_parse(content_type, body).await?;
		}
		Ok(())
	}

	async fn after_parse(&self, data: &ParsedData) -> ParseResult<()> {
		for validator in &self.validators {
			validator.after_parse(data).await?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[tokio::test]
	async fn test_size_limit_validator_within_limit() {
		let validator = SizeLimitValidator::new(100);
		let body = Bytes::from("small body");

		let result = validator.before_parse(None, &body).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_size_limit_validator_exceeds_limit() {
		let validator = SizeLimitValidator::new(10);
		let body = Bytes::from("this is a very long body that exceeds the limit");

		let result = validator.before_parse(None, &body).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_size_limit_validator_after_parse() {
		let validator = SizeLimitValidator::new(100);
		let data = ParsedData::Json(json!({"key": "value"}));

		let result = validator.after_parse(&data).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_content_type_validator_allowed() {
		let validator = ContentTypeValidator::new(vec!["application/json".to_string()]);
		let body = Bytes::new();

		let result = validator
			.before_parse(Some("application/json"), &body)
			.await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_content_type_validator_not_allowed() {
		let validator = ContentTypeValidator::new(vec!["application/json".to_string()]);
		let body = Bytes::new();

		let result = validator.before_parse(Some("text/plain"), &body).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_content_type_validator_missing() {
		let validator = ContentTypeValidator::new(vec!["application/json".to_string()]);
		let body = Bytes::new();

		let result = validator.before_parse(None, &body).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_content_type_validator_with_charset() {
		let validator = ContentTypeValidator::new(vec!["application/json".to_string()]);
		let body = Bytes::new();

		let result = validator
			.before_parse(Some("application/json; charset=utf-8"), &body)
			.await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_composite_validator_all_pass() {
		let validator = CompositeValidator::new()
			.add(SizeLimitValidator::new(100))
			.add(ContentTypeValidator::new(vec![
				"application/json".to_string(),
			]));

		let body = Bytes::from("small");
		let result = validator
			.before_parse(Some("application/json"), &body)
			.await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_composite_validator_first_fails() {
		let validator = CompositeValidator::new()
			.add(SizeLimitValidator::new(3))
			.add(ContentTypeValidator::new(vec![
				"application/json".to_string(),
			]));

		let body = Bytes::from("this is too long");
		let result = validator
			.before_parse(Some("application/json"), &body)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_composite_validator_second_fails() {
		let validator = CompositeValidator::new()
			.add(SizeLimitValidator::new(100))
			.add(ContentTypeValidator::new(vec![
				"application/json".to_string(),
			]));

		let body = Bytes::from("small");
		let result = validator.before_parse(Some("text/plain"), &body).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_composite_validator_after_parse() {
		let validator = CompositeValidator::new()
			.add(SizeLimitValidator::new(100))
			.add(ContentTypeValidator::new(vec![
				"application/json".to_string(),
			]));

		let data = ParsedData::Json(json!({"key": "value"}));
		let result = validator.after_parse(&data).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_composite_validator_empty() {
		let validator = CompositeValidator::new();
		let body = Bytes::from("test");

		let result = validator.before_parse(None, &body).await;
		assert!(result.is_ok());

		let data = ParsedData::Json(json!({"key": "value"}));
		let result = validator.after_parse(&data).await;
		assert!(result.is_ok());
	}
}
