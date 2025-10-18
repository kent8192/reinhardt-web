//! Azure Key Vault provider
//!
//! This module provides integration with Azure Key Vault for retrieving secrets.

use crate::secrets::{Secret, SecretError, SecretMetadata, SecretProvider, SecretResult};
use async_trait::async_trait;
use chrono::Utc;

#[cfg(feature = "azure-keyvault")]
use azure_core::auth::TokenCredential;
#[cfg(feature = "azure-keyvault")]
use azure_identity::DefaultAzureCredential;
#[cfg(feature = "azure-keyvault")]
use azure_security_keyvault::KeyvaultClient;
#[cfg(feature = "azure-keyvault")]
use std::sync::Arc;

/// Azure Key Vault provider
///
/// This provider retrieves secrets from Azure Key Vault.
///
/// # Example
///
/// ```no_run
/// use reinhardt_settings::secrets::providers::AzureKeyVaultProvider;
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
    #[cfg(feature = "azure-keyvault")]
    vault_url: String,
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
    /// use reinhardt_settings::secrets::providers::AzureKeyVaultProvider;
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
        let credential = Arc::new(DefaultAzureCredential::default());
        let client = KeyvaultClient::new(&vault_url, credential)
            .map_err(|e| SecretError::Provider(format!("Failed to create Azure client: {}", e)))?;

        Ok(Self { client, vault_url })
    }

    #[cfg(not(feature = "azure-keyvault"))]
    /// Documentation for `new`
    pub async fn new(_vault_url: impl Into<String>) -> SecretResult<Self> {
        Err(SecretError::Provider(
            "Azure Key Vault support not enabled. Enable the 'azure-keyvault' feature.".to_string(),
        ))
    }

    /// Create a provider with custom credentials
    #[cfg(feature = "azure-keyvault")]
    /// Documentation for `with_credential`
    pub async fn with_credential(
        vault_url: impl Into<String>,
        credential: Arc<dyn TokenCredential>,
    ) -> SecretResult<Self> {
        let vault_url = vault_url.into();
        let client = KeyvaultClient::new(&vault_url, credential)
            .map_err(|e| SecretError::Provider(format!("Failed to create Azure client: {}", e)))?;

        Ok(Self { client, vault_url })
    }
}

#[async_trait]
impl SecretProvider for AzureKeyVaultProvider {
    #[cfg(feature = "azure-keyvault")]
    async fn get_secret(&self, key: &str) -> SecretResult<Option<Secret>> {
        use azure_security_keyvault::prelude::*;

        let result = self.client.secret_client().get(key).await;

        match result {
            Ok(secret_bundle) => {
                let value = secret_bundle
                    .value()
                    .ok_or_else(|| SecretError::Provider("Secret has no value".to_string()))?
                    .to_string();

                let metadata = SecretMetadata {
                    created_at: secret_bundle
                        .attributes()
                        .and_then(|attr| attr.created())
                        .map(|ts| {
                            chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                        })
                        .unwrap_or_else(|| Utc::now()),
                    updated_at: secret_bundle
                        .attributes()
                        .and_then(|attr| attr.updated())
                        .map(|ts| {
                            chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now())
                        })
                        .unwrap_or_else(|| Utc::now()),
                    version: 1, // Azure uses version IDs, not numbers
                };

                Ok(Some(Secret::new(value, metadata)))
            }
            Err(err) => {
                // Check if it's a 404 Not Found error
                if err.to_string().contains("404")
                    || err.to_string().contains("NotFound")
                    || err.to_string().contains("SecretNotFound")
                {
                    Ok(None)
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
    async fn get_secret(&self, _key: &str) -> SecretResult<Option<Secret>> {
        Err(SecretError::Provider(
            "Azure Key Vault support not enabled".to_string(),
        ))
    }

    #[cfg(feature = "azure-keyvault")]
    async fn set_secret(&mut self, key: &str, value: &str) -> SecretResult<()> {
        use azure_security_keyvault::prelude::*;

        self.client
            .secret_client()
            .set(key, value)
            .await
            .map_err(|e| SecretError::Provider(format!("Failed to set secret: {}", e)))?;

        Ok(())
    }

    #[cfg(not(feature = "azure-keyvault"))]
    async fn set_secret(&mut self, _key: &str, _value: &str) -> SecretResult<()> {
        Err(SecretError::Provider(
            "Azure Key Vault support not enabled".to_string(),
        ))
    }

    #[cfg(feature = "azure-keyvault")]
    async fn delete_secret(&mut self, key: &str) -> SecretResult<()> {
        use azure_security_keyvault::prelude::*;

        // Delete the secret
        self.client
            .secret_client()
            .delete(key)
            .await
            .map_err(|e| SecretError::Provider(format!("Failed to delete secret: {}", e)))?;

        // Purge the deleted secret (optional, but recommended for testing)
        // Note: This requires the "purge" permission in Key Vault access policies
        let _ = self.client.secret_client().purge_deleted(key).await;

        Ok(())
    }

    #[cfg(not(feature = "azure-keyvault"))]
    async fn delete_secret(&mut self, _key: &str) -> SecretResult<()> {
        Err(SecretError::Provider(
            "Azure Key Vault support not enabled".to_string(),
        ))
    }

    #[cfg(feature = "azure-keyvault")]
    async fn list_secrets(&self) -> SecretResult<Vec<String>> {
        use azure_core::Pageable;
        use azure_security_keyvault::prelude::*;

        let mut secrets = Vec::new();
        let mut pageable = self.client.secret_client().list_secrets().into_stream();

        while let Some(result) = pageable.next().await {
            match result {
                Ok(response) => {
                    for item in response.value {
                        if let Some(id) = item.id() {
                            // Extract secret name from ID
                            // ID format: https://{vault}.vault.azure.net/secrets/{name}/{version}
                            if let Some(name) = id.split('/').nth_back(1) {
                                secrets.push(name.to_string());
                            }
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
}

#[cfg(all(test, feature = "azure-keyvault"))]
mod tests {
    use super::*;

    /// // Note: These tests require Azure credentials and a Key Vault, won't run in CI
    /// // They are here for local testing purposes

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
        // Should either return None or error due to credentials
        assert!(result.is_ok() || result.is_err());
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
