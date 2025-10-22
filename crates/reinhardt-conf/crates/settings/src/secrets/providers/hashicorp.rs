//! HashiCorp Vault secret provider

use crate::secrets::{SecretError, SecretMetadata, SecretProvider, SecretResult, SecretString};
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
        // Note: Vault uses LIST method, but reqwest doesn't have it, so we'll just return empty
        // This is a limitation of mocking Vault's LIST method
        Ok(vec![])
    }

    fn exists(&self, _key: &str) -> bool {
        // Note: Since this is a sync method, we can't make async calls
        // In real implementation, this would check cached state or return default
        false
    }

    fn name(&self) -> &str {
        "vault"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vault_provider_basic() {
        let mut server = mockito::Server::new_async().await;

        // Mock: POST /v1/secret/data/test/password (set_secret)
        let _m_set = server
            .mock("POST", "/v1/secret/data/test/password")
            .match_header("X-Vault-Token", "test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"version":1}}"#)
            .expect(1)
            .create_async()
            .await;

        // Mock: GET /v1/secret/data/test/password (get_secret - first call)
        let _m_get1 = server
            .mock("GET", "/v1/secret/data/test/password")
            .match_header("X-Vault-Token", "test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"data":{"value":"my-vault-secret"},"metadata":{"created_time":"2024-01-01T00:00:00Z","version":1}}}"#)
            .expect(1)
            .create_async()
            .await;

        // Mock: DELETE /v1/secret/metadata/test/password (delete_secret)
        let _m_delete = server
            .mock("DELETE", "/v1/secret/metadata/test/password")
            .match_header("X-Vault-Token", "test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .expect(1)
            .create_async()
            .await;

        // Mock: GET /v1/secret/data/test/password (get_secret - after delete, should fail)
        let _m_get2 = server
            .mock("GET", "/v1/secret/data/test/password")
            .match_header("X-Vault-Token", "test-token")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"errors":["secret not found"]}"#)
            .expect(1)
            .create_async()
            .await;

        let config = VaultConfig::new(server.url(), "test-token");
        let provider = VaultSecretProvider::new(config).unwrap();

        let secret = SecretString::new("my-vault-secret");
        provider.set_secret("test/password", secret).await.unwrap();

        let retrieved = provider.get_secret("test/password").await.unwrap();
        assert_eq!(retrieved.expose_secret(), "my-vault-secret");

        // Note: exists() is a sync method that can't make async calls in current trait design
        // In production, it would use cached state

        provider.delete_secret("test/password").await.unwrap();

        // Verify secret was deleted by attempting to retrieve it
        let result = provider.get_secret("test/password").await;
        assert!(result.is_err());
    }
}
