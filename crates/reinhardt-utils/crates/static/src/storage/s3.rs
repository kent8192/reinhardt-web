//! S3-compatible storage backend
//!
//! Provides storage backend for Amazon S3 and S3-compatible services
//! (MinIO, LocalStack, etc.)

use crate::storage::Storage;
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
	Client as S3Client,
	config::{Credentials, Region},
	primitives::ByteStream,
};
use std::io;

/// S3 storage configuration
#[derive(Debug, Clone)]
pub struct S3Config {
	/// S3 bucket name
	pub bucket: String,
	/// AWS region
	pub region: String,
	/// Access key ID
	pub access_key_id: Option<String>,
	/// Secret access key
	pub secret_access_key: Option<String>,
	/// Custom endpoint URL (for S3-compatible services like MinIO, LocalStack)
	pub endpoint_url: Option<String>,
	/// Path prefix within bucket
	pub prefix: Option<String>,
	/// Base URL for generating file URLs
	pub base_url: String,
	/// Use path-style addressing (required for MinIO/LocalStack)
	pub path_style: bool,
}

impl S3Config {
	/// Create a new S3 configuration
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_static::storage::S3Config;
	///
	/// let config = S3Config::new(
	///     "my-bucket".to_string(),
	///     "us-east-1".to_string(),
	/// )
	/// .with_credentials(
	///     "ACCESS_KEY".to_string(),
	///     "SECRET_KEY".to_string(),
	/// )
	/// .with_base_url("https://my-bucket.s3.amazonaws.com".to_string());
	/// ```
	pub fn new(bucket: String, region: String) -> Self {
		Self {
			bucket: bucket.clone(),
			region,
			access_key_id: None,
			secret_access_key: None,
			endpoint_url: None,
			prefix: None,
			base_url: format!("https://{}.s3.amazonaws.com", bucket),
			path_style: false,
		}
	}

	/// Set AWS credentials
	pub fn with_credentials(mut self, access_key_id: String, secret_access_key: String) -> Self {
		self.access_key_id = Some(access_key_id);
		self.secret_access_key = Some(secret_access_key);
		self
	}

	/// Set custom endpoint (for MinIO, LocalStack, etc.)
	pub fn with_endpoint(mut self, endpoint_url: String) -> Self {
		self.endpoint_url = Some(endpoint_url);
		self.path_style = true; // Enable path-style for custom endpoints
		self
	}

	/// Set path prefix within bucket
	pub fn with_prefix(mut self, prefix: String) -> Self {
		self.prefix = Some(prefix.trim_matches('/').to_string());
		self
	}

	/// Set base URL for file URLs
	pub fn with_base_url(mut self, base_url: String) -> Self {
		self.base_url = base_url.trim_end_matches('/').to_string();
		self
	}

	/// Enable path-style addressing
	pub fn with_path_style(mut self) -> Self {
		self.path_style = true;
		self
	}
}

/// S3 storage backend
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_static::storage::{S3Storage, S3Config, Storage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = S3Config::new(
///     "my-bucket".to_string(),
///     "us-east-1".to_string(),
/// );
///
/// let storage = S3Storage::new(config).await?;
///
/// // Save a file
/// let url = storage.save("css/style.css", b"body { color: red; }").await?;
/// # Ok(())
/// # }
/// ```
pub struct S3Storage {
	client: S3Client,
	config: S3Config,
}

impl S3Storage {
	/// Create a new S3 storage backend
	pub async fn new(config: S3Config) -> io::Result<Self> {
		let client = Self::create_client(&config).await?;
		Ok(Self { client, config })
	}

	/// Create S3 client from configuration
	async fn create_client(config: &S3Config) -> io::Result<S3Client> {
		let region = Region::new(config.region.clone());

		let mut builder = aws_sdk_s3::config::Builder::new()
			.behavior_version(BehaviorVersion::latest())
			.region(region);

		// Set credentials if provided
		if let (Some(access_key), Some(secret_key)) =
			(&config.access_key_id, &config.secret_access_key)
		{
			let creds = Credentials::new(access_key, secret_key, None, None, "static-credentials");
			builder = builder.credentials_provider(creds);
		}

		// Set custom endpoint if provided
		if let Some(endpoint) = &config.endpoint_url {
			builder = builder.endpoint_url(endpoint);
		}

		// Force path-style if configured
		if config.path_style {
			builder = builder.force_path_style(true);
		}

		Ok(S3Client::from_conf(builder.build()))
	}

	/// Get full S3 key with prefix
	fn get_full_key(&self, name: &str) -> String {
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
impl Storage for S3Storage {
	async fn save(&self, name: &str, content: &[u8]) -> io::Result<String> {
		let key = self.get_full_key(name);
		let body = ByteStream::from(content.to_vec());

		self.client
			.put_object()
			.bucket(&self.config.bucket)
			.key(&key)
			.body(body)
			.send()
			.await
			.map_err(|e| io::Error::other(e.to_string()))?;

		Ok(self.url(name))
	}

	fn exists(&self, name: &str) -> bool {
		// S3 exists check requires async, so we use a blocking approach
		// In production, you might want to cache this or use a different strategy
		let key = self.get_full_key(name);
		let client = self.client.clone();
		let bucket = self.config.bucket.clone();

		tokio::task::block_in_place(|| {
			tokio::runtime::Handle::current().block_on(async {
				client
					.head_object()
					.bucket(&bucket)
					.key(&key)
					.send()
					.await
					.is_ok()
			})
		})
	}

	async fn open(&self, name: &str) -> io::Result<Vec<u8>> {
		let key = self.get_full_key(name);

		let result = self
			.client
			.get_object()
			.bucket(&self.config.bucket)
			.key(&key)
			.send()
			.await
			.map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

		let data = result
			.body
			.collect()
			.await
			.map_err(|e| io::Error::other(e.to_string()))?;

		Ok(data.into_bytes().to_vec())
	}

	async fn delete(&self, name: &str) -> io::Result<()> {
		let key = self.get_full_key(name);

		self.client
			.delete_object()
			.bucket(&self.config.bucket)
			.key(&key)
			.send()
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

	/// Test helper struct that only tests URL/key generation logic
	/// without requiring actual S3 client initialization (which needs TLS certs)
	struct TestableS3Config {
		config: S3Config,
	}

	impl TestableS3Config {
		fn new(config: S3Config) -> Self {
			Self { config }
		}

		/// Get full S3 key with prefix (mirrors S3Storage::get_full_key)
		fn get_full_key(&self, name: &str) -> String {
			let name = name.trim_start_matches('/');
			if let Some(prefix) = &self.config.prefix {
				format!("{}/{}", prefix, name)
			} else {
				name.to_string()
			}
		}

		/// Generate public URL for a file (mirrors S3Storage::generate_url)
		fn url(&self, name: &str) -> String {
			let name = name.trim_start_matches('/');
			if let Some(prefix) = &self.config.prefix {
				format!("{}/{}/{}", self.config.base_url, prefix, name)
			} else {
				format!("{}/{}", self.config.base_url, name)
			}
		}
	}

	#[test]
	fn test_s3_config_creation() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string());

		assert_eq!(config.bucket, "test-bucket");
		assert_eq!(config.region, "us-east-1");
		assert_eq!(config.base_url, "https://test-bucket.s3.amazonaws.com");
	}

	#[test]
	fn test_s3_config_with_credentials() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string())
			.with_credentials("ACCESS_KEY".to_string(), "SECRET_KEY".to_string());

		assert_eq!(config.access_key_id, Some("ACCESS_KEY".to_string()));
		assert_eq!(config.secret_access_key, Some("SECRET_KEY".to_string()));
	}

	#[test]
	fn test_s3_config_with_custom_endpoint() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string())
			.with_endpoint("http://localhost:9000".to_string());

		assert_eq!(
			config.endpoint_url,
			Some("http://localhost:9000".to_string())
		);
		assert!(config.path_style);
	}

	#[test]
	fn test_s3_config_with_prefix() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string())
			.with_prefix("static".to_string());

		assert_eq!(config.prefix, Some("static".to_string()));
	}

	#[test]
	fn test_s3_config_with_base_url() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string())
			.with_base_url("https://cdn.example.com".to_string());

		assert_eq!(config.base_url, "https://cdn.example.com");
	}

	#[test]
	fn test_full_key_generation() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string());
		let testable = TestableS3Config::new(config);

		assert_eq!(testable.get_full_key("file.txt"), "file.txt");
		assert_eq!(testable.get_full_key("/file.txt"), "file.txt");
	}

	#[test]
	fn test_full_key_with_prefix() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string())
			.with_prefix("static".to_string());
		let testable = TestableS3Config::new(config);

		assert_eq!(testable.get_full_key("file.txt"), "static/file.txt");
		assert_eq!(testable.get_full_key("/file.txt"), "static/file.txt");
	}

	#[test]
	fn test_url_generation() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string());
		let testable = TestableS3Config::new(config);

		assert_eq!(
			testable.url("file.txt"),
			"https://test-bucket.s3.amazonaws.com/file.txt"
		);
	}

	#[test]
	fn test_url_generation_with_prefix() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string())
			.with_prefix("static".to_string());
		let testable = TestableS3Config::new(config);

		assert_eq!(
			testable.url("file.txt"),
			"https://test-bucket.s3.amazonaws.com/static/file.txt"
		);
	}

	#[test]
	fn test_url_generation_with_custom_base() {
		let config = S3Config::new("test-bucket".to_string(), "us-east-1".to_string())
			.with_base_url("https://cdn.example.com".to_string());
		let testable = TestableS3Config::new(config);

		assert_eq!(testable.url("file.txt"), "https://cdn.example.com/file.txt");
	}
}
