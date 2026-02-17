//! Google Cloud Storage backend
//!
//! Provides storage backend for Google Cloud Storage

use super::Storage;
use async_trait::async_trait;
use cloud_storage::Client;
use std::io;
use std::sync::Arc;

/// Google Cloud Storage configuration
#[derive(Debug, Clone)]
pub struct GcsConfig {
	/// GCS bucket name
	pub bucket: String,
	/// GCS project ID
	pub project_id: String,
	/// Path prefix within bucket
	pub prefix: Option<String>,
	/// Base URL for generating file URLs
	pub base_url: String,
	/// Service account key JSON (optional, uses default credentials if not provided)
	pub service_account_key: Option<String>,
}

impl GcsConfig {
	/// Create a new GCS configuration
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_utils::staticfiles::storage::GcsConfig;
	///
	/// let config = GcsConfig::new(
	///     "my-bucket".to_string(),
	///     "my-project-id".to_string(),
	/// );
	/// ```
	pub fn new(bucket: String, project_id: String) -> Self {
		let base_url = format!("https://storage.googleapis.com/{}", bucket);
		Self {
			bucket,
			project_id,
			prefix: None,
			base_url,
			service_account_key: None,
		}
	}

	/// Set service account key JSON
	pub fn with_service_account_key(mut self, key_json: String) -> Self {
		self.service_account_key = Some(key_json);
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
}

/// Google Cloud Storage backend
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_utils::staticfiles::storage::{GcsStorage, GcsConfig, Storage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = GcsConfig::new(
///     "my-bucket".to_string(),
///     "my-project-id".to_string(),
/// );
///
/// let storage = GcsStorage::new(config).await?;
///
/// // Save a file
/// let url = storage.save("css/style.css", b"body { color: red; }").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct GcsStorage {
	client: Arc<Client>,
	config: GcsConfig,
}

impl GcsStorage {
	/// Create a new GCS storage backend
	pub async fn new(config: GcsConfig) -> io::Result<Self> {
		let client = Self::create_client(&config).await?;
		Ok(Self {
			client: Arc::new(client),
			config,
		})
	}

	/// Create GCS client from configuration
	///
	/// # Authentication Methods
	///
	/// 1. Service Account Key (if provided): Sets GOOGLE_APPLICATION_CREDENTIALS
	/// 2. Default credentials: Uses Application Default Credentials (ADC)
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Service account key JSON is invalid
	/// - Failed to create temporary credentials file
	/// - Client initialization fails
	async fn create_client(config: &GcsConfig) -> io::Result<Client> {
		let client = if let Some(key_json) = &config.service_account_key {
			// Validate JSON structure
			serde_json::from_str::<serde_json::Value>(key_json).map_err(|e| {
				io::Error::new(
					io::ErrorKind::InvalidData,
					format!("Invalid service account key JSON: {}", e),
				)
			})?;

			// Create temporary file for service account key
			// GCS client library reads credentials from GOOGLE_APPLICATION_CREDENTIALS env var
			let temp_dir = std::env::temp_dir();
			let key_file_path =
				temp_dir.join(format!("gcs-service-account-{}.json", std::process::id()));

			// Write service account key to temporary file
			std::fs::write(&key_file_path, key_json).map_err(|e| {
				io::Error::other(format!("Failed to write service account key file: {}", e))
			})?;

			// Set environment variable for GCS client
			// SAFETY: This is safe because we're setting a process-level environment variable
			// for GCS authentication. The variable is only used by the GCS client library
			// and won't affect other parts of the application.
			unsafe {
				std::env::set_var(
					"GOOGLE_APPLICATION_CREDENTIALS",
					key_file_path.to_string_lossy().to_string(),
				);
			}

			// Create client (will use the credentials file)
			let client = Client::default();

			// Clean up: Remove the temporary file after client creation
			// Note: The client has already read the credentials at this point
			let _ = std::fs::remove_file(key_file_path);

			client
		} else {
			// Use default authentication (Application Default Credentials)
			// This includes:
			// - GOOGLE_APPLICATION_CREDENTIALS environment variable
			// - gcloud CLI credentials
			// - Compute Engine/GKE metadata server
			Client::default()
		};

		Ok(client)
	}

	/// Get full object name with prefix
	fn get_full_object_name(&self, name: &str) -> String {
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
impl Storage for GcsStorage {
	async fn save(&self, name: &str, content: &[u8]) -> io::Result<String> {
		let object_name = self.get_full_object_name(name);

		self.client
			.object()
			.create(
				&self.config.bucket,
				content.to_vec(),
				&object_name,
				"application/octet-stream",
			)
			.await
			.map_err(|e| io::Error::other(e.to_string()))?;

		Ok(self.url(name))
	}

	fn exists(&self, name: &str) -> bool {
		let object_name = self.get_full_object_name(name);
		let bucket = self.config.bucket.clone();
		let client = self.client.clone();

		tokio::task::block_in_place(|| {
			tokio::runtime::Handle::current()
				.block_on(async { client.object().read(&bucket, &object_name).await.is_ok() })
		})
	}

	async fn open(&self, name: &str) -> io::Result<Vec<u8>> {
		let object_name = self.get_full_object_name(name);

		let data = self
			.client
			.object()
			.download(&self.config.bucket, &object_name)
			.await
			.map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

		Ok(data)
	}

	async fn delete(&self, name: &str) -> io::Result<()> {
		let object_name = self.get_full_object_name(name);

		self.client
			.object()
			.delete(&self.config.bucket, &object_name)
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
	fn test_gcs_config_creation() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string());

		assert_eq!(config.bucket, "test-bucket");
		assert_eq!(config.project_id, "test-project");
		assert_eq!(
			config.base_url,
			"https://storage.googleapis.com/test-bucket"
		);
	}

	#[rstest]
	fn test_gcs_config_with_service_account_key() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_service_account_key("{\"type\": \"service_account\"}".to_string());

		assert_eq!(
			config.service_account_key,
			Some("{\"type\": \"service_account\"}".to_string())
		);
	}

	#[rstest]
	fn test_gcs_config_with_prefix() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_prefix("static".to_string());

		assert_eq!(config.prefix, Some("static".to_string()));
	}

	#[rstest]
	fn test_gcs_config_with_base_url() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_base_url("https://cdn.example.com".to_string());

		assert_eq!(config.base_url, "https://cdn.example.com");
	}

	#[rstest]
	fn test_full_object_name_generation() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string());
		let client = Client::default();

		let storage = GcsStorage {
			client: Arc::new(client),
			config: config.clone(),
		};

		assert_eq!(storage.get_full_object_name("file.txt"), "file.txt");
		assert_eq!(storage.get_full_object_name("/file.txt"), "file.txt");
	}

	#[rstest]
	fn test_full_object_name_with_prefix() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_prefix("static".to_string());
		let client = Client::default();

		let storage = GcsStorage {
			client: Arc::new(client),
			config: config.clone(),
		};

		assert_eq!(storage.get_full_object_name("file.txt"), "static/file.txt");
		assert_eq!(storage.get_full_object_name("/file.txt"), "static/file.txt");
	}

	#[rstest]
	fn test_url_generation() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string());
		let client = Client::default();

		let storage = GcsStorage {
			client: Arc::new(client),
			config: config.clone(),
		};

		assert_eq!(
			storage.url("file.txt"),
			"https://storage.googleapis.com/test-bucket/file.txt"
		);
	}

	#[rstest]
	fn test_url_generation_with_prefix() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_prefix("static".to_string());
		let client = Client::default();

		let storage = GcsStorage {
			client: Arc::new(client),
			config: config.clone(),
		};

		assert_eq!(
			storage.url("file.txt"),
			"https://storage.googleapis.com/test-bucket/static/file.txt"
		);
	}

	#[rstest]
	fn test_url_generation_with_custom_base() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_base_url("https://cdn.example.com".to_string());
		let client = Client::default();

		let storage = GcsStorage {
			client: Arc::new(client),
			config: config.clone(),
		};

		assert_eq!(storage.url("file.txt"), "https://cdn.example.com/file.txt");
	}

	#[rstest]
	fn test_prefix_trimming() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_prefix("/static/".to_string());

		assert_eq!(config.prefix, Some("static".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_create_client_with_invalid_service_account_key() {
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_service_account_key("invalid json".to_string());

		let result = GcsStorage::new(config).await;
		assert!(
			result.is_err(),
			"Should fail with invalid service account key"
		);

		let error = result.unwrap_err();
		assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
		assert!(
			error
				.to_string()
				.contains("Invalid service account key JSON"),
			"Error message should indicate JSON validation failure"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_create_client_with_valid_service_account_key_structure() {
		// Valid JSON structure (even if not a real service account key)
		let fake_key_json = r#"{
            "type": "service_account",
            "project_id": "test-project",
            "private_key_id": "test-key-id",
            "private_key": "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n",
            "client_email": "test@test-project.iam.gserviceaccount.com",
            "client_id": "123456789",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#;

		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string())
			.with_service_account_key(fake_key_json.to_string());

		// This will create a GcsStorage instance
		// In a real test environment, this would fail at the point of actual API calls
		// because the credentials are fake
		let result = GcsStorage::new(config).await;

		// The client creation should succeed (validation passes)
		// Actual GCS operations would fail with authentication errors
		assert!(
			result.is_ok(),
			"Client creation should succeed with valid JSON structure"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_create_client_without_service_account_key() {
		// Test default authentication (will use ADC if available)
		let config = GcsConfig::new("test-bucket".to_string(), "test-project".to_string());

		// This should succeed (uses default credentials)
		let result = GcsStorage::new(config).await;
		assert!(
			result.is_ok(),
			"Client creation with default credentials should succeed"
		);
	}

	#[rstest]
	fn test_service_account_key_validation() {
		// Test that JSON validation works correctly
		let valid_json = r#"{"type": "service_account"}"#;
		let invalid_json = "not json";

		assert!(serde_json::from_str::<serde_json::Value>(valid_json).is_ok());
		assert!(serde_json::from_str::<serde_json::Value>(invalid_json).is_err());
	}
}
