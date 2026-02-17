//! Azure Blob Storage backend
//!
//! Provides storage backend for Azure Blob Storage

use super::Storage;
use async_trait::async_trait;
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::*;
use std::io;

/// Azure Blob Storage configuration
#[derive(Debug, Clone)]
pub struct AzureBlobConfig {
	/// Azure storage account name
	pub account_name: String,
	/// Azure storage account key
	pub account_key: Option<String>,
	/// SAS token (alternative to account key)
	pub sas_token: Option<String>,
	/// Container name
	pub container: String,
	/// Path prefix within container
	pub prefix: Option<String>,
	/// Base URL for generating file URLs
	pub base_url: String,
}

impl AzureBlobConfig {
	/// Create a new Azure Blob Storage configuration
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_utils::staticfiles::storage::AzureBlobConfig;
	///
	/// let config = AzureBlobConfig::new(
	///     "mystorageaccount".to_string(),
	///     "mycontainer".to_string(),
	/// )
	/// .with_account_key("ACCOUNT_KEY".to_string());
	/// ```
	pub fn new(account_name: String, container: String) -> Self {
		let base_url = format!(
			"https://{}.blob.core.windows.net/{}",
			account_name, container
		);
		Self {
			account_name,
			account_key: None,
			sas_token: None,
			container,
			prefix: None,
			base_url,
		}
	}

	/// Set Azure storage account key
	pub fn with_account_key(mut self, account_key: String) -> Self {
		self.account_key = Some(account_key);
		self
	}

	/// Set SAS token (alternative to account key)
	pub fn with_sas_token(mut self, sas_token: String) -> Self {
		self.sas_token = Some(sas_token);
		self
	}

	/// Set path prefix within container
	pub fn with_prefix(mut self, prefix: String) -> Self {
		self.prefix = Some(prefix.trim_matches('/').to_string());
		self
	}

	/// Set base URL for file URLs
	pub fn with_base_url(mut self, base_url: String) -> Self {
		self.base_url = base_url.trim_end_matches('/').to_string();
		self
	}
}

/// Azure Blob Storage backend
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_utils::staticfiles::storage::{AzureBlobStorage, AzureBlobConfig, Storage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = AzureBlobConfig::new(
///     "mystorageaccount".to_string(),
///     "static-files".to_string(),
/// )
/// .with_account_key("ACCOUNT_KEY".to_string());
///
/// let storage = AzureBlobStorage::new(config).await?;
///
/// // Save a file
/// let url = storage.save("css/style.css", b"body { color: red; }").await?;
/// # Ok(())
/// # }
/// ```
pub struct AzureBlobStorage {
	client: ContainerClient,
	config: AzureBlobConfig,
}

impl AzureBlobStorage {
	/// Create a new Azure Blob Storage backend
	pub async fn new(config: AzureBlobConfig) -> io::Result<Self> {
		let client = Self::create_client(&config)?;
		Ok(Self { client, config })
	}

	/// Create Azure Blob Storage client from configuration
	fn create_client(config: &AzureBlobConfig) -> io::Result<ContainerClient> {
		let credentials = if let Some(account_key) = &config.account_key {
			StorageCredentials::access_key(config.account_name.clone(), account_key.clone())
		} else if let Some(sas_token) = &config.sas_token {
			StorageCredentials::sas_token(sas_token.clone())
				.map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
		} else {
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"Either account_key or sas_token must be provided",
			));
		};

		let client = ClientBuilder::new(config.account_name.clone(), credentials)
			.container_client(config.container.clone());

		Ok(client)
	}

	/// Get full blob name with prefix
	fn get_full_blob_name(&self, name: &str) -> String {
		let name = name.trim_start_matches('/');
		if let Some(prefix) = &self.config.prefix {
			format!("{}/{}", prefix, name)
		} else {
			name.to_string()
		}
	}

	/// Generate public URL for a file
	fn generate_url(&self, name: &str) -> String {
		let name = name.trim_start_matches('/');
		if let Some(prefix) = &self.config.prefix {
			format!("{}/{}/{}", self.config.base_url, prefix, name)
		} else {
			format!("{}/{}", self.config.base_url, name)
		}
	}
}

#[async_trait]
impl Storage for AzureBlobStorage {
	async fn save(&self, name: &str, content: &[u8]) -> io::Result<String> {
		let blob_name = self.get_full_blob_name(name);
		let content_vec = content.to_vec();

		self.client
			.blob_client(blob_name)
			.put_block_blob(content_vec)
			.await
			.map_err(|e| io::Error::other(e.to_string()))?;

		Ok(self.url(name))
	}

	fn exists(&self, name: &str) -> bool {
		let blob_name = self.get_full_blob_name(name);
		let client = self.client.clone();

		tokio::task::block_in_place(|| {
			tokio::runtime::Handle::current()
				.block_on(async { client.blob_client(blob_name).get_properties().await.is_ok() })
		})
	}

	async fn open(&self, name: &str) -> io::Result<Vec<u8>> {
		let blob_name = self.get_full_blob_name(name);

		let data = self
			.client
			.blob_client(blob_name)
			.get_content()
			.await
			.map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

		Ok(data)
	}

	async fn delete(&self, name: &str) -> io::Result<()> {
		let blob_name = self.get_full_blob_name(name);

		self.client
			.blob_client(blob_name)
			.delete()
			.await
			.map_err(|e| io::Error::other(e.to_string()))?;

		Ok(())
	}

	fn url(&self, name: &str) -> String {
		self.generate_url(name)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_azure_config_creation() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string());

		assert_eq!(config.account_name, "teststorage");
		assert_eq!(config.container, "testcontainer");
		assert_eq!(
			config.base_url,
			"https://teststorage.blob.core.windows.net/testcontainer"
		);
	}

	#[rstest]
	fn test_azure_config_with_account_key() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_account_key("ACCOUNT_KEY".to_string());

		assert_eq!(config.account_key, Some("ACCOUNT_KEY".to_string()));
	}

	#[rstest]
	fn test_azure_config_with_sas_token() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_sas_token("?sv=2021-01-01&sig=...".to_string());

		assert_eq!(config.sas_token, Some("?sv=2021-01-01&sig=...".to_string()));
	}

	#[rstest]
	fn test_azure_config_with_prefix() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_prefix("static".to_string());

		assert_eq!(config.prefix, Some("static".to_string()));
	}

	#[rstest]
	fn test_azure_config_with_base_url() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_base_url("https://cdn.example.com".to_string());

		assert_eq!(config.base_url, "https://cdn.example.com");
	}

	#[rstest]
	fn test_full_blob_name_generation() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string());

		// Create storage without actual client (for testing only)
		let credentials = StorageCredentials::access_key("teststorage".to_string(), "fake_key");
		let client = ClientBuilder::new("teststorage".to_string(), credentials)
			.container_client("testcontainer".to_string());

		let storage = AzureBlobStorage {
			client,
			config: config.clone(),
		};

		assert_eq!(storage.get_full_blob_name("file.txt"), "file.txt");
		assert_eq!(storage.get_full_blob_name("/file.txt"), "file.txt");
	}

	#[rstest]
	fn test_full_blob_name_with_prefix() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_prefix("static".to_string());

		let credentials = StorageCredentials::access_key("teststorage".to_string(), "fake_key");
		let client = ClientBuilder::new("teststorage".to_string(), credentials)
			.container_client("testcontainer".to_string());

		let storage = AzureBlobStorage {
			client,
			config: config.clone(),
		};

		assert_eq!(storage.get_full_blob_name("file.txt"), "static/file.txt");
		assert_eq!(storage.get_full_blob_name("/file.txt"), "static/file.txt");
	}

	#[rstest]
	fn test_url_generation() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string());

		let credentials = StorageCredentials::access_key("teststorage".to_string(), "fake_key");
		let client = ClientBuilder::new("teststorage".to_string(), credentials)
			.container_client("testcontainer".to_string());

		let storage = AzureBlobStorage {
			client,
			config: config.clone(),
		};

		assert_eq!(
			storage.url("file.txt"),
			"https://teststorage.blob.core.windows.net/testcontainer/file.txt"
		);
	}

	#[rstest]
	fn test_url_generation_with_prefix() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_prefix("static".to_string());

		let credentials = StorageCredentials::access_key("teststorage".to_string(), "fake_key");
		let client = ClientBuilder::new("teststorage".to_string(), credentials)
			.container_client("testcontainer".to_string());

		let storage = AzureBlobStorage {
			client,
			config: config.clone(),
		};

		assert_eq!(
			storage.url("file.txt"),
			"https://teststorage.blob.core.windows.net/testcontainer/static/file.txt"
		);
	}

	#[rstest]
	fn test_url_generation_with_custom_base() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_base_url("https://cdn.example.com".to_string());

		let credentials = StorageCredentials::access_key("teststorage".to_string(), "fake_key");
		let client = ClientBuilder::new("teststorage".to_string(), credentials)
			.container_client("testcontainer".to_string());

		let storage = AzureBlobStorage {
			client,
			config: config.clone(),
		};

		assert_eq!(storage.url("file.txt"), "https://cdn.example.com/file.txt");
	}
}
