//! Azure Blob Storage backend
//!
//! Provides storage backend for Azure Blob Storage

use super::Storage;
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use hmac::{Hmac, Mac};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Method;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::io;

type HmacSha256 = Hmac<Sha256>;

const AZURE_VERSION: &str = "2023-11-03";

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
	client: reqwest::Client,
	config: AzureBlobConfig,
}

impl AzureBlobStorage {
	/// Create a new Azure Blob Storage backend
	pub async fn new(config: AzureBlobConfig) -> io::Result<Self> {
		Self::validate_credentials(&config)?;
		let client = reqwest::Client::new();
		Ok(Self { client, config })
	}

	/// Validate Azure Blob Storage credentials.
	fn validate_credentials(config: &AzureBlobConfig) -> io::Result<()> {
		if config.account_key.is_some() || config.sas_token.is_some() {
			Ok(())
		} else {
			Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"Either account_key or sas_token must be provided",
			))
		}
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

	fn blob_url(&self, blob_name: &str) -> String {
		format!(
			"{}/{}",
			self.config.base_url.trim_end_matches('/'),
			utf8_percent_encode(blob_name, NON_ALPHANUMERIC)
		)
	}

	fn append_sas(&self, url: String) -> String {
		if self.config.account_key.is_some() {
			return url;
		}
		if let Some(sas_token) = &self.config.sas_token {
			let token = sas_token.trim_start_matches('?');
			if url.contains('?') {
				format!("{url}&{token}")
			} else {
				format!("{url}?{token}")
			}
		} else {
			url
		}
	}

	fn request_date() -> String {
		Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
	}

	fn canonicalized_headers(headers: &HeaderMap) -> String {
		let mut values = BTreeMap::new();
		for (name, value) in headers {
			let name = name.as_str().to_ascii_lowercase();
			if name.starts_with("x-ms-") {
				let value = value.to_str().unwrap_or_default();
				let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
				values.insert(name, value);
			}
		}
		values
			.into_iter()
			.map(|(name, value)| format!("{name}:{value}\n"))
			.collect::<String>()
	}

	fn canonicalized_resource(&self, url: &str) -> io::Result<String> {
		let parsed = reqwest::Url::parse(url).map_err(|err| io::Error::other(err.to_string()))?;
		let mut resource = format!("/{}{}", self.config.account_name, parsed.path());
		let mut query: BTreeMap<String, Vec<String>> = BTreeMap::new();
		for (key, value) in parsed.query_pairs() {
			query
				.entry(key.to_ascii_lowercase())
				.or_default()
				.push(value.to_string());
		}
		for (key, mut values) in query {
			values.sort();
			resource.push('\n');
			resource.push_str(&key);
			resource.push(':');
			resource.push_str(&values.join(","));
		}
		Ok(resource)
	}

	fn sign_request(
		&self,
		method: &Method,
		url: &str,
		headers: &HeaderMap,
		content_length: Option<usize>,
		content_type: Option<&str>,
	) -> io::Result<String> {
		let account_key = self.config.account_key.as_deref().ok_or_else(|| {
			io::Error::new(
				io::ErrorKind::InvalidInput,
				"Either account_key or sas_token must be provided",
			)
		})?;
		let content_length = match content_length {
			Some(0) | None => String::new(),
			Some(length) => length.to_string(),
		};
		let string_to_sign = [
			method.as_str().to_string(),
			String::new(),
			String::new(),
			content_length,
			String::new(),
			content_type.unwrap_or_default().to_string(),
			String::new(),
			String::new(),
			String::new(),
			String::new(),
			String::new(),
			String::new(),
			format!(
				"{}{}",
				Self::canonicalized_headers(headers),
				self.canonicalized_resource(url)?
			),
		]
		.join("\n");

		let key = STANDARD
			.decode(account_key)
			.map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err.to_string()))?;
		let mut mac = HmacSha256::new_from_slice(&key)
			.map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err.to_string()))?;
		mac.update(string_to_sign.as_bytes());
		Ok(STANDARD.encode(mac.finalize().into_bytes()))
	}

	async fn send(
		&self,
		method: Method,
		blob_name: String,
		body: Option<Vec<u8>>,
		content_type: Option<&str>,
		blob_request: bool,
	) -> io::Result<reqwest::Response> {
		let mut headers = HeaderMap::new();
		headers.insert(
			HeaderName::from_static("x-ms-date"),
			HeaderValue::from_str(&Self::request_date())
				.map_err(|err| io::Error::other(err.to_string()))?,
		);
		headers.insert(
			HeaderName::from_static("x-ms-version"),
			HeaderValue::from_static(AZURE_VERSION),
		);
		if blob_request {
			headers.insert(
				HeaderName::from_static("x-ms-blob-type"),
				HeaderValue::from_static("BlockBlob"),
			);
		}

		let content_length = body.as_ref().map(Vec::len);
		let url = self.append_sas(self.blob_url(&blob_name));
		let mut request = self
			.client
			.request(method.clone(), &url)
			.headers(headers.clone());
		if let Some(content_type) = content_type {
			request = request.header("content-type", content_type);
		}
		if let Some(length) = content_length {
			request = request.header("content-length", length);
		}
		if self.config.account_key.is_some() {
			let signature =
				self.sign_request(&method, &url, &headers, content_length, content_type)?;
			request = request.header(
				"authorization",
				format!("SharedKey {}:{signature}", self.config.account_name),
			);
		} else if self.config.sas_token.is_none() {
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"Either account_key or sas_token must be provided",
			));
		}
		if let Some(body) = body {
			request = request.body(body);
		}
		request
			.send()
			.await
			.map_err(|err| io::Error::other(err.to_string()))
	}
}

#[async_trait]
impl Storage for AzureBlobStorage {
	async fn save(&self, name: &str, content: &[u8]) -> io::Result<String> {
		let blob_name = self.get_full_blob_name(name);
		let response = self
			.send(
				Method::PUT,
				blob_name,
				Some(content.to_vec()),
				Some("application/octet-stream"),
				true,
			)
			.await
			.map_err(|e| io::Error::other(e.to_string()))?;
		if !response.status().is_success() {
			return Err(io::Error::other(format!(
				"Azure blob upload failed with status {}",
				response.status()
			)));
		}

		Ok(self.url(name))
	}

	fn exists(&self, name: &str) -> bool {
		let blob_name = self.get_full_blob_name(name);
		let client = self.client.clone();
		let storage = Self {
			client,
			config: self.config.clone(),
		};

		tokio::task::block_in_place(|| {
			tokio::runtime::Handle::current()
				.block_on(async {
					storage
						.send(Method::HEAD, blob_name, None, None, false)
						.await
				})
				.map(|response| response.status().is_success())
				.unwrap_or(false)
		})
	}

	async fn open(&self, name: &str) -> io::Result<Vec<u8>> {
		let blob_name = self.get_full_blob_name(name);

		let data = self
			.send(Method::GET, blob_name, None, None, false)
			.await
			.map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
		if !data.status().is_success() {
			return Err(io::Error::new(
				io::ErrorKind::NotFound,
				format!("Azure blob download failed with status {}", data.status()),
			));
		}

		data.bytes()
			.await
			.map(|bytes| bytes.to_vec())
			.map_err(|e| io::Error::other(e.to_string()))
	}

	async fn delete(&self, name: &str) -> io::Result<()> {
		let blob_name = self.get_full_blob_name(name);

		let response = self
			.send(Method::DELETE, blob_name, None, None, false)
			.await
			.map_err(|e| io::Error::other(e.to_string()))?;
		if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
			return Err(io::Error::other(format!(
				"Azure blob delete failed with status {}",
				response.status()
			)));
		}

		Ok(())
	}

	fn url(&self, name: &str) -> String {
		self.generate_url(name)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn storage(config: AzureBlobConfig) -> AzureBlobStorage {
		AzureBlobStorage {
			client: reqwest::Client::new(),
			config,
		}
	}

	#[test]
	fn test_azure_config_creation() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string());

		assert_eq!(config.account_name, "teststorage");
		assert_eq!(config.container, "testcontainer");
		assert_eq!(
			config.base_url,
			"https://teststorage.blob.core.windows.net/testcontainer"
		);
	}

	#[test]
	fn test_azure_config_with_account_key() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_account_key("ACCOUNT_KEY".to_string());

		assert_eq!(config.account_key, Some("ACCOUNT_KEY".to_string()));
	}

	#[test]
	fn test_azure_config_with_sas_token() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_sas_token("?sv=2021-01-01&sig=...".to_string());

		assert_eq!(config.sas_token, Some("?sv=2021-01-01&sig=...".to_string()));
	}

	#[test]
	fn test_azure_config_with_prefix() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_prefix("static".to_string());

		assert_eq!(config.prefix, Some("static".to_string()));
	}

	#[test]
	fn test_azure_config_with_base_url() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_base_url("https://cdn.example.com".to_string());

		assert_eq!(config.base_url, "https://cdn.example.com");
	}

	#[test]
	fn test_full_blob_name_generation() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string());

		let storage = storage(config);

		assert_eq!(storage.get_full_blob_name("file.txt"), "file.txt");
		assert_eq!(storage.get_full_blob_name("/file.txt"), "file.txt");
	}

	#[test]
	fn test_full_blob_name_with_prefix() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_prefix("static".to_string());

		let storage = storage(config);

		assert_eq!(storage.get_full_blob_name("file.txt"), "static/file.txt");
		assert_eq!(storage.get_full_blob_name("/file.txt"), "static/file.txt");
	}

	#[test]
	fn test_url_generation() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string());

		let storage = storage(config);

		assert_eq!(
			storage.url("file.txt"),
			"https://teststorage.blob.core.windows.net/testcontainer/file.txt"
		);
	}

	#[test]
	fn test_url_generation_with_prefix() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_prefix("static".to_string());

		let storage = storage(config);

		assert_eq!(
			storage.url("file.txt"),
			"https://teststorage.blob.core.windows.net/testcontainer/static/file.txt"
		);
	}

	#[test]
	fn test_url_generation_with_custom_base() {
		let config = AzureBlobConfig::new("teststorage".to_string(), "testcontainer".to_string())
			.with_base_url("https://cdn.example.com".to_string());

		let storage = storage(config);

		assert_eq!(storage.url("file.txt"), "https://cdn.example.com/file.txt");
	}
}
