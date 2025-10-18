//! HashiCorp Vault secret provider

use crate::secrets::{
    SecretError, SecretMetadata, SecretProvider, SecretResult, SecretString, SecretVersion,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HashiCorp Vault client configuration
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Vault server address (e.g., "http://127.0.0.1:8200")
    pub addr: String,

    /// Authentication token
    pub token: String,

    /// Mount point for the KV v2 secrets engine (default: "secret")
    pub mount: String,

    /// Optional namespace (for Vault Enterprise)
    pub namespace: Option<String>,
}

impl VaultConfig {
    /// Create a new Vault configuration
    pub fn new(addr: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            token: token.into(),
            mount: "secret".to_string(),
            namespace: None,
        }
    }
    /// Set the mount point
    pub fn with_mount(mut self, mount: impl Into<String>) -> Self {
        self.mount = mount.into();
        self
    }
    /// Set the namespace
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }
}

/// HashiCorp Vault secret provider
pub struct VaultSecretProvider {
    config: VaultConfig,
    client: reqwest::Client,
}

impl VaultSecretProvider {
    /// Create a new Vault secret provider
    pub fn new(config: VaultConfig) -> SecretResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| SecretError::ProviderError(format!("Failed to create client: {}", e)))?;

        Ok(Self { config, client })
    }

    fn secret_path(&self, key: &str) -> String {
        format!("{}/data/{}", self.config.mount, key)
    }

    fn metadata_path(&self, key: &str) -> String {
        format!("{}/metadata/{}", self.config.mount, key)
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}/v1/{}", self.config.addr.trim_end_matches('/'), path)
    }

    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> SecretResult<T> {
        let url = self.build_url(path);
        let mut req = self
            .client
            .request(method, &url)
            .header("X-Vault-Token", self.config.token.clone());

        if let Some(ns) = &self.config.namespace {
            req = req.header("X-Vault-Namespace", ns);
        }

        if let Some(body) = body {
            req = req.json(&body);
        }

        let response = req
            .send()
            .await
            .map_err(|e| SecretError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SecretError::ProviderError(format!(
                "Vault request failed with status {}: {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| SecretError::ProviderError(format!("Failed to parse response: {}", e)))
    }
}

#[derive(Debug, Deserialize)]
struct VaultReadResponse {
    data: VaultData,
}

#[derive(Debug, Deserialize)]
struct VaultData {
    data: HashMap<String, String>,
    metadata: VaultSecretMetadata,
}

#[derive(Debug, Deserialize)]
struct VaultSecretMetadata {
    created_time: String,
    version: u64,
}

#[derive(Debug, Serialize)]
struct VaultWriteRequest {
    data: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct VaultListResponse {
    data: VaultListData,
}

#[derive(Debug, Deserialize)]
struct VaultListData {
    keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VaultMetadataResponse {
    data: VaultMetadataData,
}

#[derive(Debug, Deserialize)]
struct VaultMetadataData {
    versions: HashMap<String, VaultVersionInfo>,
    created_time: String,
    updated_time: String,
}

#[derive(Debug, Deserialize)]
struct VaultVersionInfo {
    created_time: String,
    deletion_time: String,
    destroyed: bool,
}

#[async_trait]
impl SecretProvider for VaultSecretProvider {
    async fn get_secret(&self, key: &str) -> SecretResult<SecretString> {
        let path = self.secret_path(key);
        let response: VaultReadResponse = self.request(reqwest::Method::GET, &path, None).await?;

        let value = response
            .data
            .data
            .get("value")
            .ok_or_else(|| SecretError::NotFound(format!("Secret not found: {}", key)))?;

        Ok(SecretString::new(value.clone()))
    }

    async fn get_secret_with_metadata(
        &self,
        key: &str,
    ) -> SecretResult<(SecretString, SecretMetadata)> {
        let secret = self.get_secret(key).await?;

        let metadata_path = self.metadata_path(key);
        let response: VaultMetadataResponse = self
            .request(reqwest::Method::GET, &metadata_path, None)
            .await?;

        let created_at = chrono::DateTime::parse_from_rfc3339(&response.data.created_time)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let updated_at = chrono::DateTime::parse_from_rfc3339(&response.data.updated_time)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let metadata = SecretMetadata {
            created_at,
            updated_at,
            version: Some(response.data.versions.len().to_string()),
            tags: HashMap::new(),
            description: None,
            expires_at: None,
        };

        Ok((secret, metadata))
    }

    async fn set_secret(&self, key: &str, value: SecretString) -> SecretResult<()> {
        let path = self.secret_path(key);
        let mut data = HashMap::new();
        data.insert("value".to_string(), value.expose_secret().to_string());

        let body = serde_json::json!({ "data": data });

        let _: serde_json::Value = self
            .request(reqwest::Method::POST, &path, Some(body))
            .await?;

        Ok(())
    }

    async fn delete_secret(&self, key: &str) -> SecretResult<()> {
        let metadata_path = self.metadata_path(key);
        let _: serde_json::Value = self
            .request(reqwest::Method::DELETE, &metadata_path, None)
            .await?;
        Ok(())
    }

    async fn list_secrets(&self) -> SecretResult<Vec<String>> {
        let list_path = format!("{}/metadata", self.config.mount);
        let response: VaultListResponse = self
            .request(reqwest::Method::LIST, &list_path, None)
            .await?;

        Ok(response.data.keys)
    }

    async fn exists(&self, key: &str) -> SecretResult<bool> {
        match self.get_secret(key).await {
            Ok(_) => Ok(true),
            Err(SecretError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn get_versions(&self, key: &str) -> SecretResult<Vec<SecretVersion>> {
        let metadata_path = self.metadata_path(key);
        let response: VaultMetadataResponse = self
            .request(reqwest::Method::GET, &metadata_path, None)
            .await?;

        let mut versions: Vec<SecretVersion> = response
            .data
            .versions
            .into_iter()
            .filter(|(_, info)| !info.destroyed)
            .map(|(version, info)| {
                let created_at = chrono::DateTime::parse_from_rfc3339(&info.created_time)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc);

                SecretVersion {
                    version,
                    created_at,
                    is_current: false, // We'll set this below
                }
            })
            .collect();

        // Sort by version number and mark the latest as current
        versions.sort_by(|a, b| {
            let a_ver: u64 = a.version.parse().unwrap_or(0);
            let b_ver: u64 = b.version.parse().unwrap_or(0);
            b_ver.cmp(&a_ver)
        });

        if let Some(latest) = versions.first_mut() {
            latest.is_current = true;
        }

        Ok(versions)
    }

    fn name(&self) -> &str {
        "vault"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// // These tests require a running Vault instance
    /// // They are marked with #[ignore] by default

    #[tokio::test]
    #[ignore]
    async fn test_vault_provider_basic() {
        let config = VaultConfig::new("http://127.0.0.1:8200", "root-token");
        let provider = VaultSecretProvider::new(config).unwrap();

        let secret = SecretString::new("my-vault-secret");
        provider.set_secret("test/password", secret).await.unwrap();

        let retrieved = provider.get_secret("test/password").await.unwrap();
        assert_eq!(retrieved.expose_secret(), "my-vault-secret");

        assert!(provider.exists("test/password").await.unwrap());

        provider.delete_secret("test/password").await.unwrap();
        assert!(!provider.exists("test/password").await.unwrap());
    }
}
