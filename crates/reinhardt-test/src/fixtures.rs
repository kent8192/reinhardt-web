use rstest::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// File-based lock guard for inter-process synchronization
///
/// This guard ensures that only one test process can execute the Redis Cluster
/// fixture at a time, even when using cargo-nextest which runs tests in separate processes.
///
/// The lock is acquired by creating an exclusive file lock on a temporary file.
/// When the guard is dropped, the lock is automatically released.
#[cfg(feature = "testcontainers")]
pub struct FileLockGuard {
	#[allow(dead_code)]
	file: std::fs::File,
}

#[cfg(feature = "testcontainers")]
impl FileLockGuard {
	/// Acquire exclusive file lock (blocks until available)
	pub fn acquire() -> std::io::Result<Self> {
		use fs2::FileExt;
		use std::fs::OpenOptions;

		let lock_path = std::env::temp_dir().join("reinhardt_redis_cluster.lock");

		let file = OpenOptions::new()
			.create(true)
			.truncate(true)
			.write(true)
			.open(&lock_path)?;

		// Acquire exclusive lock (blocking until available)
		file.lock_exclusive()?;
		eprintln!(
			"🔒 Acquired Redis Cluster file lock (PID: {})",
			std::process::id()
		);

		Ok(Self { file })
	}
}

#[cfg(feature = "testcontainers")]
impl Drop for FileLockGuard {
	fn drop(&mut self) {
		// Unlock explicitly (also happens automatically when file is closed)
		// Ignore result as we can't handle errors in Drop
		#[allow(unused_imports)]
		{
			use fs2::FileExt;
			let _ = self.file.unlock();
		}
		eprintln!(
			"🔓 Released Redis Cluster file lock (PID: {})",
			std::process::id()
		);
	}
}

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
	#[error("Fixture not found: {0}")]
	NotFound(String),
	#[error("Load error: {0}")]
	Load(String),
	#[error("Parse error: {0}")]
	Parse(String),
}

pub type FixtureResult<T> = Result<T, FixtureError>;

/// Fixture data loader
pub struct FixtureLoader {
	fixtures: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl FixtureLoader {
	/// Create a new fixture loader
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::FixtureLoader;
	///
	/// let loader = FixtureLoader::new();
	// Loader is ready to load fixtures
	/// ```
	pub fn new() -> Self {
		Self {
			fixtures: Arc::new(RwLock::new(HashMap::new())),
		}
	}
	/// Load fixture from JSON string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::FixtureLoader;
	///
	/// # tokio_test::block_on(async {
	/// let loader = FixtureLoader::new();
	/// let json = r#"{"id": 1, "name": "Test"}"#;
	/// loader.load_from_json("test".to_string(), json).await.unwrap();
	/// assert!(loader.exists("test").await);
	/// # });
	/// ```
	pub async fn load_from_json(&self, name: String, json: &str) -> FixtureResult<()> {
		let value: serde_json::Value =
			serde_json::from_str(json).map_err(|e| FixtureError::Parse(e.to_string()))?;

		self.fixtures.write().await.insert(name, value);
		Ok(())
	}
	/// Load fixture data
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::FixtureLoader;
	/// use serde::Deserialize;
	///
	/// #[derive(Deserialize)]
	/// struct User {
	///     id: i32,
	///     name: String,
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let loader = FixtureLoader::new();
	/// let json = r#"{"id": 1, "name": "Alice"}"#;
	/// loader.load_from_json("user".to_string(), json).await.unwrap();
	/// let user: User = loader.load("user").await.unwrap();
	/// assert_eq!(user.id, 1);
	/// assert_eq!(user.name, "Alice");
	/// # });
	/// ```
	pub async fn load<T: for<'de> Deserialize<'de>>(&self, name: &str) -> FixtureResult<T> {
		let fixtures = self.fixtures.read().await;
		let value = fixtures
			.get(name)
			.ok_or_else(|| FixtureError::NotFound(name.to_string()))?;

		serde_json::from_value(value.clone()).map_err(|e| FixtureError::Parse(e.to_string()))
	}
	/// Get raw fixture value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::FixtureLoader;
	///
	/// # tokio_test::block_on(async {
	/// let loader = FixtureLoader::new();
	/// let json = r#"{"status": "active"}"#;
	/// loader.load_from_json("config".to_string(), json).await.unwrap();
	/// let value = loader.get("config").await.unwrap();
	/// assert!(value.is_object());
	/// # });
	/// ```
	pub async fn get(&self, name: &str) -> FixtureResult<serde_json::Value> {
		let fixtures = self.fixtures.read().await;
		fixtures
			.get(name)
			.cloned()
			.ok_or_else(|| FixtureError::NotFound(name.to_string()))
	}
	/// Check if fixture exists
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::FixtureLoader;
	///
	/// # tokio_test::block_on(async {
	/// let loader = FixtureLoader::new();
	/// assert!(!loader.exists("missing").await);
	/// loader.load_from_json("test".to_string(), "{}").await.unwrap();
	/// assert!(loader.exists("test").await);
	/// # });
	/// ```
	pub async fn exists(&self, name: &str) -> bool {
		self.fixtures.read().await.contains_key(name)
	}
	/// Clear all fixtures
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::FixtureLoader;
	///
	/// # tokio_test::block_on(async {
	/// let loader = FixtureLoader::new();
	/// loader.load_from_json("test".to_string(), "{}").await.unwrap();
	/// assert_eq!(loader.list().await.len(), 1);
	/// loader.clear().await;
	/// assert_eq!(loader.list().await.len(), 0);
	/// # });
	/// ```
	pub async fn clear(&self) {
		self.fixtures.write().await.clear();
	}
	/// List all fixture names
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::FixtureLoader;
	///
	/// # tokio_test::block_on(async {
	/// let loader = FixtureLoader::new();
	/// loader.load_from_json("test1".to_string(), "{}").await.unwrap();
	/// loader.load_from_json("test2".to_string(), "{}").await.unwrap();
	/// let names = loader.list().await;
	/// assert_eq!(names.len(), 2);
	/// assert!(names.contains(&"test1".to_string()));
	/// # });
	/// ```
	pub async fn list(&self) -> Vec<String> {
		self.fixtures.read().await.keys().cloned().collect()
	}
}

impl Default for FixtureLoader {
	fn default() -> Self {
		Self::new()
	}
}

/// Factory trait for creating test data
pub trait Factory<T>: Send + Sync {
	fn build(&self) -> T;
	fn build_batch(&self, count: usize) -> Vec<T> {
		(0..count).map(|_| self.build()).collect()
	}
}

/// Simple factory builder
pub struct FactoryBuilder<T, F>
where
	F: Fn() -> T + Send + Sync,
{
	builder: F,
	_phantom: std::marker::PhantomData<T>,
}

/// Generate a random test key using UUID
///
/// # Examples
///
/// ```
/// use reinhardt_test::fixtures::random_test_key;
///
/// let key = random_test_key();
/// assert!(key.starts_with("test_key_"));
/// ```
pub fn random_test_key() -> String {
	use uuid::Uuid;
	format!("test_key_{}", Uuid::new_v4().simple())
}

/// Generate test configuration data with timestamp
///
/// # Examples
///
/// ```
/// use reinhardt_test::fixtures::test_config_value;
///
/// let value = test_config_value("my_value");
/// assert_eq!(value["value"], "my_value");
/// ```
pub fn test_config_value(value: &str) -> serde_json::Value {
	serde_json::json!({
		"value": value,
		"timestamp": chrono::Utc::now().to_rfc3339(),
	})
}

impl<T, F> FactoryBuilder<T, F>
where
	F: Fn() -> T + Send + Sync,
{
	/// Create a new factory builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::fixtures::{FactoryBuilder, Factory};
	///
	/// #[derive(Debug, PartialEq)]
	/// struct TestData { id: i32 }
	///
	/// let factory = FactoryBuilder::new(|| TestData { id: 42 });
	/// let item = factory.build();
	/// assert_eq!(item.id, 42);
	/// ```
	pub fn new(builder: F) -> Self {
		Self {
			builder,
			_phantom: std::marker::PhantomData,
		}
	}
}

impl<T, F> Factory<T> for FactoryBuilder<T, F>
where
	F: Fn() -> T + Send + Sync,
	T: Send + Sync,
{
	fn build(&self) -> T {
		(self.builder)()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde::Serialize;

	#[derive(Debug, Serialize, Deserialize, PartialEq)]
	struct TestData {
		id: i32,
		name: String,
	}

	#[tokio::test]
	async fn test_fixture_loader() {
		let loader = FixtureLoader::new();
		let json = r#"{"id": 1, "name": "Test"}"#;

		loader
			.load_from_json("test".to_string(), json)
			.await
			.unwrap();

		let data: TestData = loader.load("test").await.unwrap();
		assert_eq!(data.id, 1);
		assert_eq!(data.name, "Test");
	}

	#[tokio::test]
	async fn test_fixture_not_found() {
		let loader = FixtureLoader::new();
		let result: FixtureResult<TestData> = loader.load("missing").await;
		assert!(result.is_err());
	}

	#[test]
	fn test_factory_builder() {
		let factory = FactoryBuilder::new(|| TestData {
			id: 1,
			name: "Test".to_string(),
		});

		let data = factory.build();
		assert_eq!(data.id, 1);

		let batch = factory.build_batch(3);
		assert_eq!(batch.len(), 3);
	}
}

// ============================================================================
// rstest integration: Fixtures for common test resources
// ============================================================================

/// Fixture providing a FixtureLoader instance
///
/// Use this fixture in tests that need to load JSON fixture data.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::fixture_loader;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_fixtures(fixture_loader: reinhardt_test::fixtures::FixtureLoader) {
///     fixture_loader.load_from_json("test".to_string(), r#"{"id": 1}"#).await.unwrap();
///     // ...
/// }
/// ```
#[fixture]
pub fn fixture_loader() -> FixtureLoader {
	FixtureLoader::new()
}

/// Fixture providing an APIClient instance
///
/// Use this fixture in tests that need to make test HTTP requests.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::api_client;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_api_request(api_client: reinhardt_test::client::APIClient) {
///     // Make requests with client
/// }
/// ```
#[fixture]
pub fn api_client() -> crate::client::APIClient {
	crate::client::APIClient::new()
}

/// Fixture providing a temporary directory that is automatically cleaned up
///
/// # Examples
///
/// ```rust
/// use reinhardt_test::fixtures::temp_dir;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_temp_dir(temp_dir: tempfile::TempDir) {
///     let path = temp_dir.path();
///     std::fs::write(path.join("test.txt"), "data").unwrap();
///     // temp_dir is automatically cleaned up when test ends
/// }
/// ```
#[fixture]
pub fn temp_dir() -> tempfile::TempDir {
	tempfile::tempdir().expect("Failed to create temporary directory")
}

// ============================================================================
// TestContainers fixtures (optional, requires "testcontainers" feature)
// ============================================================================

#[cfg(feature = "testcontainers")]
use testcontainers::{ContainerAsync, runners::AsyncRunner};
#[cfg(feature = "testcontainers")]
use testcontainers_modules::{postgres::Postgres, redis::Redis};

/// Fixture providing a PostgreSQL TestContainer
///
/// Returns a tuple of (container, connection_url).
/// The container is automatically cleaned up when the test ends.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::postgres_container;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_postgres(#[future] postgres_container: (ContainerAsync<Postgres>, String)) {
///     let (_container, url) = postgres_container.await;
///     // Use PostgreSQL database at `url`
/// }
/// ```
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn postgres_container() -> (ContainerAsync<Postgres>, String) {
	let container = Postgres::default()
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = container
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

	(container, url)
}

/// Fixture providing a Redis TestContainer
///
/// Returns a tuple of (container, connection_url).
/// The container is automatically cleaned up when the test ends.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::redis_container;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_redis(#[future] redis_container: (ContainerAsync<Redis>, String)) {
///     let (_container, url) = redis_container.await;
///     // Use Redis at `url`
/// }
/// ```
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn redis_container() -> (ContainerAsync<Redis>, String) {
	let container = Redis::default()
		.start()
		.await
		.expect("Failed to start Redis container");

	let port = container
		.get_host_port_ipv4(6379)
		.await
		.expect("Failed to get Redis port");

	let url = format!("redis://localhost:{}", port);

	(container, url)
}

// ============================================================================
// Composable Redis Cluster Fixtures (using rstest fixture composition)
// ============================================================================

/// Redis Cluster container information
///
/// This struct holds the container instance, connection URLs, and the file lock.
/// The lock is held for the lifetime of the container to ensure exclusive access.
#[cfg(feature = "testcontainers")]
pub struct RedisClusterContainer {
	/// The running container instance
	pub container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
	/// Connection URLs for all 6 cluster nodes (ports 17000-17005)
	pub urls: Vec<String>,
	/// Container host (usually "localhost")
	pub host: String,
	/// File lock guard (held for the container's lifetime)
	_lock: FileLockGuard,
}

/// Level 1: Acquire Redis Cluster file lock
///
/// This is the foundation fixture that ensures only one test process can
/// execute Redis Cluster tests at a time, even when using cargo-nextest
/// which runs tests in separate processes.
///
/// # Returns
///
/// - `FileLockGuard`: File lock that will be automatically released when dropped
///
/// # Panics
///
/// Panics if the file lock cannot be acquired.
#[cfg(feature = "testcontainers")]
#[fixture]
pub fn redis_cluster_lock() -> FileLockGuard {
	FileLockGuard::acquire().expect("Failed to acquire Redis Cluster lock")
}

/// Level 2: Cleanup stale Redis Cluster containers
///
/// This fixture depends on `redis_cluster_lock` and performs cleanup
/// after acquiring the lock.
///
/// # Arguments
///
/// - `_redis_cluster_lock`: File lock from previous fixture (ownership transferred)
///
/// # Returns
///
/// - `FileLockGuard`: The lock is passed to the next fixture
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn redis_cluster_cleanup(_redis_cluster_lock: FileLockGuard) -> FileLockGuard {
	use std::process::Command;

	eprintln!("🧹 Cleaning up stale Redis Cluster containers...");

	// Find all containers based on neohq/redis-cluster image
	let output = Command::new("podman")
		.args(["ps", "-aq", "--filter", "ancestor=neohq/redis-cluster"])
		.output();

	if let Ok(output) = output {
		let container_ids = String::from_utf8_lossy(&output.stdout);
		let ids: Vec<&str> = container_ids.lines().collect();

		if !ids.is_empty() {
			eprintln!("  Found {} stale container(s)", ids.len());

			for id in &ids {
				let id = id.trim();
				if !id.is_empty() {
					let result = Command::new("podman").args(["rm", "-f", id]).output();

					match result {
						Ok(_) => eprintln!("  ✓ Removed container: {}", id),
						Err(e) => eprintln!("  ✗ Failed to remove container {}: {}", id, e),
					}
				}
			}

			eprintln!("  Cleanup complete");
		} else {
			eprintln!("  No stale containers found");
		}
	} else {
		eprintln!("  Warning: Failed to query podman containers");
	}

	eprintln!("✓ Redis Cluster cleanup complete");
	_redis_cluster_lock
}

/// Level 3: Wait for Redis Cluster ports to become available
///
/// This fixture depends on `redis_cluster_cleanup` and waits for all
/// required ports (17000-17005) to become available after cleanup.
///
/// # Arguments
///
/// - `redis_cluster_cleanup`: File lock from previous fixture (async, requires #[future])
///
/// # Returns
///
/// - `Result<FileLockGuard, Box<dyn std::error::Error>>`: The lock or an error
///
/// # Errors
///
/// Returns error if ports don't become available within the timeout period.
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn redis_cluster_ports_ready(
	#[future] redis_cluster_cleanup: FileLockGuard,
) -> Result<FileLockGuard, Box<dyn std::error::Error>> {
	use std::process::Command;
	use std::time::Duration;
	use tokio::time::sleep;

	let lock = redis_cluster_cleanup.await;

	const CLUSTER_PORTS: &[u16] = &[17000, 17001, 17002, 17003, 17004, 17005];
	const MAX_ATTEMPTS: u32 = 20;
	const RETRY_INTERVAL_MS: u64 = 500;

	eprintln!(
		"  Waiting for ports {:?} to become available...",
		CLUSTER_PORTS
	);

	for attempt in 1..=MAX_ATTEMPTS {
		// Use lsof to check if any process is listening on the ports
		let mut all_available = true;

		for &port in CLUSTER_PORTS {
			// Check if any process is listening on this port
			let output = Command::new("lsof")
				.args(["-i", &format!("TCP:{}", port), "-s", "TCP:LISTEN"])
				.output();

			if let Ok(output) = output {
				// If lsof returns any output, the port is in use
				if !output.stdout.is_empty() {
					all_available = false;
					eprintln!(
						"  ⏳ Port {} still in use (attempt {}/{})",
						port, attempt, MAX_ATTEMPTS
					);
					break;
				}
			}
		}

		if all_available {
			eprintln!(
				"  ✓ All ports available (attempt {}/{})",
				attempt, MAX_ATTEMPTS
			);
			// Add a small additional delay to ensure port release is complete
			sleep(Duration::from_millis(200)).await;
			eprintln!("✓ Redis Cluster ports are ready");
			return Ok(lock);
		}

		if attempt < MAX_ATTEMPTS {
			sleep(Duration::from_millis(RETRY_INTERVAL_MS)).await;
		}
	}

	Err(format!(
		"Ports {:?} not available after {} attempts ({} seconds total)",
		CLUSTER_PORTS,
		MAX_ATTEMPTS,
		(MAX_ATTEMPTS as u64 * RETRY_INTERVAL_MS) / 1000
	)
	.into())
}

/// Level 4: Start Redis Cluster container
///
/// This fixture depends on `redis_cluster_ports_ready` and starts the
/// Redis Cluster container after ports become available.
///
/// # Arguments
///
/// - `redis_cluster_ports_ready`: Result from previous fixture (async, requires #[future])
///
/// # Returns
///
/// - `RedisClusterContainer`: Container info with lock, URLs, and host
///
/// # Panics
///
/// Panics if:
/// - Ports are not available
/// - Container fails to start
/// - Host cannot be retrieved
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn redis_cluster_container(
	#[future] redis_cluster_ports_ready: Result<FileLockGuard, Box<dyn std::error::Error>>,
) -> RedisClusterContainer {
	use testcontainers::{GenericImage, ImageExt, core::ContainerPort, runners::AsyncRunner};

	let lock = redis_cluster_ports_ready
		.await
		.expect("Redis Cluster ports not ready");

	// Use neohq/redis-cluster image - ARM64 native build of grokzen/redis-cluster
	// Benefits: Native ARM64 performance, faster startup (~30s vs 120s+ with QEMU emulation)
	// Note: We don't use .with_wait_for() here because the cluster doesn't output
	// a simple "Ready to accept connections" message. Instead, we rely on inline logic
	// that polls CLUSTER INFO for cluster_state:ok
	//
	// IMPORTANT: Redis Cluster requires FIXED port mapping where host ports match container ports.
	// Reason: Cluster nodes advertise their ports via CLUSTER SLOTS, and clients connect to those advertised ports.
	// Dynamic port mapping (7000→38483) breaks this, causing "ClusterConnectionNotFound".
	//
	// Port Configuration:
	// - INITIAL_PORT=17000: Start cluster on ports 17000-17005 (avoids macOS port 7000 conflict with ControlCenter)
	// - Fixed mappings: 17000:17000, 17001:17001, ..., 17005:17005
	// - Tests use #[serial(redis_cluster)] to prevent port conflicts between parallel test runs
	let cluster = GenericImage::new("neohq/redis-cluster", "latest")
		// Map each cluster port to the SAME port on host (required for Redis Cluster)
		.with_mapped_port(17000, ContainerPort::Tcp(17000))
		.with_mapped_port(17001, ContainerPort::Tcp(17001))
		.with_mapped_port(17002, ContainerPort::Tcp(17002))
		.with_mapped_port(17003, ContainerPort::Tcp(17003))
		.with_mapped_port(17004, ContainerPort::Tcp(17004))
		.with_mapped_port(17005, ContainerPort::Tcp(17005))
		.with_env_var("IP", "0.0.0.0")  // Required for Mac: enables proper cluster discovery
		.with_env_var("REDIS_TLS_ENABLED", "no")
		.with_env_var("INITIAL_PORT", "17000")  // Start cluster at port 17000 instead of default 7000
		.start()
		.await
		.expect("Failed to start Redis Cluster container");

	eprintln!("✓ Redis Cluster container started (neohq/redis-cluster:latest, ports 17000-17005)");

	// Get host (localhost for testcontainers)
	let host_str = cluster
		.get_host()
		.await
		.expect("Failed to get container host")
		.to_string();

	// Build URLs for all 6 cluster nodes using ports 17000-17005
	// We use fixed port mapping, so host ports match container ports exactly
	let mut urls = Vec::new();
	for port in 17000..=17005 {
		let url = format!("redis://{}:{}", host_str, port);
		urls.push(url.clone());
		eprintln!("  Node {}: {}", port - 17000, url);
	}

	RedisClusterContainer {
		container: cluster,
		urls,
		host: host_str,
		_lock: lock, // Lock is held for the container's lifetime
	}
}

#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn redis_cluster_fixture() -> (
	testcontainers::ContainerAsync<testcontainers::GenericImage>,
	Vec<String>,
) {
	use testcontainers::{GenericImage, ImageExt, core::ContainerPort, runners::AsyncRunner};

	// Clean up any stale containers from previous test runs before starting new one
	{
		use std::process::Command;

		eprintln!("🧹 Cleaning up stale Redis Cluster containers...");

		let output = Command::new("podman")
			.args(["ps", "-aq", "--filter", "ancestor=neohq/redis-cluster"])
			.output();

		if let Ok(output) = output {
			let container_ids = String::from_utf8_lossy(&output.stdout);
			let ids: Vec<&str> = container_ids.lines().collect();

			if !ids.is_empty() {
				eprintln!("  Found {} stale container(s)", ids.len());
				for id in &ids {
					let id = id.trim();
					if !id.is_empty() {
						let result = Command::new("podman").args(["rm", "-f", id]).output();
						match result {
							Ok(_) => eprintln!("  ✓ Removed container: {}", id),
							Err(e) => eprintln!("  ✗ Failed to remove container {}: {}", id, e),
						}
					}
				}
				eprintln!("  Cleanup complete");
			} else {
				eprintln!("  No stale containers found");
			}
		} else {
			eprintln!("  Warning: Failed to query podman containers");
		}
	}

	// Use neohq/redis-cluster image - ARM64 native build of grokzen/redis-cluster
	// Benefits: Native ARM64 performance, faster startup (~30s vs 120s+ with QEMU emulation)
	// Note: We don't use .with_wait_for() here because the cluster doesn't output
	// a simple "Ready to accept connections" message. Instead, we rely on inline logic
	// that polls CLUSTER INFO for cluster_state:ok
	//
	// IMPORTANT: Redis Cluster requires FIXED port mapping where host ports match container ports.
	// Reason: Cluster nodes advertise their ports via CLUSTER SLOTS, and clients connect to those advertised ports.
	// Dynamic port mapping (7000→38483) breaks this, causing "ClusterConnectionNotFound".
	//
	// Port Configuration:
	// - INITIAL_PORT=17000: Start cluster on ports 17000-17005 (avoids macOS port 7000 conflict with ControlCenter)
	// - Fixed mappings: 17000:17000, 17001:17001, ..., 17005:17005
	// - Tests use #[serial(redis_cluster)] to prevent port conflicts between parallel test runs
	let cluster = GenericImage::new("neohq/redis-cluster", "latest")
        // Map each cluster port to the SAME port on host (required for Redis Cluster)
        .with_mapped_port(17000, ContainerPort::Tcp(17000))
        .with_mapped_port(17001, ContainerPort::Tcp(17001))
        .with_mapped_port(17002, ContainerPort::Tcp(17002))
        .with_mapped_port(17003, ContainerPort::Tcp(17003))
        .with_mapped_port(17004, ContainerPort::Tcp(17004))
        .with_mapped_port(17005, ContainerPort::Tcp(17005))
        .with_env_var("IP", "0.0.0.0")  // Required for Mac: enables proper cluster discovery
        .with_env_var("REDIS_TLS_ENABLED", "no")
        .with_env_var("INITIAL_PORT", "17000")  // Start cluster at port 17000 instead of default 7000
        .start()
        .await
        .expect("Failed to start Redis Cluster container");

	eprintln!("Redis Cluster container started (neohq/redis-cluster:latest, ports 17000-17005)");

	// Get host (localhost for testcontainers)
	let host_str = cluster
		.get_host()
		.await
		.expect("Failed to get container host");
	let host_str = host_str.to_string();

	// Build URLs for all 6 cluster nodes using ports 17000-17005
	// We use fixed port mapping, so host ports match container ports exactly
	let mut urls = Vec::new();
	for port in 17000..=17005 {
		let url = format!("redis://{}:{}", host_str, port);
		urls.push(url.clone());
		eprintln!("  Node {}: {}", port - 17000, url);
	}

	// Wait for cluster to be ready by checking cluster state with Redis client
	eprintln!("Waiting for Redis Cluster to become ready...");

	// Use first node's port (17000) for readiness check
	{
		use std::time::Duration;

		const MAX_ATTEMPTS: u32 = 60;
		const RETRY_INTERVAL_MS: u64 = 500;
		let port: u16 = 17000;

		let mut cluster_ready = false;
		for attempt in 1..=MAX_ATTEMPTS {
			let url = format!("redis://{}:{}", host_str, port);

			match redis::Client::open(url.as_str()) {
				Ok(client) => match client.get_multiplexed_async_connection().await {
					Ok(mut conn) => {
						match redis::cmd("PING").query_async::<String>(&mut conn).await {
							Ok(_) => {
								let result: Result<String, redis::RedisError> =
									redis::cmd("CLUSTER")
										.arg("INFO")
										.query_async(&mut conn)
										.await;

								if let Ok(info) = result {
									if info.contains("cluster_state:ok") {
										eprintln!(
											"✓ Redis Cluster is ready (attempt {}/{})",
											attempt, MAX_ATTEMPTS
										);
										cluster_ready = true;
										break;
									} else if attempt <= 3 || attempt % 10 == 0 {
										eprintln!(
											"  Cluster initializing... (attempt {}/{})",
											attempt, MAX_ATTEMPTS
										);
									}
								} else if attempt <= 3 || attempt % 10 == 0 {
									eprintln!(
										"  CLUSTER INFO command failed (attempt {}/{})",
										attempt, MAX_ATTEMPTS
									);
								}
							}
							Err(e) => {
								if attempt <= 3 || attempt % 10 == 0 {
									eprintln!(
										"  Node not responding to PING (attempt {}/{}): {}",
										attempt, MAX_ATTEMPTS, e
									);
								}
							}
						}
					}
					Err(e) => {
						if attempt <= 3 || attempt % 10 == 0 {
							eprintln!(
								"  Connection failed (attempt {}/{}): {}",
								attempt, MAX_ATTEMPTS, e
							);
						}
					}
				},
				Err(e) => {
					if attempt <= 3 {
						eprintln!(
							"  Client creation failed (attempt {}/{}): {}",
							attempt, MAX_ATTEMPTS, e
						);
					}
				}
			}

			if attempt < MAX_ATTEMPTS {
				tokio::time::sleep(Duration::from_millis(RETRY_INTERVAL_MS)).await;
			}
		}

		if !cluster_ready {
			panic!(
				"Redis Cluster did not become ready after {} attempts ({}s timeout). \
                Image: neohq/redis-cluster:latest (ports 17000-17005). \
                Check that Docker/Podman is running and ports 17000-17005 are available.",
				MAX_ATTEMPTS,
				(MAX_ATTEMPTS as u64 * RETRY_INTERVAL_MS) / 1000
			);
		}
	}

	eprintln!("✓ Redis Cluster is ready for testing");

	(cluster, urls)
}

/// Level 5: Redis Cluster fixture with automatic cleanup via RedisClusterGuard
///
/// This is the top-level fixture that depends on `redis_cluster_container`
/// and waits for the cluster to become ready before returning a Guard.
///
/// This fixture returns a `RedisClusterGuard` that automatically manages the
/// lifecycle of a Redis Cluster container. When the guard is dropped (at test end),
/// the container is automatically cleaned up.
///
/// # Usage in tests
///
/// ```ignore
/// #[rstest]
/// #[serial(redis_cluster)]
/// #[tokio::test]
/// async fn test_redis_operations(
///     #[future] redis_cluster: RedisClusterGuard
/// ) {
///     let cluster = redis_cluster.await;
///     let urls = cluster.urls();  // Get cluster node URLs
///
///     // Use cluster...
///     // No explicit cleanup needed - automatic via Drop
/// }
/// ```
///
/// # Partial Usage Examples
///
/// You can use intermediate fixtures for testing specific stages:
///
/// ```ignore
/// // Test container startup only (skip cluster readiness wait)
/// #[rstest]
/// #[tokio::test]
/// async fn test_container_only(
///     #[future] redis_cluster_container: RedisClusterContainer
/// ) {
///     let container = redis_cluster_container.await;
///     assert_eq!(container.urls.len(), 6);
/// }
///
/// // Test port availability only
/// #[rstest]
/// #[tokio::test]
/// async fn test_ports_only(
///     #[future] redis_cluster_ports_ready: Result<FileLockGuard, Box<dyn std::error::Error>>
/// ) {
///     assert!(redis_cluster_ports_ready.await.is_ok());
/// }
/// ```
///
/// # Arguments
///
/// - `redis_cluster_container`: Container from previous fixture (async, requires #[future])
///
/// # Returns
///
/// - `RedisClusterGuard`: Guard that encapsulates the container and provides access to cluster URLs
///
/// # Panics
///
/// Panics if the Redis Cluster fails to become ready within the timeout period.
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn redis_cluster(
	#[future] redis_cluster_container: RedisClusterContainer,
) -> crate::containers::RedisClusterGuard {
	use std::time::Duration;

	let container_info = redis_cluster_container.await;

	eprintln!("Waiting for Redis Cluster to become ready...");

	// Wait for cluster to be ready by checking cluster state with Redis client
	const MAX_ATTEMPTS: u32 = 60;
	const RETRY_INTERVAL_MS: u64 = 500;
	let host = &container_info.host;
	let port: u16 = 17000;

	let mut cluster_ready = false;
	for attempt in 1..=MAX_ATTEMPTS {
		let url = format!("redis://{}:{}", host, port);

		match redis::Client::open(url.as_str()) {
			Ok(client) => {
				match client.get_multiplexed_async_connection().await {
					Ok(mut conn) => {
						// Stage 1: Check node is alive with PING
						match redis::cmd("PING").query_async::<String>(&mut conn).await {
							Ok(_) => {
								// Stage 2: Check cluster state with CLUSTER INFO
								let result: Result<String, redis::RedisError> =
									redis::cmd("CLUSTER")
										.arg("INFO")
										.query_async(&mut conn)
										.await;

								if let Ok(info) = result {
									if info.contains("cluster_state:ok") {
										eprintln!(
											"✓ Redis Cluster is ready (attempt {}/{})",
											attempt, MAX_ATTEMPTS
										);
										cluster_ready = true;
										break;
									} else {
										// Log only on first 3 attempts and every 10th attempt
										if attempt <= 3 || attempt % 10 == 0 {
											eprintln!(
												"  Cluster initializing... (attempt {}/{})",
												attempt, MAX_ATTEMPTS
											);
										}
									}
								} else if attempt <= 3 || attempt % 10 == 0 {
									eprintln!(
										"  CLUSTER INFO command failed (attempt {}/{})",
										attempt, MAX_ATTEMPTS
									);
								}
							}
							Err(e) => {
								if attempt <= 3 || attempt % 10 == 0 {
									eprintln!(
										"  Node not responding to PING (attempt {}/{}): {}",
										attempt, MAX_ATTEMPTS, e
									);
								}
							}
						}
					}
					Err(e) => {
						// Log connection errors only on first 3 attempts and every 10th
						if attempt <= 3 || attempt % 10 == 0 {
							eprintln!(
								"  Connection failed (attempt {}/{}): {}",
								attempt, MAX_ATTEMPTS, e
							);
						}
					}
				}
			}
			Err(e) => {
				if attempt <= 3 {
					eprintln!(
						"  Client creation failed (attempt {}/{}): {}",
						attempt, MAX_ATTEMPTS, e
					);
				}
			}
		}

		if attempt < MAX_ATTEMPTS {
			tokio::time::sleep(Duration::from_millis(RETRY_INTERVAL_MS)).await;
		}
	}

	if !cluster_ready {
		panic!(
			"Redis Cluster did not become ready after {} attempts ({}s timeout). \
             Image: neohq/redis-cluster:latest (ports 17000-17005). \
             Check that Docker/Podman is running and ports 17000-17005 are available.",
			MAX_ATTEMPTS,
			(MAX_ATTEMPTS as u64 * RETRY_INTERVAL_MS) / 1000
		);
	}

	eprintln!("✓ Redis Cluster is ready for testing");

	// Create and return guard - automatic cleanup via Drop
	crate::containers::RedisClusterGuard::new(container_info.container, container_info.urls)
		.await
		.expect("Failed to create RedisClusterGuard")
}

/// LocalStack container fixture for AWS services testing
///
/// This fixture provides a LocalStack container that emulates AWS services locally.
/// Useful for testing AWS integrations without actual AWS credentials.
///
/// # Examples
///
/// ```no_run
/// use rstest::*;
/// use reinhardt_test::fixtures::localstack_fixture;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_localstack(
///     #[future] localstack_fixture: (ContainerAsync<GenericImage>, String)
/// ) {
///     let (_container, endpoint_url) = localstack_fixture.await;
///     // Use endpoint_url to configure AWS SDK
/// }
/// ```
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn localstack_fixture() -> (
	testcontainers::ContainerAsync<testcontainers::GenericImage>,
	String,
) {
	use std::time::Duration;
	use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

	// LocalStack community image - minimal configuration for faster startup
	// No wait condition - rely on port mapping and sleep instead
	let localstack = GenericImage::new("localstack/localstack", "latest")
		.with_env_var("SERVICES", "secretsmanager") // Only enable Secrets Manager service
		.with_env_var("EDGE_PORT", "4566") // Default LocalStack edge port
		.start()
		.await
		.expect("Failed to start LocalStack container");

	// Get the mapped port for LocalStack edge port (4566)
	let port = localstack
		.get_host_port_ipv4(4566)
		.await
		.expect("Failed to get LocalStack port");

	// Construct endpoint URL
	let endpoint_url = format!("http://localhost:{}", port);

	eprintln!("LocalStack started at: {}", endpoint_url);

	// Wait for LocalStack to fully initialize (no log watching, just sleep)
	tokio::time::sleep(Duration::from_secs(5)).await;

	(localstack, endpoint_url)
}

// ============================================================================
// Advanced Setup/Teardown Fixtures using resource.rs
// ============================================================================

#[cfg(feature = "testcontainers")]
pub use suite_resources::*;

/// Suite-wide shared resources using `resource.rs` SuiteResource pattern
#[cfg(feature = "testcontainers")]
mod suite_resources {
	use super::*;
	use crate::resource::{SuiteGuard, SuiteResource, acquire_suite};
	use std::sync::{Mutex, OnceLock, Weak};

	#[cfg(feature = "testcontainers")]
	use testcontainers::core::{ContainerPort, WaitFor};

	/// Suite-wide PostgreSQL container resource
	///
	/// This resource is shared across all tests in the suite and automatically
	/// cleaned up when the last test completes. Uses `SuiteResource` pattern
	/// from `resource.rs` for safe lifecycle management.
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_test::fixtures::*;
	/// use rstest::*;
	///
	/// #[rstest]
	/// #[tokio::test]
	/// async fn test_database_query(postgres_suite: SuiteGuard<PostgresSuiteResource>) {
	///     let pool = &postgres_suite.pool;
	///     let result = sqlx::query("SELECT 1").fetch_one(pool).await;
	///     assert!(result.is_ok());
	/// }
	/// ```
	pub struct PostgresSuiteResource {
		// Note: Container must be held to keep it alive during test suite execution
		// TestContainers automatically stops/removes containers when dropped
		#[allow(dead_code)]
		pub container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
		pub pool: sqlx::postgres::PgPool,
		pub port: u16,
		pub database_url: String,
	}

	impl SuiteResource for PostgresSuiteResource {
		fn init() -> Self {
			// Block on async initialization (SuiteResource::init is sync)
			tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current().block_on(async { Self::init_async().await })
			})
		}
	}

	impl PostgresSuiteResource {
		async fn init_async() -> Self {
			use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

			let postgres = GenericImage::new("postgres", "17-alpine")
				.with_wait_for(WaitFor::message_on_stderr(
					"database system is ready to accept connections",
				))
				.with_exposed_port(ContainerPort::Tcp(5432))
				.with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
				.start()
				.await
				.expect("Failed to start PostgreSQL container");

			let port = postgres
				.get_host_port_ipv4(ContainerPort::Tcp(5432))
				.await
				.expect("Failed to get PostgreSQL port");

			let database_url = format!("postgres://postgres@localhost:{}/postgres", port);

			// Retry connection with exponential backoff
			let pool = {
				use sqlx::postgres::PgPoolOptions;
				use std::time::Duration;

				const MAX_RETRIES: u32 = 10;
				let mut pool_result = None;

				for attempt in 0..MAX_RETRIES {
					match PgPoolOptions::new()
						.max_connections(5)
						.acquire_timeout(Duration::from_secs(3))
						.connect(&database_url)
						.await
					{
						Ok(pool) => {
							pool_result = Some(pool);
							break;
						}
						Err(e) if attempt < MAX_RETRIES - 1 => {
							eprintln!(
								"Connection attempt {} failed: {}. Retrying...",
								attempt + 1,
								e
							);
							tokio::time::sleep(Duration::from_millis(100 * (attempt as u64 + 1)))
								.await;
						}
						Err(e) => panic!(
							"Failed to connect to PostgreSQL after {} retries: {}",
							MAX_RETRIES, e
						),
					}
				}

				pool_result.expect("Pool should be initialized")
			};

			Self {
				container: postgres,
				pool,
				port,
				database_url,
			}
		}
	}

	static POSTGRES_SUITE: OnceLock<Mutex<Weak<PostgresSuiteResource>>> = OnceLock::new();

	/// Acquire shared PostgreSQL suite resource
	///
	/// This fixture provides a suite-wide PostgreSQL container that is shared
	/// across all tests and automatically cleaned up when the last test completes.
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_test::fixtures::*;
	/// use rstest::*;
	///
	/// #[rstest]
	/// #[tokio::test]
	/// async fn test_example(postgres_suite: SuiteGuard<PostgresSuiteResource>) {
	///     let pool = &postgres_suite.pool;
	///     // Use pool in test
	/// }
	/// ```
	#[fixture]
	pub fn postgres_suite() -> SuiteGuard<PostgresSuiteResource> {
		acquire_suite(&POSTGRES_SUITE)
	}

	/// Suite-wide MySQL container resource
	pub struct MySqlSuiteResource {
		// Note: Container must be held to keep it alive during test suite execution
		// TestContainers automatically stops/removes containers when dropped
		#[allow(dead_code)]
		pub container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
		pub pool: sqlx::mysql::MySqlPool,
		pub port: u16,
		pub database_url: String,
	}

	impl SuiteResource for MySqlSuiteResource {
		fn init() -> Self {
			tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current().block_on(async { Self::init_async().await })
			})
		}
	}

	impl MySqlSuiteResource {
		async fn init_async() -> Self {
			use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};

			let mysql = GenericImage::new("mysql", "8.0")
				.with_wait_for(WaitFor::message_on_stderr("ready for connections"))
				.with_exposed_port(ContainerPort::Tcp(3306))
				.with_env_var("MYSQL_ROOT_PASSWORD", "test")
				.with_env_var("MYSQL_DATABASE", "test")
				.start()
				.await
				.expect("Failed to start MySQL container");

			let port = mysql
				.get_host_port_ipv4(ContainerPort::Tcp(3306))
				.await
				.expect("Failed to get MySQL port");

			let database_url = format!("mysql://root:test@localhost:{}/test", port);

			// Retry connection with exponential backoff
			let pool = {
				use sqlx::mysql::MySqlPoolOptions;
				use std::time::Duration;

				const MAX_RETRIES: u32 = 10;
				let mut pool_result = None;

				for attempt in 0..MAX_RETRIES {
					match MySqlPoolOptions::new()
						.max_connections(5)
						.acquire_timeout(Duration::from_secs(3))
						.connect(&database_url)
						.await
					{
						Ok(pool) => {
							pool_result = Some(pool);
							break;
						}
						Err(e) if attempt < MAX_RETRIES - 1 => {
							eprintln!(
								"Connection attempt {} failed: {}. Retrying...",
								attempt + 1,
								e
							);
							tokio::time::sleep(Duration::from_millis(100 * (attempt as u64 + 1)))
								.await;
						}
						Err(e) => panic!(
							"Failed to connect to MySQL after {} retries: {}",
							MAX_RETRIES, e
						),
					}
				}

				pool_result.expect("Pool should be initialized")
			};

			Self {
				container: mysql,
				pool,
				port,
				database_url,
			}
		}
	}

	static MYSQL_SUITE: OnceLock<Mutex<Weak<MySqlSuiteResource>>> = OnceLock::new();

	/// Acquire shared MySQL suite resource
	#[fixture]
	pub fn mysql_suite() -> SuiteGuard<MySqlSuiteResource> {
		acquire_suite(&MYSQL_SUITE)
	}
}

// ============================================================================
// Per-test Resources using TestResource pattern
// ============================================================================

pub use test_resources::*;

/// Per-test resources using `resource.rs` TestResource pattern
mod test_resources {
	use super::*;
	use crate::resource::{TeardownGuard, TestResource};
	use std::path::PathBuf;

	/// Per-test template directory resource with automatic cleanup
	///
	/// Creates a temporary directory for template files and automatically
	/// removes it when the test completes.
	///
	/// ## Example
	///
	/// ```rust
	/// use reinhardt_test::fixtures::*;
	/// use rstest::*;
	///
	/// #[rstest]
	/// fn test_template_rendering(template_dir: TeardownGuard<TemplateDirResource>) {
	///     let dir = template_dir.path();
	///     // Write template files to dir
	///     // Directory is automatically cleaned up
	/// }
	/// ```
	pub struct TemplateDirResource {
		path: PathBuf,
	}

	impl TemplateDirResource {
		pub fn path(&self) -> &PathBuf {
			&self.path
		}
	}

	impl TestResource for TemplateDirResource {
		fn setup() -> Self {
			let test_id = uuid::Uuid::new_v4();
			let path = PathBuf::from(format!("/tmp/reinhardt_template_test_{}", test_id));
			std::fs::create_dir_all(&path).expect("Failed to create template test directory");
			Self { path }
		}

		fn teardown(&mut self) {
			if self.path.exists() {
				std::fs::remove_dir_all(&self.path)
					.unwrap_or_else(|e| eprintln!("Failed to cleanup template directory: {}", e));
			}
		}
	}

	/// Per-test template directory fixture
	#[fixture]
	pub fn template_dir() -> TeardownGuard<TemplateDirResource> {
		TeardownGuard::new()
	}
}

// ================================================================================
// Mock Database Connection Fixtures
// ================================================================================

#[cfg(feature = "testcontainers")]
pub use mock_database::*;

#[cfg(feature = "testcontainers")]
mod mock_database {
	use reinhardt_db::backends::Result;
	use reinhardt_db::backends::backend::DatabaseBackend as BackendTrait;
	use reinhardt_db::backends::connection::DatabaseConnection as BackendsConnection;
	use reinhardt_db::backends::types::{DatabaseType, QueryResult, QueryValue, Row};
	use reinhardt_orm::{DatabaseBackend, DatabaseConnection};
	use rstest::*;
	use std::sync::Arc;

	/// Mock backend implementation for database testing
	///
	/// This mock backend provides a no-op implementation of all database operations,
	/// useful for testing code that depends on DatabaseConnection without requiring
	/// an actual database.
	struct MockBackend;

	#[async_trait::async_trait]
	impl BackendTrait for MockBackend {
		fn database_type(&self) -> DatabaseType {
			DatabaseType::Postgres
		}

		fn placeholder(&self, index: usize) -> String {
			format!("${}", index)
		}

		fn supports_returning(&self) -> bool {
			true
		}

		fn supports_on_conflict(&self) -> bool {
			true
		}

		async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult { rows_affected: 0 })
		}

		async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			Ok(Row::new())
		}

		async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(Vec::new())
		}

		async fn fetch_optional(
			&self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			Ok(None)
		}

		fn as_any(&self) -> &dyn std::any::Any {
			self
		}
	}

	/// Fixture for creating a mock database connection
	///
	/// Returns a DatabaseConnection with a mock backend that doesn't perform
	/// actual database operations. Useful for testing code that requires a
	/// connection but doesn't need real data.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_test::fixtures::mock_connection;
	/// use rstest::*;
	///
	/// #[rstest]
	/// fn test_with_mock_db(mock_connection: DatabaseConnection) {
	///     // Use mock_connection for testing
	/// }
	/// ```
	#[fixture]
	pub fn mock_connection() -> DatabaseConnection {
		let mock_backend = Arc::new(MockBackend);
		let backends_conn = BackendsConnection::new(mock_backend);
		DatabaseConnection::new(DatabaseBackend::Postgres, backends_conn)
	}
}
