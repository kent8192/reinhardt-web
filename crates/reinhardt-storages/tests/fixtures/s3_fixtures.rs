//! S3 storage backend test fixtures using LocalStack.

use reinhardt_storages::{StorageBackend, StorageConfig};
use rstest::fixture;
use std::sync::Arc;
use testcontainers::{
	clients,
	core::{ContainerPort, WaitFor},
	GenericImage, Image,
};

/// S3 test container wrapper.
pub struct S3TestContainer {
	/// LocalStack container
	#[allow(dead_code)]
	container: testcontainers::Container<GenericImage>,
	/// S3 endpoint URL
	pub endpoint: String,
	/// S3 bucket name
	pub bucket: String,
	/// AWS region
	pub region: String,
}

impl S3TestContainer {
	/// Create a new S3 test container with LocalStack.
	///
	/// This starts a LocalStack container and creates a test bucket.
	pub async fn new() -> Self {
		let docker = clients::Cli::default();

		// Use LocalStack image
		let image = GenericImage::new("localstack/localstack", "latest")
			.with_exposed_port(ContainerPort::new(4566))
			.with_wait_for(WaitFor::message_on_stdout("Ready."));

		let container = docker.run(image);
		let port = container.get_host_port_ipv4(4566);
		let endpoint = format!("http://localhost:{}", port);
		let bucket = "test-bucket".to_string();
		let region = "us-east-1".to_string();

		// Create bucket using AWS CLI or SDK
		// For now, we'll configure the client to create the bucket
		Self {
			container,
			endpoint,
			bucket,
			region,
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

	/// Create S3 storage backend from this container.
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

	/// Create S3 storage backend with prefix from this container.
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

/// S3 backend fixture using LocalStack container.
///
/// This fixture creates a new LocalStack container for each test.
#[fixture]
pub async fn s3_backend() -> Arc<dyn StorageBackend> {
	let container = S3TestContainer::new().await;
	container.create_backend().await
}

/// S3 backend fixture with path prefix.
#[fixture]
pub async fn s3_backend_with_prefix() -> Arc<dyn StorageBackend> {
	let container = S3TestContainer::new().await;
	container.create_backend_with_prefix("test-prefix").await
}

/// S3 test container fixture.
#[fixture]
pub async fn s3_container() -> S3TestContainer {
	S3TestContainer::new().await
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_s3_container_creation() {
		let container = S3TestContainer::new().await;
		assert!(container.endpoint().contains("localhost"));
		assert_eq!(container.bucket(), "test-bucket");
		assert_eq!(container.region(), "us-east-1");
	}

	#[tokio::test]
	async fn test_s3_backend_fixture() {
		let backend = s3_backend().await;

		// Test basic save and read
		backend
			.save("test.txt", b"Hello, S3!")
			.await
			.expect("Failed to save");

		let content = backend
			.open("test.txt")
			.await
			.expect("Failed to read");

		assert_eq!(content, b"Hello, S3!");

		// Cleanup
		backend.delete("test.txt").await.ok();
	}

	#[tokio::test]
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
