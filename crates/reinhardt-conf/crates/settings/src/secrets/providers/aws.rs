//! AWS Secrets Manager provider
//!
//! This module provides integration with AWS Secrets Manager for retrieving secrets.

use crate::secrets::{SecretError, SecretMetadata, SecretProvider, SecretResult, SecretString};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;

#[cfg(feature = "aws-secrets")]
use aws_config::BehaviorVersion;
#[cfg(feature = "aws-secrets")]
use aws_sdk_secretsmanager::Client;

/// AWS Secrets Manager provider
///
/// This provider retrieves secrets from AWS Secrets Manager.
///
/// # Example
///
/// ```no_run
/// use reinhardt_settings::secrets::providers::aws::AwsSecretsProvider;
/// use reinhardt_settings::prelude::SecretProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = AwsSecretsProvider::new(None).await?;
/// let secret = provider.get_secret("database/password").await?;
/// # Ok(())
/// # }
/// ```
pub struct AwsSecretsProvider {
	#[cfg(feature = "aws-secrets")]
	client: Client,
	#[cfg(not(feature = "aws-secrets"))]
	_phantom: std::marker::PhantomData<()>,
	prefix: Option<String>,
}

impl AwsSecretsProvider {
	/// Create a new AWS Secrets Manager provider
	///
	/// # Arguments
	///
	/// * `prefix` - Optional prefix to prepend to all secret names
	///
	/// # Example
	///
	/// ```no_run
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_settings::secrets::providers::aws::AwsSecretsProvider;
	///
	// Without prefix
	/// let provider = AwsSecretsProvider::new(None).await?;
	///
	// With prefix
	/// let provider = AwsSecretsProvider::new(Some("myapp/".to_string())).await?;
	/// # Ok(())
	/// # }
	/// ```
	#[cfg(feature = "aws-secrets")]
	/// Documentation for `new`
	pub async fn new(prefix: Option<String>) -> SecretResult<Self> {
		let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
		let client = Client::new(&config);

		Ok(Self { client, prefix })
	}

	#[cfg(not(feature = "aws-secrets"))]
	/// Documentation for `new`
	pub async fn new(_prefix: Option<String>) -> SecretResult<Self> {
		Err(SecretError::Provider(
			"AWS Secrets Manager support not enabled. Enable the 'aws-secrets' feature."
				.to_string(),
		))
	}

	/// Create a provider with custom AWS config
	#[cfg(feature = "aws-secrets")]
	/// Documentation for `with_config`
	pub async fn with_config(
		config: aws_config::SdkConfig,
		prefix: Option<String>,
	) -> SecretResult<Self> {
		let client = Client::new(&config);
		Ok(Self { client, prefix })
	}

	/// Get the full secret name with prefix
	fn get_full_name(&self, key: &str) -> String {
		match &self.prefix {
			Some(prefix) => format!("{}{}", prefix, key),
			None => key.to_string(),
		}
	}

	/// Parse secret value from AWS response
	#[cfg(feature = "aws-secrets")]
	fn parse_secret_value(&self, secret_string: &str) -> SecretResult<String> {
		// Try to parse as JSON first
		if let Ok(json_value) = serde_json::from_str::<Value>(secret_string) {
			// If it's a JSON object with a single key, return that value
			if let Some(obj) = json_value.as_object()
				&& obj.len() == 1
					&& let Some(value) = obj.values().next()
						&& let Some(string_value) = value.as_str() {
							return Ok(string_value.to_string());
						}
		}

		// Otherwise, return the raw string
		Ok(secret_string.to_string())
	}
}

#[async_trait]
impl SecretProvider for AwsSecretsProvider {
	#[cfg(feature = "aws-secrets")]
	async fn get_secret(&self, key: &str) -> SecretResult<SecretString> {
		let full_name = self.get_full_name(key);

		let result = self
			.client
			.get_secret_value()
			.secret_id(&full_name)
			.send()
			.await;

		match result {
			Ok(output) => {
				if let Some(secret_string) = output.secret_string() {
					let value = self.parse_secret_value(secret_string)?;
					Ok(SecretString::new(value))
				} else {
					Err(SecretError::NotFound(format!(
						"Secret '{}' has no value",
						key
					)))
				}
			}
			Err(err) => {
				if err.to_string().contains("ResourceNotFoundException") {
					Err(SecretError::NotFound(format!(
						"Secret '{}' not found in AWS Secrets Manager",
						key
					)))
				} else {
					Err(SecretError::Provider(format!(
						"AWS Secrets Manager error: {}",
						err
					)))
				}
			}
		}
	}

	#[cfg(not(feature = "aws-secrets"))]
	async fn get_secret(&self, _key: &str) -> SecretResult<SecretString> {
		Err(SecretError::Provider(
			"AWS Secrets Manager support not enabled".to_string(),
		))
	}

	#[cfg(feature = "aws-secrets")]
	async fn get_secret_with_metadata(
		&self,
		key: &str,
	) -> SecretResult<(SecretString, SecretMetadata)> {
		let full_name = self.get_full_name(key);

		let result = self
			.client
			.get_secret_value()
			.secret_id(&full_name)
			.send()
			.await;

		match result {
			Ok(output) => {
				if let Some(secret_string) = output.secret_string() {
					let value = self.parse_secret_value(secret_string)?;

					let metadata = SecretMetadata {
						created_at: output.created_date().map(|dt| {
							chrono::DateTime::from_timestamp(dt.secs(), dt.subsec_nanos())
								.unwrap_or_else(Utc::now)
						}),
						updated_at: Some(Utc::now()),
					};

					Ok((SecretString::new(value), metadata))
				} else {
					Err(SecretError::NotFound(format!(
						"Secret '{}' has no value",
						key
					)))
				}
			}
			Err(err) => {
				if err.to_string().contains("ResourceNotFoundException") {
					Err(SecretError::NotFound(format!(
						"Secret '{}' not found in AWS Secrets Manager",
						key
					)))
				} else {
					Err(SecretError::Provider(format!(
						"AWS Secrets Manager error: {}",
						err
					)))
				}
			}
		}
	}

	#[cfg(not(feature = "aws-secrets"))]
	async fn get_secret_with_metadata(
		&self,
		_key: &str,
	) -> SecretResult<(SecretString, SecretMetadata)> {
		Err(SecretError::Provider(
			"AWS Secrets Manager support not enabled".to_string(),
		))
	}

	#[cfg(feature = "aws-secrets")]
	async fn set_secret(&self, key: &str, value: SecretString) -> SecretResult<()> {
		let full_name = self.get_full_name(key);

		// Try to update existing secret first
		let update_result = self
			.client
			.update_secret()
			.secret_id(&full_name)
			.secret_string(value.expose_secret())
			.send()
			.await;

		match update_result {
			Ok(_) => Ok(()),
			Err(err) => {
				// If secret doesn't exist, create it
				if err.to_string().contains("ResourceNotFoundException") {
					self.client
						.create_secret()
						.name(&full_name)
						.secret_string(value.expose_secret())
						.send()
						.await
						.map_err(|e| {
							SecretError::Provider(format!("Failed to create secret: {}", e))
						})?;
					Ok(())
				} else {
					Err(SecretError::Provider(format!(
						"Failed to update secret: {}",
						err
					)))
				}
			}
		}
	}

	#[cfg(not(feature = "aws-secrets"))]
	async fn set_secret(&self, _key: &str, _value: SecretString) -> SecretResult<()> {
		Err(SecretError::Provider(
			"AWS Secrets Manager support not enabled".to_string(),
		))
	}

	#[cfg(feature = "aws-secrets")]
	async fn delete_secret(&self, key: &str) -> SecretResult<()> {
		let full_name = self.get_full_name(key);

		self.client
			.delete_secret()
			.secret_id(&full_name)
			.force_delete_without_recovery(true)
			.send()
			.await
			.map_err(|e| SecretError::Provider(format!("Failed to delete secret: {}", e)))?;

		Ok(())
	}

	#[cfg(not(feature = "aws-secrets"))]
	async fn delete_secret(&self, _key: &str) -> SecretResult<()> {
		Err(SecretError::Provider(
			"AWS Secrets Manager support not enabled".to_string(),
		))
	}

	#[cfg(feature = "aws-secrets")]
	async fn list_secrets(&self) -> SecretResult<Vec<String>> {
		let mut secrets = Vec::new();
		let mut next_token: Option<String> = None;

		loop {
			let mut request = self.client.list_secrets();

			if let Some(token) = next_token {
				request = request.next_token(token);
			}

			let response = request
				.send()
				.await
				.map_err(|e| SecretError::Provider(format!("Failed to list secrets: {}", e)))?;

			for secret in response.secret_list() {
				if let Some(name) = secret.name() {
					// Remove prefix if present
					let key = if let Some(prefix) = &self.prefix {
						if let Some(stripped) = name.strip_prefix(prefix) {
							stripped.to_string()
						} else {
							continue; // Skip secrets that don't match our prefix
						}
					} else {
						name.to_string()
					};

					secrets.push(key);
				}
			}

			next_token = response.next_token().map(|s| s.to_string());

			if next_token.is_none() {
				break;
			}
		}

		Ok(secrets)
	}

	#[cfg(not(feature = "aws-secrets"))]
	async fn list_secrets(&self) -> SecretResult<Vec<String>> {
		Err(SecretError::Provider(
			"AWS Secrets Manager support not enabled".to_string(),
		))
	}

	fn exists(&self, _key: &str) -> bool {
		// Cannot make async calls in sync method
		// Consumers should use get_secret() and check for NotFound error
		false
	}

	fn name(&self) -> &str {
		"aws-secrets-manager"
	}
}

#[cfg(all(test, feature = "aws-secrets"))]
mod tests {
	use super::*;

	// Note: These tests require AWS credentials and won't run in CI
	// They are here for local testing purposes

	#[tokio::test]
	#[ignore] // Ignore by default as it requires AWS credentials
	async fn test_aws_provider_creation() {
		let result = AwsSecretsProvider::new(None).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	#[ignore]
	async fn test_aws_get_nonexistent_secret() {
		let provider = AwsSecretsProvider::new(Some("test/".to_string()))
			.await
			.unwrap();

		let result = provider.get_secret("nonexistent-secret-12345").await;

		assert!(result.is_err());
		if let Err(SecretError::NotFound(_)) = result {
			// Expected
		} else {
			panic!("Expected NotFound error");
		}
	}
}

#[cfg(all(test, not(feature = "aws-secrets")))]
mod tests_no_feature {
	use super::*;

	#[tokio::test]
	async fn test_aws_provider_disabled() {
		let result = AwsSecretsProvider::new(None).await;
		assert!(result.is_err());
	}
}
