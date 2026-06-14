//! Test fixtures for storage backend tests.
//!
//! This module provides reusable fixtures for testing storage components.
//! All fixtures are designed to work with rstest and can be composed together.

#![allow(dead_code)]
#![allow(unreachable_pub)]
#![allow(deprecated)] // Fixtures still cover compatibility config until removal.

use chrono::{DateTime, Utc};
use reinhardt_storages::{StorageBackend, StorageConfig};
use rstest::fixture;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

struct AwsEnvGuard {
	originals: Vec<(&'static str, Option<String>)>,
}

impl AwsEnvGuard {
	fn test_credentials() -> Self {
		let guard = Self::capture(&[
			"AWS_ACCESS_KEY_ID",
			"AWS_SECRET_ACCESS_KEY",
			"AWS_SESSION_TOKEN",
		]);

		// SAFETY: S3 tests that create `MockS3Server` run under #[serial(s3)],
		// preventing concurrent environment access.
		unsafe {
			env::set_var("AWS_ACCESS_KEY_ID", "test");
			env::set_var("AWS_SECRET_ACCESS_KEY", "test");
			env::remove_var("AWS_SESSION_TOKEN");
		}

		guard
	}

	fn capture(keys: &[&'static str]) -> Self {
		Self {
			originals: keys.iter().map(|key| (*key, env::var(key).ok())).collect(),
		}
	}
}

impl Drop for AwsEnvGuard {
	fn drop(&mut self) {
		for (key, value) in self.originals.iter().rev() {
			// SAFETY: The same #[serial(s3)] guard that protects creation also
			// protects restoration when the mock server is dropped.
			unsafe {
				if let Some(value) = value {
					env::set_var(key, value);
				} else {
					env::remove_var(key);
				}
			}
		}
	}
}

// ============================================================================
// Test Data (inline to avoid super::utils dependency issues)
// ============================================================================

/// Test file structure.
#[derive(Debug, Clone)]
pub struct TestFile {
	pub name: String,
	pub content: Vec<u8>,
	pub size: usize,
}

impl TestFile {
	/// Create a new test file.
	pub fn new(name: String, content: Vec<u8>) -> Self {
		let size = content.len();
		Self {
			name,
			content,
			size,
		}
	}
}

/// Generate text content with specified number of lines.
fn generate_text_content(lines: usize) -> String {
	(0..lines)
		.map(|i| format!("Line {}: test content", i))
		.collect::<Vec<_>>()
		.join("\n")
}

/// Generate binary content containing all byte values.
fn generate_binary_content() -> Vec<u8> {
	(0u8..=255).collect::<Vec<_>>()
}

/// Generate unique file name with prefix.
pub fn generate_unique_name(prefix: &str) -> String {
	format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

/// Create test file with text content.
fn create_text_file(name: String, lines: usize) -> TestFile {
	let content = generate_text_content(lines);
	TestFile::new(name, content.into_bytes())
}

/// Create test file with binary content.
fn create_binary_file(name: String) -> TestFile {
	let content = generate_binary_content();
	TestFile::new(name, content)
}

// ============================================================================
// Common Fixtures
// ============================================================================

/// Empty file fixture (0 bytes).
#[fixture]
pub fn empty_file() -> TestFile {
	TestFile::new("empty.txt".to_string(), vec![])
}

/// Small file fixture (< 1KB).
#[fixture]
pub fn small_file() -> TestFile {
	create_text_file("small.txt".to_string(), 10)
}

/// Medium file fixture (around 1KB).
#[fixture]
pub fn medium_file() -> TestFile {
	create_text_file("medium.txt".to_string(), 100)
}

/// Large file fixture (around 100KB).
#[fixture]
pub fn large_file() -> TestFile {
	create_text_file("large.txt".to_string(), 10000)
}

/// Binary file fixture with all byte values.
#[fixture]
pub fn binary_file() -> TestFile {
	create_binary_file("binary.bin".to_string())
}

/// Collection of test files with varying sizes.
#[fixture]
pub fn test_files() -> Vec<TestFile> {
	vec![
		TestFile::new("empty.txt".to_string(), vec![]),
		create_text_file("small.txt".to_string(), 10),
		create_text_file("medium.txt".to_string(), 100),
		create_binary_file("binary.bin".to_string()),
	]
}

/// Unique file name fixture.
#[fixture]
pub fn unique_file_name() -> String {
	generate_unique_name("test")
}

// ============================================================================
// Local Storage Fixtures
// ============================================================================

/// Test directory wrapper for local storage.
pub struct LocalTestDir {
	temp_dir: TempDir,
	backend: Arc<dyn StorageBackend>,
}

impl LocalTestDir {
	/// Create a new test directory with storage backend.
	pub async fn new() -> Self {
		let temp_dir = TempDir::new().expect("Failed to create temp dir");
		let base_path = temp_dir.path().to_str().unwrap().to_string();

		let config = StorageConfig::Local(reinhardt_storages::config::LocalConfig { base_path });

		let backend = reinhardt_storages::create_storage(config)
			.await
			.expect("Failed to create local backend");

		Self { temp_dir, backend }
	}

	/// Get the storage backend.
	pub fn backend(&self) -> Arc<dyn StorageBackend> {
		Arc::clone(&self.backend)
	}

	/// Get the temp directory path.
	pub fn path(&self) -> &Path {
		self.temp_dir.path()
	}
}

/// Local storage backend fixture.
///
/// Note: This fixture uses `keep()` to prevent automatic cleanup when TempDir
/// goes out of scope. The temp directory will persist after the test.
/// This is acceptable for test code since OS cleans up temp directories.
#[fixture]
pub async fn local_backend() -> Arc<dyn StorageBackend> {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let base_path_str = temp_dir.path().to_str().unwrap().to_string();
	let _ = temp_dir.keep();

	let config = StorageConfig::Local(reinhardt_storages::config::LocalConfig {
		base_path: base_path_str,
	});

	reinhardt_storages::create_storage(config)
		.await
		.expect("Failed to create local backend")
}

/// Local test directory fixture.
#[fixture]
pub async fn local_temp_dir() -> LocalTestDir {
	LocalTestDir::new().await
}

// ============================================================================
// S3 Storage Fixtures (using wiremock Mock S3 Server)
// ============================================================================

/// An object stored in the mock S3 server.
struct StoredObject {
	content: Vec<u8>,
	last_modified: DateTime<Utc>,
}

/// In-memory state for the mock S3 server.
struct MockS3State {
	objects: HashMap<String, StoredObject>,
	buckets: HashSet<String>,
}

/// Stateful responder that mimics the AWS S3 REST API.
///
/// Routes requests based on HTTP method and URL path (path-style addressing):
/// - `PUT /{bucket}` -> CreateBucket
/// - `PUT /{bucket}/{key}` -> PutObject
/// - `GET /{bucket}/{key}` -> GetObject
/// - `HEAD /{bucket}/{key}` -> HeadObject
/// - `DELETE /{bucket}/{key}` -> DeleteObject
struct MockS3Responder {
	state: Arc<Mutex<MockS3State>>,
}

/// Parse a path-style S3 URL path into (bucket, optional key).
fn parse_s3_path(path: &str) -> (String, Option<String>) {
	let trimmed = path.trim_start_matches('/');
	if let Some(idx) = trimmed.find('/') {
		let bucket = trimmed[..idx].to_string();
		let key = trimmed[idx + 1..].to_string();
		(bucket, Some(key))
	} else {
		(trimmed.to_string(), None)
	}
}

impl Respond for MockS3Responder {
	fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
		let path = request.url.path();
		let (bucket, key) = parse_s3_path(&path);
		let method = request.method.as_str();

		match (method, key) {
			("PUT", None) => {
				// CreateBucket
				let mut state = self.state.lock().unwrap();
				state.buckets.insert(bucket.clone());
				ResponseTemplate::new(200).insert_header("Location", format!("/{}", bucket))
			}
			("PUT", Some(key)) => {
				// PutObject
				let full_key = format!("{}/{}", bucket, key);
				let mut state = self.state.lock().unwrap();
				state.objects.insert(
					full_key,
					StoredObject {
						content: request.body.clone(),
						last_modified: Utc::now(),
					},
				);
				ResponseTemplate::new(200)
					.insert_header("ETag", "\"mock-etag\"")
					.insert_header("Content-Length", "0")
			}
			("GET", Some(key)) => {
				// GetObject
				let full_key = format!("{}/{}", bucket, key);
				let state = self.state.lock().unwrap();
				match state.objects.get(&full_key) {
					Some(obj) => {
						let last_modified = obj.last_modified.format("%a, %d %b %Y %H:%M:%S GMT");
						ResponseTemplate::new(200)
							.set_body_bytes(obj.content.clone())
							.insert_header("Content-Length", obj.content.len().to_string())
							.insert_header("Content-Type", "application/octet-stream")
							.insert_header("ETag", "\"mock-etag\"")
							.insert_header("Last-Modified", last_modified.to_string())
							.insert_header("Accept-Ranges", "bytes")
					}
					None => {
						let xml = format!(
							"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
							<Error>\
								<Code>NoSuchKey</Code>\
								<Message>The specified key does not exist.</Message>\
								<Key>{}</Key>\
							</Error>",
							key
						);
						ResponseTemplate::new(404)
							.set_body_string(xml)
							.insert_header("Content-Type", "application/xml")
					}
				}
			}
			("HEAD", Some(key)) => {
				// HeadObject
				let full_key = format!("{}/{}", bucket, key);
				let state = self.state.lock().unwrap();
				match state.objects.get(&full_key) {
					Some(obj) => {
						let last_modified = obj.last_modified.format("%a, %d %b %Y %H:%M:%S GMT");
						ResponseTemplate::new(200)
							.insert_header("Content-Length", obj.content.len().to_string())
							.insert_header("Content-Type", "application/octet-stream")
							.insert_header("ETag", "\"mock-etag\"")
							.insert_header("Last-Modified", last_modified.to_string())
							.insert_header("Accept-Ranges", "bytes")
					}
					None => ResponseTemplate::new(404),
				}
			}
			("DELETE", Some(key)) => {
				// DeleteObject
				let full_key = format!("{}/{}", bucket, key);
				let mut state = self.state.lock().unwrap();
				state.objects.remove(&full_key);
				ResponseTemplate::new(204)
			}
			_ => ResponseTemplate::new(400).set_body_string("Bad Request"),
		}
	}
}

/// Mock S3 server using wiremock with AWS S3-standard responses.
///
/// Replaces the previous LocalStack-based `S3TestContainer`.
/// The actual `S3Storage` production code is exercised end-to-end through
/// this mock server, without requiring Docker.
pub struct MockS3Server {
	// Kept alive so wiremock continues serving HTTP requests for the test duration.
	#[allow(dead_code)]
	server: MockServer,
	// Kept alive so the MockS3Responder's Arc reference remains valid.
	#[allow(dead_code)]
	state: Arc<Mutex<MockS3State>>,
	endpoint: String,
	bucket: String,
	region: String,
	_env_guard: AwsEnvGuard,
}

impl MockS3Server {
	/// Create a new mock S3 server with a pre-created test bucket.
	pub async fn new() -> Self {
		let env_guard = AwsEnvGuard::test_credentials();
		let server = MockServer::start().await;
		let endpoint = server.uri();
		let bucket = "test-bucket".to_string();
		let region = "us-east-1".to_string();

		let mut buckets = HashSet::new();
		buckets.insert(bucket.clone());

		let state = Arc::new(Mutex::new(MockS3State {
			objects: HashMap::new(),
			buckets,
		}));

		let responder = MockS3Responder {
			state: Arc::clone(&state),
		};

		Mock::given(any())
			.respond_with(responder)
			.mount(&server)
			.await;

		Self {
			server,
			state,
			endpoint,
			bucket,
			region,
			_env_guard: env_guard,
		}
	}

	/// Get the S3 endpoint URL.
	pub fn endpoint(&self) -> &str {
		&self.endpoint
	}

	/// Get the bucket name.
	pub fn bucket(&self) -> &str {
		&self.bucket
	}

	/// Get the region.
	pub fn region(&self) -> &str {
		&self.region
	}

	/// Create S3 storage backend from this mock server.
	pub async fn create_backend(&self) -> Arc<dyn StorageBackend> {
		let config = StorageConfig::S3(reinhardt_storages::config::S3Config {
			bucket: self.bucket.clone(),
			region: Some(self.region.clone()),
			endpoint: Some(self.endpoint.clone()),
			prefix: None,
		});

		reinhardt_storages::create_storage(config)
			.await
			.expect("Failed to create S3 backend")
	}

	/// Create S3 storage backend with prefix from this mock server.
	pub async fn create_backend_with_prefix(&self, prefix: &str) -> Arc<dyn StorageBackend> {
		let config = StorageConfig::S3(reinhardt_storages::config::S3Config {
			bucket: self.bucket.clone(),
			region: Some(self.region.clone()),
			endpoint: Some(self.endpoint.clone()),
			prefix: Some(prefix.to_string()),
		});

		reinhardt_storages::create_storage(config)
			.await
			.expect("Failed to create S3 backend with prefix")
	}
}

/// Owns a `MockS3Server` alongside the backend it created, keeping the
/// mock HTTP server alive for the test lifetime without leaking memory.
pub struct S3BackendOwner {
	pub backend: Arc<dyn StorageBackend>,
	#[allow(dead_code)]
	// Dropped at end of test, cleaning up the wiremock listener.
	mock: MockS3Server,
}

impl std::ops::Deref for S3BackendOwner {
	type Target = dyn StorageBackend;

	fn deref(&self) -> &Self::Target {
		&*self.backend
	}
}

/// S3 backend fixture using wiremock mock server.
#[fixture]
pub async fn s3_backend() -> S3BackendOwner {
	let mock = MockS3Server::new().await;
	let backend = mock.create_backend().await;
	S3BackendOwner { backend, mock }
}

/// S3 backend fixture with path prefix.
#[fixture]
pub async fn s3_backend_with_prefix() -> S3BackendOwner {
	let mock = MockS3Server::new().await;
	let backend = mock.create_backend_with_prefix("test-prefix").await;
	S3BackendOwner { backend, mock }
}

/// Mock S3 server fixture.
#[fixture]
pub async fn s3_container() -> MockS3Server {
	MockS3Server::new().await
}

// ============================================================================
// GCS and Azure Emulator Fixtures
// ============================================================================

#[cfg(feature = "gcs")]
pub struct GcsFixture {
	pub _container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
	pub backend: Arc<dyn StorageBackend>,
	pub endpoint: String,
	pub bucket: String,
}

#[cfg(feature = "gcs")]
pub async fn gcs_fixture() -> GcsFixture {
	use testcontainers::core::{ContainerPort, WaitFor};
	use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

	let image = GenericImage::new("fsouza/fake-gcs-server", "1.52.2")
		.with_exposed_port(ContainerPort::Tcp(4443))
		.with_wait_for(WaitFor::seconds(1))
		.with_startup_timeout(std::time::Duration::from_secs(120))
		.with_cmd(vec!["-scheme", "http", "-port", "4443"]);

	let container = image
		.start()
		.await
		.expect("Failed to start fake-gcs-server");
	let port = container
		.get_host_port_ipv4(4443)
		.await
		.expect("Failed to get fake-gcs-server port");
	let endpoint = format!("http://127.0.0.1:{}", port);
	let bucket = generate_unique_name("reinhardt-gcs");

	let client = reqwest::Client::new();
	let mut last_error = None;
	for _ in 0..20 {
		match client
			.post(format!("{}/storage/v1/b?project=test-project", endpoint))
			.json(&serde_json::json!({ "name": bucket }))
			.send()
			.await
		{
			Ok(response) if response.status().is_success() => {
				last_error = None;
				break;
			}
			Ok(response) => {
				last_error = Some(response.status().to_string());
			}
			Err(error) => {
				last_error = Some(error.to_string());
			}
		}

		tokio::time::sleep(std::time::Duration::from_millis(250)).await;
	}
	assert!(
		last_error.is_none(),
		"fake GCS bucket creation failed: {}",
		last_error.unwrap_or_else(|| "unknown error".to_string())
	);

	let config = StorageConfig::Gcs(reinhardt_storages::config::GcsConfig {
		bucket: bucket.clone(),
		prefix: None,
		endpoint: Some(endpoint.clone()),
		service_account_json: None,
	});
	let backend = reinhardt_storages::create_storage(config)
		.await
		.expect("Failed to create GCS backend");

	GcsFixture {
		_container: container,
		backend,
		endpoint,
		bucket,
	}
}

#[cfg(feature = "azure")]
pub const AZURITE_ACCOUNT: &str = "devstoreaccount1";

#[cfg(feature = "azure")]
pub const AZURITE_KEY: &str =
	"Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";

#[cfg(feature = "azure")]
pub struct AzureFixture {
	pub _container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
	pub backend: Arc<dyn StorageBackend>,
	pub endpoint: String,
	pub container_name: String,
}

#[cfg(feature = "azure")]
pub async fn azure_fixture() -> AzureFixture {
	use testcontainers::core::{ContainerPort, WaitFor};
	use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

	// Wait for Azurite's readiness log line instead of a fixed delay: a flat
	// `WaitFor::seconds(1)` races the blob endpoint and intermittently yields
	// `NetworkError` when `create_storage` issues the container-create request
	// before Azurite is listening. Azurite 3.35.0 prints this on stdout once
	// the blob service is accepting connections.
	let image = GenericImage::new("mcr.microsoft.com/azure-storage/azurite", "3.35.0")
		.with_exposed_port(ContainerPort::Tcp(10000))
		.with_wait_for(WaitFor::message_on_stdout(
			"Azurite Blob service successfully listens",
		))
		.with_startup_timeout(std::time::Duration::from_secs(120))
		.with_cmd(vec!["azurite-blob", "--blobHost", "0.0.0.0"]);

	let container = image.start().await.expect("Failed to start Azurite");
	let port = container
		.get_host_port_ipv4(10000)
		.await
		.expect("Failed to get Azurite port");
	let endpoint = format!("http://127.0.0.1:{}/{}", port, AZURITE_ACCOUNT);
	let container_name = generate_unique_name("reinhardt").replace('_', "-");

	let config = StorageConfig::Azure(reinhardt_storages::config::AzureConfig {
		account: AZURITE_ACCOUNT.to_string(),
		container: container_name.clone(),
		prefix: None,
		endpoint: Some(endpoint.clone()),
		access_key: Some(AZURITE_KEY.into()),
		sas_token: None,
		connection_string: None,
	});
	let backend = reinhardt_storages::create_storage(config)
		.await
		.expect("Failed to create Azure backend");

	AzureFixture {
		_container: container,
		backend,
		endpoint,
		container_name,
	}
}

// ============================================================================
// Tests for Fixtures
// ============================================================================

#[cfg(test)]
mod tests {
	use super::*;
	use serial_test::serial;

	// Common fixture tests
	#[test]
	fn test_empty_file_fixture() {
		let file = empty_file();
		assert_eq!(file.name, "empty.txt");
		assert_eq!(file.size, 0);
		assert!(file.content.is_empty());
	}

	#[test]
	fn test_small_file_fixture() {
		let file = small_file();
		assert_eq!(file.name, "small.txt");
		assert!(file.size > 0);
		assert!(file.size < 1024);
	}

	#[test]
	fn test_medium_file_fixture() {
		let file = medium_file();
		assert_eq!(file.name, "medium.txt");
		assert!(file.size > 100);
	}

	#[test]
	fn test_large_file_fixture() {
		let file = large_file();
		assert_eq!(file.name, "large.txt");
		assert!(file.size > 10000);
	}

	#[test]
	fn test_binary_file_fixture() {
		let file = binary_file();
		assert_eq!(file.name, "binary.bin");
		assert_eq!(file.size, 256); // All byte values
	}

	#[test]
	fn test_test_files_fixture() {
		let files = test_files();
		assert_eq!(files.len(), 4);
		assert_eq!(files[0].name, "empty.txt");
		assert_eq!(files[1].name, "small.txt");
		assert_eq!(files[2].name, "medium.txt");
		assert_eq!(files[3].name, "binary.bin");
	}

	#[test]
	fn test_unique_file_name_fixture() {
		let name1 = unique_file_name();
		let name2 = unique_file_name();
		assert_ne!(name1, name2);
		assert!(name1.starts_with("test-"));
	}

	// Local fixture tests
	#[tokio::test]
	async fn test_local_test_dir_creation() {
		let test_dir = LocalTestDir::new().await;
		assert!(test_dir.path().exists());
		assert!(test_dir.path().is_dir());
	}

	#[tokio::test]
	async fn test_local_backend_fixture() {
		let backend = local_backend().await;
		// Just verify it creates successfully
		drop(backend);
	}

	#[tokio::test]
	async fn test_local_backend_save_and_read() {
		let backend = local_backend().await;

		backend
			.save("test.txt", b"Hello, world!")
			.await
			.expect("Failed to save");

		let content = backend.open("test.txt").await.expect("Failed to read");

		assert_eq!(content, b"Hello, world!");
	}

	// S3 fixture tests
	#[tokio::test]
	#[serial(s3)]
	async fn test_s3_container_creation() {
		let container = MockS3Server::new().await;
		assert!(
			container.endpoint().starts_with("http://127.0.0.1:"),
			"endpoint should bind to 127.0.0.1: {}",
			container.endpoint()
		);
		assert_eq!(container.bucket(), "test-bucket");
		assert_eq!(container.region(), "us-east-1");
	}

	#[tokio::test]
	#[serial(s3)]
	async fn test_s3_backend_fixture() {
		let backend = s3_backend().await;

		// Test basic save and read
		backend
			.save("test.txt", b"Hello, S3!")
			.await
			.expect("Failed to save");

		let content = backend.open("test.txt").await.expect("Failed to read");

		assert_eq!(content, b"Hello, S3!");

		// Cleanup
		backend.delete("test.txt").await.ok();
	}

	#[tokio::test]
	#[serial(s3)]
	async fn test_s3_backend_with_prefix_fixture() {
		let backend = s3_backend_with_prefix().await;

		// Test that prefix is applied
		let path = backend
			.save("test.txt", b"Hello, S3 with prefix!")
			.await
			.expect("Failed to save");

		assert!(path.contains("test-prefix"));
		assert!(path.contains("test.txt"));

		// Cleanup
		backend.delete("test.txt").await.ok();
	}
}
