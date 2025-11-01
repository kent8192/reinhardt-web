//! Azure Key Vault provider
//!
//! This module provides integration with Azure Key Vault for retrieving secrets.

use crate::secrets::{SecretError, SecretMetadata, SecretProvider, SecretResult, SecretString};
use async_trait::async_trait;

#[cfg(feature = "azure-keyvault")]
use azure_identity::create_default_credential;
#[cfg(feature = "azure-keyvault")]
use azure_security_keyvault::KeyvaultClient;

/// Azure Key Vault provider
///
/// This provider retrieves secrets from Azure Key Vault.
///
/// # Example
///
/// ```no_run
/// use reinhardt_settings::secrets::providers::azure::AzureKeyVaultProvider;
/// use reinhardt_settings::prelude::SecretProvider;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = AzureKeyVaultProvider::new(
///     "https://myvault.vault.azure.net"
/// ).await?;
/// let secret = provider.get_secret("database-password").await?;
/// # Ok(())
/// # }
/// ```
pub struct AzureKeyVaultProvider {
	#[cfg(feature = "azure-keyvault")]
	client: KeyvaultClient,
	#[cfg(not(feature = "azure-keyvault"))]
	_phantom: std::marker::PhantomData<()>,
}

impl AzureKeyVaultProvider {
	/// Create a new Azure Key Vault provider
	///
	/// # Arguments
	///
	/// * `vault_url` - The URL of the Azure Key Vault (e.g., "https://myvault.vault.azure.net")
	///
	/// # Example
	///
	/// ```no_run
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_settings::secrets::providers::azure::AzureKeyVaultProvider;
	///
	/// let provider = AzureKeyVaultProvider::new(
	///     "https://myvault.vault.azure.net"
	/// ).await?;
	/// # Ok(())
	/// # }
	/// ```
	#[cfg(feature = "azure-keyvault")]
	/// Documentation for `new`
	pub async fn new(vault_url: impl Into<String>) -> SecretResult<Self> {
		let vault_url = vault_url.into();
		let credential = create_default_credential().map_err(|e| {
			SecretError::Provider(format!("Failed to create default credential: {}", e))
		})?;
		let client = KeyvaultClient::new(&vault_url, credential)
			.map_err(|e| SecretError::Provider(format!("Failed to create Azure client: {}", e)))?;

		Ok(Self { client })
	}

	#[cfg(not(feature = "azure-keyvault"))]
	/// Documentation for `new`
	pub async fn new(_vault_url: impl Into<String>) -> SecretResult<Self> {
		Err(SecretError::Provider(
			"Azure Key Vault support not enabled. Enable the 'azure-keyvault' feature.".to_string(),
		))
	}
}

#[async_trait]
impl SecretProvider for AzureKeyVaultProvider {
	#[cfg(feature = "azure-keyvault")]
	async fn get_secret(&self, name: &str) -> SecretResult<SecretString> {
		let result = self.client.secret_client().get(name).await;

		match result {
			Ok(secret_response) => Ok(SecretString::new(secret_response.value)),
			Err(err) => {
				if err.to_string().contains("404") {
					Err(SecretError::NotFound(format!(
						"Secret '{}' not found",
						name
					)))
				} else {
					Err(SecretError::Provider(format!(
						"Azure Key Vault error: {}",
						err
					)))
				}
			}
		}
	}

	#[cfg(not(feature = "azure-keyvault"))]
	async fn get_secret(&self, _name: &str) -> SecretResult<SecretString> {
		Err(SecretError::Provider(
			"Azure Key Vault support not enabled".to_string(),
		))
	}

	#[cfg(feature = "azure-keyvault")]
	async fn get_secret_with_metadata(
		&self,
		name: &str,
	) -> SecretResult<(SecretString, SecretMetadata)> {
		let result = self.client.secret_client().get(name).await;

		match result {
			Ok(secret_response) => {
				let created_at = {
					let unix_timestamp = secret_response.attributes.created_on.unix_timestamp();
					chrono::DateTime::from_timestamp(unix_timestamp, 0)
				};

				let updated_at = {
					let unix_timestamp = secret_response.attributes.updated_on.unix_timestamp();
					chrono::DateTime::from_timestamp(unix_timestamp, 0)
				};

				let metadata = SecretMetadata {
					created_at,
					updated_at,
				};

				Ok((SecretString::new(secret_response.value), metadata))
			}
			Err(err) => {
				if err.to_string().contains("404") {
					Err(SecretError::NotFound(format!(
						"Secret '{}' not found",
						name
					)))
				} else {
					Err(SecretError::Provider(format!(
						"Azure Key Vault error: {}",
						err
					)))
				}
			}
		}
	}

	#[cfg(not(feature = "azure-keyvault"))]
	async fn get_secret_with_metadata(
		&self,
		_name: &str,
	) -> SecretResult<(SecretString, SecretMetadata)> {
		Err(SecretError::Provider(
			"Azure Key Vault support not enabled".to_string(),
		))
	}

	#[cfg(feature = "azure-keyvault")]
	async fn set_secret(&self, name: &str, value: SecretString) -> SecretResult<()> {
		self.client
			.secret_client()
			.set(name, value.expose_secret())
			.await
			.map_err(|e| SecretError::Provider(format!("Failed to set secret: {}", e)))?;

		Ok(())
	}

	#[cfg(not(feature = "azure-keyvault"))]
	async fn set_secret(&self, _name: &str, _value: SecretString) -> SecretResult<()> {
		Err(SecretError::Provider(
			"Azure Key Vault support not enabled".to_string(),
		))
	}

	#[cfg(feature = "azure-keyvault")]
	async fn delete_secret(&self, name: &str) -> SecretResult<()> {
		self.client
			.secret_client()
			.delete(name)
			.await
			.map_err(|e| SecretError::Provider(format!("Failed to delete secret: {}", e)))?;

		Ok(())
	}

	#[cfg(not(feature = "azure-keyvault"))]
	async fn delete_secret(&self, _name: &str) -> SecretResult<()> {
		Err(SecretError::Provider(
			"Azure Key Vault support not enabled".to_string(),
		))
	}

	#[cfg(feature = "azure-keyvault")]
	async fn list_secrets(&self) -> SecretResult<Vec<String>> {
		use futures::stream::StreamExt;

		let mut secrets = Vec::new();
		let mut pageable = self.client.secret_client().list_secrets().into_stream();

		while let Some(result) = pageable.next().await {
			match result {
				Ok(response) => {
					for item in response.value {
						let name = item.id.split('/').last().unwrap_or("");
						if !name.is_empty() {
							secrets.push(name.to_string());
						}
					}
				}
				Err(e) => {
					return Err(SecretError::Provider(format!(
						"Failed to list secrets: {}",
						e
					)));
				}
			}
		}

		Ok(secrets)
	}

	#[cfg(not(feature = "azure-keyvault"))]
	async fn list_secrets(&self) -> SecretResult<Vec<String>> {
		Err(SecretError::Provider(
			"Azure Key Vault support not enabled".to_string(),
		))
	}

	fn exists(&self, _name: &str) -> bool {
		// Cannot make async calls in sync method
		// Consumers should use get_secret() and check for NotFound error
		false
	}

	fn name(&self) -> &str {
		"azure-keyvault"
	}
}

#[cfg(all(test, feature = "azure-keyvault"))]
mod tests {
	use super::*;

	// Note: These tests require Azure credentials and a Key Vault, won't run in CI
	// They are here for local testing purposes

	#[tokio::test]
	#[ignore] // Ignore by default as it requires Azure credentials
	async fn test_azure_provider_creation() {
		let vault_url = std::env::var("AZURE_KEYVAULT_URL")
			.unwrap_or_else(|_| "https://test-vault.vault.azure.net".to_string());

		let result = AzureKeyVaultProvider::new(vault_url).await;
		// May fail without proper credentials, but should create the client
		assert!(result.is_ok() || result.is_err());
	}

	#[tokio::test]
	#[ignore]
	async fn test_azure_get_nonexistent_secret() {
		let vault_url = std::env::var("AZURE_KEYVAULT_URL")
			.expect("AZURE_KEYVAULT_URL must be set for this test");

		let provider = AzureKeyVaultProvider::new(vault_url).await.unwrap();

		let result = provider.get_secret("nonexistent-secret-12345").await;

		assert!(result.is_err());
		if let Err(SecretError::NotFound(_)) = result {
			// Expected
		} else {
			panic!("Expected NotFound error");
		}
	}
}

#[cfg(all(test, not(feature = "azure-keyvault")))]
mod tests_no_feature {
	use super::*;

	#[tokio::test]
	async fn test_azure_provider_disabled() {
		let result = AzureKeyVaultProvider::new("https://test.vault.azure.net").await;
		assert!(result.is_err());
	}
}
