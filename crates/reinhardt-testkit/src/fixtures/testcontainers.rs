#[cfg(feature = "testcontainers")]
use rstest::*;
#[cfg(feature = "testcontainers")]
use std::sync::Arc;

#[cfg(feature = "testcontainers")]
use testcontainers::{
	ImageExt,
	core::{ContainerPort, WaitFor},
	runners::AsyncRunner,
};

// Public re-exports for fixtures.rs
#[cfg(feature = "testcontainers")]
pub use testcontainers::{ContainerAsync, GenericImage};

/// RAII guard for an ORM handle backed by a migration test database.
#[cfg(feature = "testcontainers")]
pub struct MigrationDatabase {
	connection: reinhardt_db::orm::DatabaseConnection,
	_connection_lease: reinhardt_db::orm::DatabaseConnectionLease,
}

#[cfg(feature = "testcontainers")]
impl std::ops::Deref for MigrationDatabase {
	type Target = reinhardt_db::orm::DatabaseConnection;

	fn deref(&self) -> &Self::Target {
		&self.connection
	}
}

/// Check if a port is available (not in use by any process)
#[cfg(feature = "testcontainers")]
async fn is_port_available(port: u16) -> bool {
	use tokio::net::TcpListener;
	TcpListener::bind(format!("127.0.0.1:{}", port))
		.await
		.is_ok()
}

/// Check if all 6 consecutive ports starting from base_port are available
#[cfg(feature = "testcontainers")]
async fn is_port_range_available(base_port: u16) -> bool {
	for offset in 0..6 {
		if !is_port_available(base_port + offset).await {
			return false;
		}
	}
	true
}

/// Get database connection pool configuration from environment variables.
///
/// This function reads pool configuration from environment variables,
/// falling back to sensible defaults if not set.
///
/// # Environment Variables
/// - `TEST_MAX_CONNECTIONS`: Maximum number of connections in the pool (default: 20)
/// - `TEST_ACQUIRE_TIMEOUT_SECS`: Timeout in seconds for acquiring a connection (default: 60)
///
/// # Returns
/// A tuple of (max_connections, acquire_timeout_secs)
///
/// # Example
/// ```bash
/// # Use custom pool configuration
/// TEST_MAX_CONNECTIONS=10 TEST_ACQUIRE_TIMEOUT_SECS=120 cargo nextest run
///
/// # Use default configuration (max_connections=5, timeout=60s)
/// cargo nextest run
/// ```
#[cfg(feature = "testcontainers")]
fn get_pool_config() -> (u32, u64) {
	let max_connections = std::env::var("TEST_MAX_CONNECTIONS")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(5); // Default: 5 (MUST be > 1 to avoid sqlx v0.7+ prepared statement cache bug #2885)

	let acquire_timeout = std::env::var("TEST_ACQUIRE_TIMEOUT_SECS")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(60); // Default: 60s - shorter timeout exposes real issues faster

	(max_connections, acquire_timeout)
}

/// Create an AnyPool with proper timeout configuration for tests.
///
/// This function uses the same timeout settings as `postgres_container` fixture,
/// ensuring consistent behavior across all test database connections.
///
/// # Arguments
/// * `database_url` - Connection URL (postgres://, mysql://, sqlite://)
///
/// # Example
/// ```no_run
/// use reinhardt_testkit::fixtures::testcontainers::create_test_any_pool;
///
/// # async fn example() {
/// let database_url = "postgres://localhost:5432/test";
/// let pool = create_test_any_pool(database_url).await.expect("Failed to connect");
/// # }
/// ```
#[cfg(feature = "testcontainers")]
pub async fn create_test_any_pool(database_url: &str) -> Result<sqlx::AnyPool, sqlx::Error> {
	use sqlx::any::AnyPoolOptions;

	let (max_conns, timeout_secs) = get_pool_config();

	AnyPoolOptions::new()
		.max_connections(max_conns)
		.min_connections(1)
		.acquire_timeout(std::time::Duration::from_secs(timeout_secs))
		.idle_timeout(std::time::Duration::from_secs(600))
		.max_lifetime(std::time::Duration::from_secs(1800))
		.connect(database_url)
		.await
}

/// Fixture: Find and return an available port range for Redis Cluster.
///
/// This fixture automatically searches for 6 consecutive available ports,
/// ensuring tests never fail due to port conflicts.
///
/// Port selection strategy:
/// 1. Check REDIS_CLUSTER_BASE_PORT environment variable (default: 17000)
/// 2. Verify all 6 consecutive ports are available
/// 3. If not available, try candidates: 27000, 37000, 47000
/// 4. If all candidates occupied, search 20000-60000 in steps of 1000
/// 5. Panic if no available range found
///
/// # Returns
/// Base port number where ports [base_port, base_port+5] are all available
///
/// # Example
/// ```rust
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_auto_ports(
///     #[future] redis_cluster_base_port: u16
/// ) {
///     let base = redis_cluster_base_port.await;
///     // Use ports: base, base+1, ..., base+5
/// }
/// ```
#[fixture]
#[cfg(feature = "testcontainers")]
pub async fn redis_cluster_base_port() -> u16 {
	// Generate process-specific port offset to avoid conflicts in parallel test execution
	// Each process gets a unique 10-port range based on its PID
	let pid = std::process::id();
	let pid_offset = ((pid % 10) * 10) as u16;
	let pid_based_port = 17000 + pid_offset;

	// Priority order:
	// 1. Environment variable (explicit override)
	// 2. PID-based port (automatic per-process allocation)
	// 3. Default 17000
	let env_preferred = std::env::var("REDIS_CLUSTER_BASE_PORT")
		.ok()
		.and_then(|s| s.parse().ok());

	// Build candidate list with priorities
	let mut candidates = Vec::new();

	// First priority: Environment variable override
	if let Some(env_port) = env_preferred {
		candidates.push(env_port);
	}

	// Second priority: PID-based port (for parallel execution)
	candidates.push(pid_based_port);

	// Third priority: Default 17000
	if !candidates.contains(&17000) {
		candidates.push(17000);
	}

	// Fourth priority: Standard fallbacks
	candidates.extend_from_slice(&[27000, 37000, 47000]);

	// Try each candidate
	for &candidate in &candidates {
		if is_port_range_available(candidate).await {
			eprintln!(
				"Using Redis Cluster port range: {}-{} (PID: {}, offset: {})",
				candidate,
				candidate + 5,
				pid,
				if candidate == pid_based_port {
					format!("{} [PID-based]", pid_offset)
				} else {
					"N/A".to_string()
				}
			);
			return candidate;
		}
	}

	eprintln!("WARNING: All preferred port ranges are occupied. Searching 20000-60000...");

	// If all predefined candidates are occupied, search for any available range
	// Start from 20000 to avoid well-known ports
	for base in (20000..60000).step_by(1000) {
		if is_port_range_available(base).await {
			eprintln!(
				"Found available port range: {}-{} (searched from 20000)",
				base,
				base + 5
			);
			return base;
		}
	}

	panic!(
		"Failed to find 6 consecutive available ports. Please free up some ports and try again."
	);
}

// File locking support
use fs2::FileExt;

// ============================================================================
// File Lock Guard for Inter-Process Synchronization
// ============================================================================

/// File-based lock guard for inter-process synchronization
///
/// Uses fs2::FileExt for cross-platform file locking. This is essential for
/// tests that require exclusive access to shared resources across process boundaries.
///
/// # Platform Support
///
/// - **Unix**: Uses advisory locking via flock(2)
/// - **Windows**: Uses mandatory locking via LockFileEx
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::FileLockGuard;
///
/// // Acquire lock (blocks until available)
/// let guard = FileLockGuard::new("/tmp/test.lock")?;
///
/// // Perform exclusive operations...
///
/// // Lock automatically released when guard drops
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct FileLockGuard {
	file: std::fs::File,
}

impl FileLockGuard {
	/// Create a new file lock guard
	///
	/// This will block the current thread until the lock can be acquired.
	///
	/// # Errors
	///
	/// Returns an error if the lock file cannot be created or locked.
	pub fn new(lock_path: impl Into<std::path::PathBuf>) -> std::io::Result<Self> {
		let path: std::path::PathBuf = lock_path.into();
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(false)
			.open(&path)?;

		file.lock_exclusive()?;

		Ok(Self { file })
	}
}

impl Drop for FileLockGuard {
	fn drop(&mut self) {
		// Only unlock; do not remove the lock file.
		// Removing the file after unlock creates a race condition where another
		// process can acquire the lock between unlock and delete, then the
		// delete removes a valid lock held by that process.
		let _ = self.file.unlock();
	}
}

// ============================================================================
// PostgreSQL Container Fixtures
// ============================================================================

/// Configuration for a test-scoped PostgreSQL container.
///
/// Defaults to `postgres:16-alpine`, user and database `postgres`, trust
/// authentication, a random host port, the standard readiness log message,
/// and a 120-second startup timeout. Pool settings remain controlled by
/// `TEST_MAX_CONNECTIONS` and `TEST_ACQUIRE_TIMEOUT_SECS`.
///
/// Available with the `testcontainers` feature for native Docker tests (P0).
///
/// # Examples
///
/// ```rust
/// use reinhardt_testkit::PostgresContainerConfig;
///
/// let config = PostgresContainerConfig::default()
///     .image("postgres", "16-alpine")
///     .user("test_user")
///     .password("test_password")
///     .database("test_db")
///     .args(["-c", "max_connections=400"]);
/// ```
#[derive(Clone, Debug)]
pub struct PostgresContainerConfig {
	image_name: String,
	image_tag: String,
	user: String,
	password: Option<String>,
	database: String,
	extra_env: std::collections::BTreeMap<String, String>,
	command_args: Vec<String>,
	host_port: Option<u16>,
	wait_for: WaitFor,
	startup_timeout: std::time::Duration,
}

impl Default for PostgresContainerConfig {
	fn default() -> Self {
		Self {
			image_name: "postgres".into(),
			image_tag: "16-alpine".into(),
			user: "postgres".into(),
			password: None,
			database: "postgres".into(),
			extra_env: std::collections::BTreeMap::new(),
			command_args: Vec::new(),
			host_port: None,
			wait_for: WaitFor::message_on_stderr("database system is ready to accept connections"),
			startup_timeout: std::time::Duration::from_secs(120),
		}
	}
}

impl PostgresContainerConfig {
	/// Set the Docker image name and tag. It must support the PostgreSQL entrypoint.
	pub fn image(mut self, name: impl Into<String>, tag: impl Into<String>) -> Self {
		self.image_name = name.into();
		self.image_tag = tag.into();
		self
	}

	/// Set both `POSTGRES_USER` and the connection URL's user.
	pub fn user(mut self, user: impl Into<String>) -> Self {
		self.user = user.into();
		self
	}

	/// Enable password authentication with `POSTGRES_PASSWORD` and URL credentials.
	///
	/// Without this method, trust authentication is used. Reserved characters in
	/// credentials are percent-encoded in the URL.
	pub fn password(mut self, password: impl Into<String>) -> Self {
		self.password = Some(password.into());
		self
	}

	/// Set both `POSTGRES_DB` and the connection URL's database.
	pub fn database(mut self, database: impl Into<String>) -> Self {
		self.database = database.into();
		self
	}

	/// Add an environment variable, replacing an earlier value for the same key.
	///
	/// `POSTGRES_USER`, `POSTGRES_DB`, `POSTGRES_PASSWORD`, and
	/// `POSTGRES_HOST_AUTH_METHOD` are derived from the credentials and take
	/// precedence over extra environment variables, including removal of
	/// incompatible authentication settings.
	pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.extra_env.insert(key.into(), value.into());
		self
	}

	/// Append a command argument for the container's PostgreSQL entrypoint.
	pub fn arg(mut self, arg: impl Into<String>) -> Self {
		self.command_args.push(arg.into());
		self
	}

	/// Append command arguments, for example `["-c", "max_connections=400"]`.
	pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
		self.command_args.extend(args.into_iter().map(Into::into));
		self
	}

	/// Map a fixed host port to container TCP port 5432.
	///
	/// Fixed ports can collide across parallel test processes, including nextest.
	/// Callers are responsible for serializing tests that share a fixed port;
	/// an in-process lock alone does not serialize separate nextest processes.
	/// The default lets Docker choose a random host port.
	pub fn host_port(mut self, port: u16) -> Self {
		self.host_port = Some(port);
		self
	}

	/// Replace the container's wait condition.
	///
	/// Connection and `SELECT 1` readiness retries always run, even with
	/// `WaitFor::Nothing`.
	pub fn wait_for(mut self, wait_for: WaitFor) -> Self {
		self.wait_for = wait_for;
		self
	}

	/// Set the timeout for the Docker container's startup wait condition.
	///
	/// This does not change the subsequent connection and readiness retries.
	pub fn startup_timeout(mut self, timeout: std::time::Duration) -> Self {
		self.startup_timeout = timeout;
		self
	}

	fn environment(&self) -> std::collections::BTreeMap<&str, &str> {
		let mut env: std::collections::BTreeMap<_, _> = self
			.extra_env
			.iter()
			.map(|(key, value)| (key.as_str(), value.as_str()))
			.collect();
		env.insert("POSTGRES_USER", &self.user);
		env.insert("POSTGRES_DB", &self.database);
		if let Some(password) = &self.password {
			env.remove("POSTGRES_HOST_AUTH_METHOD");
			env.insert("POSTGRES_PASSWORD", password);
		} else {
			env.remove("POSTGRES_PASSWORD");
			env.insert("POSTGRES_HOST_AUTH_METHOD", "trust");
		}
		env
	}

	fn database_url(&self, port: u16) -> String {
		let user = urlencoding::encode(&self.user);
		let database = urlencoding::encode(&self.database);
		let credentials = match &self.password {
			Some(password) => format!("{user}:{}", urlencoding::encode(password)),
			None => user.into_owned(),
		};
		format!("postgres://{credentials}@localhost:{port}/{database}?sslmode=disable")
	}
}

/// Fixture providing a PostgreSQL container with connection pool
///
/// Starts a PostgreSQL 16 Alpine container and provides a connection pool
/// for testing database operations.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::{ContainerAsync, GenericImage, postgres_container};
/// use rstest::*;
/// use std::sync::Arc;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_postgres(
///     #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String)
/// ) {
///     let (_container, pool, port, url) = postgres_container.await;
///     let result = sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await;
///     assert!(result.is_ok());
/// }
/// ```
#[fixture]
pub async fn postgres_container() -> (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String)
{
	start_postgres_container(PostgresContainerConfig::default()).await
}

/// PostgreSQL fixture accepting a configuration through rstest's `#[with(...)]`.
///
/// Omitting the configuration uses the same defaults as `postgres_container`.
/// Available with the `testcontainers` feature for native Docker tests (P0).
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::{
///     ContainerAsync, GenericImage, PostgresContainerConfig, postgres_container_with,
/// };
/// use rstest::rstest;
/// use std::sync::Arc;
///
/// #[rstest]
/// #[tokio::test]
/// async fn custom_postgres(
///     #[with(PostgresContainerConfig::default().args(["-c", "max_connections=400"]))]
///     #[future] postgres_container_with: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
/// ) {
///     let (_container, pool, _port, _url) = postgres_container_with.await;
///     assert_eq!(pool.is_closed(), false);
/// }
/// ```
#[fixture]
pub async fn postgres_container_with(
	#[default(PostgresContainerConfig::default())] config: PostgresContainerConfig,
) -> (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String) {
	start_postgres_container(config).await
}

/// Start a configured PostgreSQL container and return its guard, pool, port, and URL.
///
/// Port discovery, connection establishment, and `SELECT 1` readiness checks
/// always retry independently of the configured wait condition. Keep the
/// returned container guard alive while using its pool or URL.
/// Available with the `testcontainers` feature for native Docker tests (P0).
///
/// # Panics
///
/// Panics if Docker startup, port discovery, or database readiness fails.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::{PostgresContainerConfig, start_postgres_container};
///
/// # async fn example() {
/// let (_container, pool, _port, url) = start_postgres_container(
///     PostgresContainerConfig::default().database("test_db"),
/// ).await;
/// assert_eq!(pool.is_closed(), false);
/// # }
/// ```
pub async fn start_postgres_container(
	config: PostgresContainerConfig,
) -> (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String) {
	use testcontainers::core::IntoContainerPort;

	let mut image = GenericImage::new(&config.image_name, &config.image_tag)
		.with_exposed_port(5432.tcp())
		.with_wait_for(config.wait_for.clone())
		.with_startup_timeout(config.startup_timeout)
		.with_cmd(config.command_args.iter().cloned());
	for (key, value) in config.environment() {
		image = image.with_env_var(key, value);
	}
	if let Some(port) = config.host_port {
		image = image.with_mapped_port(port, 5432.tcp());
	}

	let postgres = image
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	// Wait briefly before first port query to ensure container networking is ready
	// Increased from 200ms to 500ms for better reliability under heavy load
	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

	// Retry getting port with exponential backoff
	let mut port_retry = 0;
	let max_port_retries = 7; // Increased from 5 for better reliability under load
	let port = loop {
		match postgres.get_host_port_ipv4(5432).await {
			Ok(p) => break p,
			Err(e) if port_retry < max_port_retries => {
				port_retry += 1;
				let delay = tokio::time::Duration::from_millis(200 * 2_u64.pow(port_retry));
				eprintln!(
					"PostgreSQL port query attempt {} of {} failed: {:?}",
					port_retry, max_port_retries, e
				);
				tokio::time::sleep(delay).await;
			}
			Err(e) => {
				panic!(
					"Failed to get PostgreSQL port after {} retries: {}",
					max_port_retries, e
				);
			}
		}
	};

	let database_url = config.database_url(port);

	// Get pool configuration from environment variables
	let (max_conns, timeout_secs) = get_pool_config();

	// Retry connection to PostgreSQL with exponential backoff
	let mut retry_count = 0;
	let max_retries = 7; // Increased from 5 for better reliability in CI environments

	// Wait briefly before first connection to ensure container is fully ready
	tokio::time::sleep(std::time::Duration::from_millis(500)).await;

	let pool = loop {
		match sqlx::postgres::PgPoolOptions::new()
			.max_connections(max_conns)
			.min_connections(1)
			.acquire_timeout(std::time::Duration::from_secs(timeout_secs))
			.idle_timeout(std::time::Duration::from_secs(600)) // Increase from 30s for sqlx v0.7+ compatibility
			.max_lifetime(std::time::Duration::from_secs(1800)) // Increase from 120s for long-running tests
			.test_before_acquire(false) // sqlx v0.7+ bug workaround (issue #2885, #3241)
			.connect(&database_url)
			.await
		{
			Ok(pool) => {
				// Verify wire protocol is working correctly
				match sqlx::query("SELECT 1").fetch_one(&pool).await {
					Ok(_) => break pool,
					Err(e) if retry_count < max_retries => {
						eprintln!(
							"PostgreSQL health check attempt {} of {} failed: {:?}",
							retry_count + 1,
							max_retries,
							e
						);
						retry_count += 1;
						let delay = std::time::Duration::from_millis(200 * 2_u64.pow(retry_count));
						tokio::time::sleep(delay).await;
						continue;
					}
					Err(e) => {
						panic!(
							"PostgreSQL pool created but health check failed after {} retries: {}",
							max_retries, e
						);
					}
				}
			}
			Err(e) if retry_count < max_retries => {
				eprintln!(
					"PostgreSQL connection attempt {} of {} failed: {:?}",
					retry_count + 1,
					max_retries,
					e
				);
				retry_count += 1;
				let delay = std::time::Duration::from_millis(200 * 2_u64.pow(retry_count));
				tokio::time::sleep(delay).await;
			}
			Err(e) => {
				panic!(
					"Failed to connect to PostgreSQL after {} retries: {}",
					max_retries, e
				);
			}
		}
	};

	(postgres, Arc::new(pool), port, database_url)
}

/// Create a CockroachDB container with a connection pool for testing
pub async fn cockroachdb_container()
-> (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String) {
	use testcontainers::core::IntoContainerPort;

	let cockroachdb = GenericImage::new("cockroachdb/cockroach", "v23.1.0")
		.with_exposed_port(26257.tcp())
		.with_wait_for(WaitFor::message_on_stderr("initialized new cluster"))
		.with_cmd(vec![
			"start-single-node".to_string(),
			"--insecure".to_string(),
			"--store=type=mem,size=1GiB".to_string(),
		])
		.start()
		.await
		.expect("Failed to start CockroachDB container");

	let port = cockroachdb
		.get_host_port_ipv4(26257)
		.await
		.expect("Failed to get CockroachDB port");

	// Connect to postgres database to create defaultdb if needed
	let postgres_url = format!("postgresql://root@127.0.0.1:{}/postgres", port);

	let postgres_pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(1)
		.connect(&postgres_url)
		.await
		.expect("Failed to connect to CockroachDB postgres database");

	// Create defaultdb database
	sqlx::query("CREATE DATABASE IF NOT EXISTS defaultdb")
		.execute(&postgres_pool)
		.await
		.expect("Failed to create defaultdb");

	postgres_pool.close().await;

	// Now connect to defaultdb
	let database_url = format!("postgresql://root@127.0.0.1:{}/defaultdb", port);

	// Get pool configuration from environment variables
	let (max_conns, timeout_secs) = get_pool_config();

	let pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(max_conns)
		.min_connections(1)
		.acquire_timeout(std::time::Duration::from_secs(timeout_secs))
		.idle_timeout(std::time::Duration::from_secs(30))
		.max_lifetime(std::time::Duration::from_secs(120))
		.connect(&database_url)
		.await
		.expect("Failed to connect to CockroachDB defaultdb");

	(cockroachdb, Arc::new(pool), port, database_url)
}

// ============================================================================
// Redis Container Fixtures
// ============================================================================

/// Fixture providing a Redis container
///
/// Starts a Redis 7 Alpine container for testing cache and pub/sub operations.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::redis_container;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_redis(
///     #[future] redis_container: (ContainerAsync<GenericImage>, u16, String)
/// ) {
///     let (_container, port, url) = redis_container.await;
///     let client = redis::Client::open(url.as_str()).unwrap();
///     let mut conn = client.get_multiplexed_async_connection().await.unwrap();
///     redis::cmd("PING").query_async::<String>(&mut conn).await.unwrap();
/// }
/// ```
#[fixture]
pub async fn redis_container() -> (ContainerAsync<GenericImage>, u16, String) {
	const MAX_RETRIES: u32 = 3;
	const RETRY_DELAY_MS: u64 = 2000;

	let mut last_error = None;

	for attempt in 0..MAX_RETRIES {
		match try_start_redis_container().await {
			Ok(result) => return result,
			Err(e) => {
				eprintln!(
					"Redis container start attempt {} of {} failed: {:?}",
					attempt + 1,
					MAX_RETRIES,
					e
				);
				last_error = Some(e);

				if attempt < MAX_RETRIES - 1 {
					tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
				}
			}
		}
	}

	panic!(
		"Failed to start Redis container after {} attempts: {:?}",
		MAX_RETRIES, last_error
	);
}

async fn try_start_redis_container()
-> Result<(ContainerAsync<GenericImage>, u16, String), Box<dyn std::error::Error>> {
	use testcontainers::core::IntoContainerPort;

	let redis = GenericImage::new("redis", "7-alpine")
		.with_exposed_port(6379.tcp())
		.with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
		.start()
		.await?;

	let port = redis.get_host_port_ipv4(6379).await?;

	let url = format!("redis://localhost:{}", port);

	Ok((redis, port, url))
}

// ============================================================================
// NATS Container Fixtures
// ============================================================================

/// Fixture providing a per-test NATS container with JetStream enabled.
///
/// Starts the official `nats` image with `-js` and returns the container, the host
/// port mapped to 4222/tcp, and a `nats://localhost:<port>` connection URL.
/// Readiness requires both startup log messages and a TCP `INFO` response with
/// `"jetstream":true`. Keep the container handle alive for the test duration;
/// dropping it removes the container, including its JetStream data.
///
/// The image name is fixed; only its tag is configurable. The default tag is
/// `2.12-alpine`. Only this default tag is pre-pulled in CI; other tags are pulled
/// at test time and are subject to Docker Hub rate limits.
///
/// Requires the `testcontainers` feature and Docker. This fixture is native-only
/// (P0 target-only behavior); it is not available through the WASM test facade.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::{ContainerAsync, GenericImage, nats_container};
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_nats(
///     #[future] nats_container: (ContainerAsync<GenericImage>, u16, String),
/// ) {
///     let (_container, port, url) = nats_container.await;
///     assert_eq!(url, format!("nats://localhost:{port}"));
/// }
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_another_nats_tag(
///     #[future] #[with("2.11-alpine")]
///     nats_container: (ContainerAsync<GenericImage>, u16, String),
/// ) {
///     let (_container, port, url) = nats_container.await;
///     assert_eq!(url, format!("nats://localhost:{port}"));
/// }
/// ```
#[fixture]
pub async fn nats_container(
	#[default("2.12-alpine")] tag: &str,
) -> (ContainerAsync<GenericImage>, u16, String) {
	const MAX_RETRIES: u32 = 3;
	const RETRY_DELAY_MS: u64 = 2000;

	let mut last_error = None;

	for attempt in 0..MAX_RETRIES {
		match try_start_nats_container(tag).await {
			Ok(result) => return result,
			Err(error) => {
				eprintln!(
					"NATS container start attempt {} of {} failed: {:?}",
					attempt + 1,
					MAX_RETRIES,
					error
				);
				last_error = Some(error);

				if attempt < MAX_RETRIES - 1 {
					tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
				}
			}
		}
	}

	panic!(
		"Failed to start NATS container after {} attempts: {:?}",
		MAX_RETRIES, last_error
	);
}

async fn try_start_nats_container(
	tag: &str,
) -> Result<(ContainerAsync<GenericImage>, u16, String), Box<dyn std::error::Error>> {
	use testcontainers::core::IntoContainerPort;

	let nats = GenericImage::new("nats", tag)
		.with_exposed_port(4222.tcp())
		.with_wait_for(WaitFor::message_on_stderr(
			"Listening for client connections on 0.0.0.0:4222",
		))
		.with_wait_for(WaitFor::message_on_stderr("Server is ready"))
		.with_cmd(["-js"])
		.start()
		.await?;

	let port = nats.get_host_port_ipv4(4222).await?;
	probe_nats_jetstream(port).await?;
	let url = format!("nats://localhost:{port}");

	Ok((nats, port, url))
}

async fn read_nats_info(port: u16) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
	use tokio::io::{AsyncBufReadExt, BufReader};

	let stream = tokio::net::TcpStream::connect(("localhost", port)).await?;
	let mut line = String::new();
	BufReader::new(stream).read_line(&mut line).await?;
	let json = line
		.strip_prefix("INFO ")
		.and_then(|info| info.strip_suffix("\r\n"))
		.ok_or_else(|| {
			std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"NATS did not send a complete initial INFO line",
			)
		})?;

	Ok(serde_json::from_str(json)?)
}

async fn probe_nats_jetstream(port: u16) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
	use std::time::Duration;
	use tokio::time::{Instant, sleep_until, timeout_at};

	let deadline = Instant::now() + Duration::from_secs(30);

	loop {
		// Bound each connection/read attempt as well as the complete readiness loop.
		let attempt_deadline = deadline.min(Instant::now() + Duration::from_secs(1));
		let error: Box<dyn std::error::Error> =
			match timeout_at(attempt_deadline, read_nats_info(port)).await {
				Ok(Ok(info)) if info["jetstream"].as_bool() == Some(true) => return Ok(info),
				Ok(Ok(_)) => Box::new(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					"NATS INFO does not report jetstream:true",
				)),
				Ok(Err(error)) => error,
				Err(error) => Box::new(error),
			};

		if Instant::now() >= deadline {
			return Err(Box::new(std::io::Error::new(
				std::io::ErrorKind::TimedOut,
				format!("NATS JetStream readiness timed out on port {port}: {error}"),
			)));
		}

		sleep_until(deadline.min(Instant::now() + Duration::from_millis(200))).await;
	}
}

// ============================================================================
// Redis Cluster Container Fixtures
// ============================================================================

/// Metadata for Redis Cluster container
///
/// Stores cluster container reference and initial node ports.
/// Used for cleanup and port tracking.
pub struct RedisClusterContainer {
	/// The running Redis Cluster container handle.
	pub container: ContainerAsync<GenericImage>,
	/// Initial 6 node ports (7000-7005 mapped to host ports)
	pub node_ports: Vec<u16>,
}

impl std::fmt::Debug for RedisClusterContainer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RedisClusterContainer")
			.field("node_ports", &self.node_ports)
			.field("container", &"<ContainerAsync>")
			.finish()
	}
}

/// Level 1: Acquire file lock for Redis Cluster initialization
///
/// Prevents concurrent cluster initialization across test processes.
/// Lock is held for the entire test duration.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::testcontainers::redis_cluster_lock;
/// use rstest::*;
///
/// #[rstest]
/// fn test_with_cluster_lock(redis_cluster_lock: reinhardt_testkit::fixtures::FileLockGuard) {
///     // Lock ensures exclusive cluster access
/// }
/// ```
#[fixture]
pub fn redis_cluster_lock() -> FileLockGuard {
	let lock_path = std::env::temp_dir().join("reinhardt_redis_cluster.lock");
	FileLockGuard::new(lock_path).expect("Failed to acquire Redis cluster lock")
}

/// Level 2: Stop and remove any existing Redis Cluster container
///
/// Ensures clean state before starting new cluster.
/// Depends on: redis_cluster_lock
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::testcontainers::{redis_cluster_lock, redis_cluster_cleanup};
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_cleanup(
///     redis_cluster_lock: reinhardt_testkit::fixtures::FileLockGuard,
///     #[future] redis_cluster_cleanup: ()
/// ) {
///     let _ = redis_cluster_cleanup.await;
///     // Old cluster is now removed
/// }
/// ```
#[fixture]
pub async fn redis_cluster_cleanup(_redis_cluster_lock: FileLockGuard) {
	// DISABLED: This cleanup was stopping containers from other parallel tests
	// TestContainers automatically cleans up containers when they are dropped

	// // Try to find and stop any existing Redis cluster container
	// // Use docker CLI to find container by name pattern
	// let output = tokio::process::Command::new("docker")
	// 	.args([
	// 		"ps",
	// 		"-a",
	// 		"--filter",
	// 		"ancestor=neohq/redis-cluster:latest",
	// 		"--format",
	// 		"{{.ID}}",
	// 	])
	// 	.output()
	// 	.await;
	//
	// if let Ok(output) = output {
	// 	let container_ids = String::from_utf8_lossy(&output.stdout);
	// 	for container_id in container_ids.lines() {
	// 		let container_id = container_id.trim();
	// 		if !container_id.is_empty() {
	// 			eprintln!(
	// 				"Stopping existing Redis cluster container: {}",
	// 				container_id
	// 			);
	// 			let _ = tokio::process::Command::new("docker")
	// 				.args(["stop", container_id])
	// 				.output()
	// 				.await;
	// 			let _ = tokio::process::Command::new("docker")
	// 				.args(["rm", container_id])
	// 				.output()
	// 				.await;
	// 		}
	// 	}
	// }
	//
	// // Small delay to ensure complete cleanup
}

/// Helper function to attempt Redis cluster container start
async fn try_start_redis_cluster(
	base_port: u16,
) -> Result<(ContainerAsync<GenericImage>, Vec<u16>), Box<dyn std::error::Error>> {
	let cluster = GenericImage::new("grokzen/redis-cluster", "7.0.10")
		.with_wait_for(WaitFor::message_on_stdout("Cluster state changed: ok"))
		.with_startup_timeout(std::time::Duration::from_secs(600))
		.with_env_var("IP", "0.0.0.0")
		.with_env_var("INITIAL_PORT", base_port.to_string())
		.with_mapped_port(base_port, ContainerPort::Tcp(base_port))
		.with_mapped_port(base_port + 1, ContainerPort::Tcp(base_port + 1))
		.with_mapped_port(base_port + 2, ContainerPort::Tcp(base_port + 2))
		.with_mapped_port(base_port + 3, ContainerPort::Tcp(base_port + 3))
		.with_mapped_port(base_port + 4, ContainerPort::Tcp(base_port + 4))
		.with_mapped_port(base_port + 5, ContainerPort::Tcp(base_port + 5))
		.start()
		.await?;

	let node_ports = vec![
		base_port,
		base_port + 1,
		base_port + 2,
		base_port + 3,
		base_port + 4,
		base_port + 5,
	];

	// Wait for all Redis services to start listening
	let max_retries = 30;
	for retry in 0..max_retries {
		let mut all_ready = true;
		for &port in &node_ports {
			if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
				.await
				.is_err()
			{
				all_ready = false;
				break;
			}
		}

		if all_ready {
			eprintln!("All Redis cluster ports ready after {} attempts", retry + 1);
			return Ok((cluster, node_ports));
		}
	}

	Err(format!(
		"Redis cluster ports not ready after {} retries. Ports: {:?}",
		max_retries, node_ports
	)
	.into())
}

/// Start a Redis Cluster container and wait until all node ports are ready
#[fixture]
pub async fn redis_cluster_ports_ready(
	#[future] redis_cluster_cleanup: (),
	#[future] redis_cluster_base_port: u16,
) -> (ContainerAsync<GenericImage>, Vec<u16>) {
	let _ = redis_cluster_cleanup.await;
	let mut base_port = redis_cluster_base_port.await;

	// IMPORTANT: Use fixed port mapping (host port = container port)
	//
	// Why fixed ports are necessary:
	// 1. grokzen/redis-cluster runs 6 Redis instances in a single container
	// 2. ClusterClient executes CLUSTER SLOTS to discover topology
	// 3. CLUSTER SLOTS returns internal ports that cannot be overridden
	// 4. redis-rs ClusterClient has no configuration to override port mapping
	// 5. Therefore, host ports MUST match container ports for ClusterClient to work
	//
	// Port selection is handled by redis_cluster_base_port fixture:
	// - Automatically finds 6 consecutive available ports
	// - Checks REDIS_CLUSTER_BASE_PORT env var (default: 17000)
	// - Falls back to alternatives (27000, 37000, 47000) if occupied
	// - Searches 20000-60000 range if all predefined candidates are taken
	// - This ensures tests never fail due to port conflicts

	const MAX_PORT_RETRIES: usize = 5;
	const PORT_INCREMENT: u16 = 1000;

	for retry in 0..MAX_PORT_RETRIES {
		if retry > 0 {
			eprintln!(
				"Retrying Redis cluster start with port {} (attempt {}/{})",
				base_port,
				retry + 1,
				MAX_PORT_RETRIES
			);
		} else {
			eprintln!(
				"Using Redis Cluster port range: {}-{}",
				base_port,
				base_port + 5
			);
		}

		// Verify ports are still available just before container start
		if !is_port_range_available(base_port).await {
			eprintln!(
				"Port range {}-{} became unavailable, trying next range",
				base_port,
				base_port + 5
			);
			base_port += PORT_INCREMENT;
			continue;
		}

		match try_start_redis_cluster(base_port).await {
			Ok((container, node_ports)) => {
				eprintln!(
					"Redis cluster started successfully on ports {:?}",
					node_ports
				);
				return (container, node_ports);
			}
			Err(e) => {
				eprintln!("Failed to start Redis cluster on port {}: {}", base_port, e);

				// If port allocation error, try next port range
				if e.to_string().contains("port is already allocated")
					|| e.to_string().contains("address already in use")
				{
					base_port += PORT_INCREMENT;
					continue;
				}

				// For other errors, panic immediately
				panic!("Failed to start Redis cluster (non-port error): {}", e);
			}
		}
	}

	panic!(
		"Failed to start Redis cluster after {} attempts. Last port tried: {}",
		MAX_PORT_RETRIES, base_port
	);
}

/// Level 4: Wait for cluster initialization (CLUSTER INFO shows cluster_state:ok)
///
/// Polls CLUSTER INFO until cluster is fully initialized.
/// Depends on: redis_cluster_ports_ready
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::testcontainers::redis_cluster_container;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_cluster_ready(
///     redis_cluster_lock: reinhardt_testkit::fixtures::FileLockGuard,
///     #[future] redis_cluster_cleanup: (),
///     #[future] redis_cluster_ports_ready: (ContainerAsync<GenericImage>, Vec<u16>),
///     #[future] redis_cluster_container: reinhardt_testkit::fixtures::RedisClusterContainer
/// ) {
///     let container = redis_cluster_container.await;
///     assert_eq!(container.node_ports.len(), 6);
/// }
/// ```
#[fixture]
pub async fn redis_cluster_container(
	#[future] redis_cluster_ports_ready: (ContainerAsync<GenericImage>, Vec<u16>),
) -> RedisClusterContainer {
	let (cluster, node_ports) = redis_cluster_ports_ready.await;

	// WaitFor condition already confirmed "Cluster state changed: ok"
	// No retry needed - just return the container
	eprintln!("Redis cluster ready with ports: {:?}", node_ports);

	RedisClusterContainer {
		container: cluster,
		node_ports,
	}
}

/// Level 5: Complete Redis Cluster fixture with connection
///
/// Provides initialized cluster container + working redis::cluster::ClusterClient.
/// Depends on: redis_cluster_container
///
/// This is the top-level fixture you should use in most tests.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_testkit::fixtures::redis_cluster;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_redis_cluster(
///     redis_cluster_lock: reinhardt_testkit::fixtures::FileLockGuard,
///     #[future] redis_cluster_cleanup: (),
///     #[future] redis_cluster_ports_ready: (ContainerAsync<GenericImage>, Vec<u16>),
///     #[future] redis_cluster_container: reinhardt_testkit::fixtures::RedisClusterContainer,
///     #[future] redis_cluster: (
///         reinhardt_testkit::fixtures::RedisClusterContainer,
///         Arc<redis::cluster::ClusterClient>,
///         Vec<String>
///     )
/// ) {
///     let (container, client, nodes) = redis_cluster.await;
///     let mut conn = client.get_async_connection().await.unwrap();
///     redis::cmd("SET").arg("key").arg("value").query_async::<()>(&mut conn).await.unwrap();
/// }
/// ```
#[fixture]
pub async fn redis_cluster(
	#[future] redis_cluster_container: RedisClusterContainer,
) -> (
	RedisClusterContainer,
	Arc<redis::cluster::ClusterClient>,
	Vec<String>,
) {
	let container = redis_cluster_container.await;

	// Build cluster node URLs
	let cluster_nodes: Vec<String> = container
		.node_ports
		.iter()
		.map(|&port| format!("redis://127.0.0.1:{}", port))
		.collect();

	// Create cluster client
	let client = redis::cluster::ClusterClient::new(cluster_nodes.clone())
		.expect("Failed to create cluster client");

	// Verify cluster connection works
	let mut conn = client
		.get_async_connection()
		.await
		.expect("Failed to connect to cluster");

	// Test basic operation
	redis::cmd("PING")
		.query_async::<String>(&mut conn)
		.await
		.expect("Failed to PING cluster");

	eprintln!("Redis cluster connection verified");

	(container, Arc::new(client), cluster_nodes)
}

/// Lightweight Redis Cluster fixture
///
/// Returns cluster client, node URLs, and the container reference.
/// The container must be kept alive for the duration of the test to prevent
/// premature cleanup of the Redis cluster.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::testcontainers::redis_cluster_client;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_redis(
///     #[future] redis_cluster_client: (
///         Arc<redis::cluster::ClusterClient>,
///         Vec<String>,
///         RedisClusterContainer,
///     )
/// ) {
///     let (client, _nodes, _container) = redis_cluster_client.await;
///     let mut conn = client.get_async_connection().await.unwrap();
///     redis::cmd("SET").arg("key").arg("value").query_async::<()>(&mut conn).await.unwrap();
/// }
/// ```
#[fixture]
pub async fn redis_cluster_client(
	#[future] redis_cluster_container: RedisClusterContainer,
) -> (
	Arc<redis::cluster::ClusterClient>,
	Vec<String>,
	RedisClusterContainer,
) {
	let container = redis_cluster_container.await;

	// Build cluster node URLs
	let cluster_nodes: Vec<String> = container
		.node_ports
		.iter()
		.map(|&port| format!("redis://127.0.0.1:{}", port))
		.collect();

	// Create cluster client
	let client = redis::cluster::ClusterClient::new(cluster_nodes.clone())
		.expect("Failed to create cluster client");

	// Verify cluster connection works
	let mut conn = client
		.get_async_connection()
		.await
		.expect("Failed to connect to cluster");

	// Test basic operation
	redis::cmd("PING")
		.query_async::<String>(&mut conn)
		.await
		.expect("Failed to PING cluster");

	eprintln!("Redis cluster client created");

	// Return container to keep it alive during the test
	(Arc::new(client), cluster_nodes, container)
}

/// Ultra-lightweight Redis Cluster URLs fixture
///
/// Returns only cluster node URLs, completely avoiding container types.
/// This is the safest fixture for tests that don't need container lifecycle control.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::testcontainers::redis_cluster_urls;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_redis(#[future] redis_cluster_urls: Vec<String>) {
///     let urls = redis_cluster_urls.await;
///     // Use urls to create cache or client
/// }
/// ```
#[fixture]
pub async fn redis_cluster_urls(
	#[future] redis_cluster_container: RedisClusterContainer,
) -> (Vec<String>, RedisClusterContainer) {
	let container = redis_cluster_container.await;

	// Build cluster node URLs from health-checked container
	// redis_cluster_container already verified CLUSTER INFO shows cluster_state:ok
	let cluster_nodes: Vec<String> = container
		.node_ports
		.iter()
		.map(|&port| format!("redis://127.0.0.1:{}", port))
		.collect();

	// Return both URLs and container to keep container alive during test
	(cluster_nodes, container)
}

/// Alternative Redis Cluster fixture without composable dependencies
///
/// This fixture provides a complete Redis Cluster setup in a single fixture,
/// without requiring explicit declaration of intermediate dependency fixtures.
/// Internally manages file locking and cleanup.
///
/// Use this when you want a simpler test setup without the 5-level composable pattern.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::testcontainers::redis_cluster_fixture;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_simple_cluster(
///     #[future] redis_cluster_fixture: (
///         reinhardt_testkit::fixtures::RedisClusterContainer,
///         Arc<redis::cluster::ClusterClient>,
///         Vec<String>
///     )
/// ) {
///     let (_container, client, _nodes) = redis_cluster_fixture.await;
///     let mut conn = client.get_async_connection().await.unwrap();
///     redis::cmd("SET").arg("key").arg("value").query_async::<()>(&mut conn).await.unwrap();
/// }
/// ```
#[fixture]
pub async fn redis_cluster_fixture() -> (
	RedisClusterContainer,
	Arc<redis::cluster::ClusterClient>,
	Vec<String>,
) {
	// Level 1: Acquire lock
	let _lock = {
		let lock_path = std::env::temp_dir().join("reinhardt_redis_cluster.lock");
		FileLockGuard::new(lock_path).expect("Failed to acquire Redis cluster lock")
	};

	// Level 2: Cleanup existing containers
	{
		let output = tokio::process::Command::new("docker")
			.args([
				"ps",
				"-a",
				"--filter",
				"ancestor=neohq/redis-cluster:latest",
				"--format",
				"{{.ID}}",
			])
			.output()
			.await;

		if let Ok(output) = output {
			let container_ids = String::from_utf8_lossy(&output.stdout);
			for container_id in container_ids.lines() {
				let container_id = container_id.trim();
				if !container_id.is_empty() {
					eprintln!(
						"Stopping existing Redis cluster container: {}",
						container_id
					);
					let _ = tokio::process::Command::new("docker")
						.args(["stop", container_id])
						.output()
						.await;
					let _ = tokio::process::Command::new("docker")
						.args(["rm", container_id])
						.output()
						.await;
				}
			}
		}
	}

	// Level 3: Start container and wait for ports
	let (cluster, node_ports) = {
		use testcontainers::core::IntoContainerPort;

		let cluster = GenericImage::new("neohq/redis-cluster", "latest")
			.with_exposed_port(7000.tcp())
			.with_exposed_port(7001.tcp())
			.with_exposed_port(7002.tcp())
			.with_exposed_port(7003.tcp())
			.with_exposed_port(7004.tcp())
			.with_exposed_port(7005.tcp())
			.with_wait_for(WaitFor::message_on_stdout("[OK] All 16384 slots covered."))
			.with_startup_timeout(std::time::Duration::from_secs(600))
			.start()
			.await
			.expect("Failed to start Redis cluster container");

		let node_ports = vec![
			cluster
				.get_host_port_ipv4(7000)
				.await
				.expect("Failed to get port for node 7000"),
			cluster
				.get_host_port_ipv4(7001)
				.await
				.expect("Failed to get port for node 7001"),
			cluster
				.get_host_port_ipv4(7002)
				.await
				.expect("Failed to get port for node 7002"),
			cluster
				.get_host_port_ipv4(7003)
				.await
				.expect("Failed to get port for node 7003"),
			cluster
				.get_host_port_ipv4(7004)
				.await
				.expect("Failed to get port for node 7004"),
			cluster
				.get_host_port_ipv4(7005)
				.await
				.expect("Failed to get port for node 7005"),
		];

		// Wait for all ports to be accessible
		let max_retries = 30;
		for retry in 0..max_retries {
			let mut all_ready = true;
			for &port in &node_ports {
				if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
					.await
					.is_err()
				{
					all_ready = false;
					break;
				}
			}

			if all_ready {
				eprintln!("All Redis cluster ports ready after {} attempts", retry + 1);
				break;
			}

			if retry == max_retries - 1 {
				panic!(
					"Redis cluster ports not ready after {} retries. Ports: {:?}",
					max_retries, node_ports
				);
			}
		}

		(cluster, node_ports)
	};

	// Level 4: Wait for cluster initialization
	{
		let max_retries = 60;
		for retry in 0..max_retries {
			let client_result = redis::Client::open(format!("redis://127.0.0.1:{}", node_ports[0]));

			if let Ok(client) = client_result
				&& let Ok(mut conn) = client.get_multiplexed_async_connection().await
				&& let Ok(info) = redis::cmd("CLUSTER")
					.arg("INFO")
					.query_async::<String>(&mut conn)
					.await && info.contains("cluster_state:ok")
			{
				eprintln!(
					"Redis cluster fully initialized after {} attempts",
					retry + 1
				);
				break;
			}

			if retry == max_retries - 1 {
				panic!(
					"Redis cluster not initialized after {} retries. Ports: {:?}",
					max_retries, node_ports
				);
			}
		}
	}

	// Level 5: Create client and verify connection
	let cluster_nodes: Vec<String> = node_ports
		.iter()
		.map(|&port| format!("redis://127.0.0.1:{}", port))
		.collect();

	let client = redis::cluster::ClusterClient::new(cluster_nodes.clone())
		.expect("Failed to create cluster client");

	let mut conn = client
		.get_async_connection()
		.await
		.expect("Failed to connect to cluster");

	redis::cmd("PING")
		.query_async::<String>(&mut conn)
		.await
		.expect("Failed to PING cluster");

	eprintln!("Redis cluster connection verified");

	let container = RedisClusterContainer {
		container: cluster,
		node_ports,
	};

	(container, Arc::new(client), cluster_nodes)
}

// ============================================================================
// MongoDB Container Fixture
// ============================================================================

async fn try_start_mongodb_container()
-> Result<(ContainerAsync<GenericImage>, String, u16), Box<dyn std::error::Error>> {
	use testcontainers::core::IntoContainerPort;

	let mongo = GenericImage::new("mongo", "7.0")
		.with_exposed_port(27017.tcp())
		.with_wait_for(WaitFor::message_on_stdout("Waiting for connections"))
		.with_startup_timeout(std::time::Duration::from_secs(60))
		.start()
		.await?;

	let port = mongo.get_host_port_ipv4(27017).await?;
	let connection_string = format!("mongodb://127.0.0.1:{}", port);

	Ok((mongo, connection_string, port))
}

/// Fixture providing a MongoDB container
///
/// Starts a MongoDB 7.0 container for testing document operations.
#[fixture]
pub async fn mongodb_container() -> (ContainerAsync<GenericImage>, String, u16) {
	const MAX_RETRIES: u32 = 3;
	const RETRY_DELAY_MS: u64 = 2000;

	let mut last_error = None;

	for attempt in 0..MAX_RETRIES {
		match try_start_mongodb_container().await {
			Ok(result) => return result,
			Err(e) => {
				eprintln!(
					"MongoDB container start attempt {} of {} failed: {:?}",
					attempt + 1,
					MAX_RETRIES,
					e
				);
				last_error = Some(e);

				if attempt < MAX_RETRIES - 1 {
					tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
				}
			}
		}
	}

	panic!(
		"Failed to start MongoDB container after {} attempts: {:?}",
		MAX_RETRIES, last_error
	);
}

// ============================================================================
// LocalStack Container Fixture
// ============================================================================

/// Fixture providing LocalStack container for AWS service mocking
///
/// Starts a LocalStack container with S3, DynamoDB, and other AWS services.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::localstack_fixture;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_localstack(
///     #[future] localstack_fixture: (ContainerAsync<GenericImage>, u16, String)
/// ) {
///     let (_container, port, endpoint) = localstack_fixture.await;
///     // Use endpoint for AWS SDK configuration
/// }
/// ```
#[fixture]
pub async fn localstack_fixture() -> (ContainerAsync<GenericImage>, u16, String) {
	use testcontainers::core::IntoContainerPort;

	let localstack = GenericImage::new("localstack/localstack", "latest")
		.with_exposed_port(4566.tcp())
		.with_wait_for(WaitFor::message_on_stdout("Ready."))
		.with_env_var("SERVICES", "s3,dynamodb")
		.start()
		.await
		.expect("Failed to start LocalStack container");

	let port = localstack
		.get_host_port_ipv4(4566)
		.await
		.expect("Failed to get LocalStack port");

	let endpoint = format!("http://localhost:{}", port);

	(localstack, port, endpoint)
}

// ============================================================================
// Migration Application Fixtures
// ============================================================================

/// Fixture: PostgreSQL container with migrations from a MigrationProvider
///
/// This function starts a PostgreSQL container, applies migrations from the
/// specified `MigrationProvider`, and returns a ready-to-use connection.
///
/// Unlike `postgres_with_migrations`, this function uses compile-time migration
/// collection via the `MigrationProvider` trait, which is necessary because Rust
/// cannot dynamically load code at runtime.
///
/// # Type Parameters
/// * `P` - A type implementing `MigrationProvider`
///
/// # Returns
/// * `(ContainerAsync<GenericImage>, MigrationDatabase)` - Container and guarded ORM connection
///
/// # Example
///
/// ```no_run
/// # use reinhardt_testkit::fixtures::postgres_with_migrations_from;
/// # use reinhardt_db::migrations::MigrationProvider;
/// # #[tokio::main]
/// # async fn main() {
/// // In your app's migrations.rs, use collect_migrations! macro
/// // pub mod _0001_initial;
/// // pub mod _0002_add_field;
///
/// // collect_migrations!(
/// //     app_label = "myapp",
/// //     _0001_initial,
/// //     _0002_add_field,
/// // );
///
/// // Migrations are automatically registered in global registry via linkme
///
/// // #[tokio::test]
/// // async fn test_with_migrations() {
/// //     let (container, db) = postgres_with_migrations_from::<MyappMigrations>().await;
/// //     // Database has all migrations applied from MyappMigrations provider
/// //     let result = db.fetch_all("SELECT * FROM my_table", vec![]).await;
/// //     assert!(result.is_ok());
/// // }
/// # }
/// ```
#[cfg(feature = "testcontainers")]
pub async fn postgres_with_migrations_from<P: reinhardt_db::migrations::MigrationProvider>()
-> Result<(ContainerAsync<GenericImage>, MigrationDatabase), Box<dyn std::error::Error>> {
	let (container, _pool, _port, url) = postgres_container().await;
	let database = apply_postgres_migrations_from::<P>(&url).await?;
	Ok((container, database))
}

/// Apply a `MigrationProvider` to an already-started PostgreSQL database URL.
///
/// Returns a guarded ORM connection. The caller must keep the container guard
/// alive until the returned connection is dropped. Available with the
/// `testcontainers` feature for native database tests (P0).
///
/// # Errors
///
/// Returns an error if connecting, applying migrations, or registering the
/// connection fails.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_db::migrations::{Migration, MigrationProvider};
/// use reinhardt_testkit::{
///     PostgresContainerConfig, apply_postgres_migrations_from, start_postgres_container,
/// };
///
/// struct AppMigrations;
/// impl MigrationProvider for AppMigrations {
///     fn migrations() -> Vec<Migration> { Vec::new() }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let (_container, _pool, _port, url) = start_postgres_container(
///     PostgresContainerConfig::default().database("test_db"),
/// ).await;
/// let db = apply_postgres_migrations_from::<AppMigrations>(&url).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "testcontainers")]
pub async fn apply_postgres_migrations_from<P: reinhardt_db::migrations::MigrationProvider>(
	database_url: &str,
) -> Result<MigrationDatabase, Box<dyn std::error::Error>> {
	use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
	use reinhardt_db::migrations::executor::DatabaseMigrationExecutor;
	use reinhardt_db::orm::DatabaseConnectionLease;

	// Connect to database
	let owner = BackendsConnection::connect_postgres(database_url)
		.await
		.map_err(|e| format!("Failed to connect to PostgreSQL for migrations: {}", e))?;

	// Get migrations from provider
	let migrations = P::migrations();

	if !migrations.is_empty() {
		let mut executor = DatabaseMigrationExecutor::new(owner.clone());
		executor
			.apply_migrations(&migrations)
			.await
			.map_err(|e| format!("Failed to apply migrations: {}", e))?;
	}

	let connection_lease = DatabaseConnectionLease::register(owner)?;
	Ok(MigrationDatabase {
		connection: connection_lease.handle(),
		_connection_lease: connection_lease,
	})
}

/// Fixture: MySQL container (base fixture)
///
/// Starts a MySQL 8.0 container and provides a connection pool.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::mysql_container;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_mysql(
///     #[future] mysql_container: (ContainerAsync<GenericImage>, Arc<sqlx::MySqlPool>, u16, String)
/// ) {
///     let (_container, pool, _port, url) = mysql_container.await;
///     let result = sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await;
///     assert!(result.is_ok());
/// }
/// ```
#[fixture]
#[cfg(feature = "testcontainers")]
pub async fn mysql_container() -> (
	ContainerAsync<GenericImage>,
	Arc<sqlx::MySqlPool>,
	u16,
	String,
) {
	use testcontainers::core::IntoContainerPort;

	let mysql = GenericImage::new("mysql", "8.0")
		.with_exposed_port(3306.tcp())
		.with_wait_for(WaitFor::message_on_stderr(
			"port: 3306  MySQL Community Server",
		))
		.with_startup_timeout(std::time::Duration::from_secs(120))
		.with_env_var("MYSQL_ROOT_PASSWORD", "test")
		.with_env_var("MYSQL_DATABASE", "test_db")
		.start()
		.await
		.expect("Failed to start MySQL container");

	// Wait briefly before first port query to ensure container networking is ready
	// Increased from 200ms to 500ms for better reliability under heavy load
	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

	// Retry getting port with exponential backoff
	let mut port_retry = 0;
	let max_port_retries = 7; // Increased from 5 for better reliability under load
	let port = loop {
		match mysql.get_host_port_ipv4(3306).await {
			Ok(p) => break p,
			Err(e) if port_retry < max_port_retries => {
				port_retry += 1;
				let delay = tokio::time::Duration::from_millis(200 * 2_u64.pow(port_retry));
				eprintln!(
					"MySQL port query attempt {} of {} failed: {:?}",
					port_retry, max_port_retries, e
				);
				tokio::time::sleep(delay).await;
			}
			Err(e) => panic!(
				"Failed to get MySQL port after {} retries: {}",
				max_port_retries, e
			),
		}
	};

	let database_url = format!("mysql://root:test@localhost:{}/test_db", port);

	// Get pool configuration from environment variables
	let (max_conns, timeout_secs) = get_pool_config();

	// Retry connection to MySQL with exponential backoff
	let mut retry_count = 0;
	let max_retries = 7; // Increased from 5 for better reliability in CI environments

	// Wait briefly before first connection to ensure container is fully ready
	tokio::time::sleep(std::time::Duration::from_millis(500)).await;

	let pool = loop {
		match sqlx::mysql::MySqlPoolOptions::new()
			.max_connections(max_conns)
			.min_connections(1)
			.acquire_timeout(std::time::Duration::from_secs(timeout_secs))
			.idle_timeout(std::time::Duration::from_secs(600)) // Increase from 30s for sqlx v0.7+ compatibility
			.max_lifetime(std::time::Duration::from_secs(1800)) // Increase from 120s for long-running tests
			.test_before_acquire(false) // sqlx v0.7+ bug workaround (issue #2885, #3241)
			.connect(&database_url)
			.await
		{
			Ok(pool) => {
				// Verify wire protocol is working correctly
				match sqlx::query("SELECT 1").fetch_one(&pool).await {
					Ok(_) => break pool,
					Err(e) if retry_count < max_retries => {
						eprintln!(
							"MySQL health check attempt {} of {} failed: {:?}",
							retry_count + 1,
							max_retries,
							e
						);
						retry_count += 1;
						let delay = std::time::Duration::from_millis(200 * 2_u64.pow(retry_count));
						tokio::time::sleep(delay).await;
						continue;
					}
					Err(e) => panic!(
						"MySQL health check failed after {} retries: {}",
						max_retries, e
					),
				}
			}
			Err(e) if retry_count < max_retries => {
				eprintln!(
					"MySQL connection attempt {} of {} failed: {:?}",
					retry_count + 1,
					max_retries,
					e
				);
				retry_count += 1;
				let delay = std::time::Duration::from_millis(200 * 2_u64.pow(retry_count));
				tokio::time::sleep(delay).await;
			}
			Err(e) => panic!(
				"Failed to connect to MySQL after {} retries: {}",
				max_retries, e
			),
		}
	};

	(mysql, Arc::new(pool), port, database_url)
}

/// MySQL container with migrations from a MigrationProvider
///
/// This function starts a MySQL container, applies migrations from the
/// specified `MigrationProvider`, and returns a ready-to-use connection.
///
/// # Type Parameters
/// * `P` - A type implementing `MigrationProvider`
///
/// # Returns
/// * `(ContainerAsync<GenericImage>, MigrationDatabase)` - Container and guarded ORM connection
///
/// # Example
///
/// ```no_run
/// # use reinhardt_testkit::fixtures::mysql_with_migrations_from;
/// # use reinhardt_db::migrations::MigrationProvider;
/// # #[tokio::main]
/// # async fn main() {
/// // In your app's migrations.rs, use collect_migrations! macro
/// // pub mod _0001_initial;
///
/// // collect_migrations!(
/// //     app_label = "myapp",
/// //     _0001_initial,
/// // );
///
/// // Migrations are automatically registered in global registry via linkme
///
/// // #[tokio::test]
/// // async fn test_with_migrations() {
/// //     let (container, db) = mysql_with_migrations_from::<MyappMigrations>().await;
/// //     // Database has all migrations applied from MyappMigrations provider
/// // }
/// # }
/// ```
#[cfg(feature = "testcontainers")]
pub async fn mysql_with_migrations_from<P: reinhardt_db::migrations::MigrationProvider>()
-> (ContainerAsync<GenericImage>, MigrationDatabase) {
	use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
	use reinhardt_db::migrations::executor::DatabaseMigrationExecutor;
	use reinhardt_db::orm::DatabaseConnectionLease;

	// Start MySQL container
	let (container, _pool, _port, url) = mysql_container().await;

	// Connect to database
	let owner = BackendsConnection::connect_mysql(&url)
		.await
		.expect("Failed to connect to MySQL for migrations");

	// Get migrations from provider
	let migrations = P::migrations();

	if !migrations.is_empty() {
		let mut executor = DatabaseMigrationExecutor::new(owner.clone());
		executor
			.apply_migrations(&migrations)
			.await
			.expect("Failed to apply migrations");
	}

	let connection_lease =
		DatabaseConnectionLease::register(owner).expect("Failed to register connection");
	(
		container,
		MigrationDatabase {
			connection: connection_lease.handle(),
			_connection_lease: connection_lease,
		},
	)
}

/// SQLite in-memory database with migrations from a MigrationProvider
///
/// This function creates an SQLite in-memory database, applies migrations from the
/// specified `MigrationProvider`, and returns a ready-to-use connection.
///
/// # Type Parameters
/// * `P` - A type implementing `MigrationProvider`
///
/// # Returns
/// * `MigrationDatabase` - Guarded ORM connection (no container needed for SQLite)
///
/// # Example
///
/// ```no_run
/// # use reinhardt_testkit::fixtures::sqlite_with_migrations_from;
/// # use reinhardt_db::migrations::MigrationProvider;
/// # #[tokio::main]
/// # async fn main() {
/// // In your app's migrations.rs, use collect_migrations! macro
/// // pub mod _0001_initial;
///
/// // collect_migrations!(
/// //     app_label = "myapp",
/// //     _0001_initial,
/// // );
///
/// // Migrations are automatically registered in global registry via linkme
///
/// // #[tokio::test]
/// // async fn test_with_migrations() {
/// //     let db = sqlite_with_migrations_from::<MyappMigrations>().await;
/// //     // Database has all migrations applied from MyappMigrations provider
/// // }
/// # }
/// ```
#[cfg(feature = "testcontainers")]
pub async fn sqlite_with_migrations_from<P: reinhardt_db::migrations::MigrationProvider>()
-> MigrationDatabase {
	use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
	use reinhardt_db::migrations::executor::DatabaseMigrationExecutor;
	use reinhardt_db::orm::DatabaseConnectionLease;

	let database_url = "sqlite::memory:";

	// Connect to database
	let owner = BackendsConnection::connect_sqlite(database_url)
		.await
		.expect("Failed to connect to SQLite for migrations");

	// Get migrations from provider
	let migrations = P::migrations();

	if !migrations.is_empty() {
		let mut executor = DatabaseMigrationExecutor::new(owner.clone());
		executor
			.apply_migrations(&migrations)
			.await
			.expect("Failed to apply migrations");
	}

	let connection_lease =
		DatabaseConnectionLease::register(owner).expect("Failed to register connection");
	MigrationDatabase {
		connection: connection_lease.handle(),
		_connection_lease: connection_lease,
	}
}

/// Helper function for creating a PostgreSQL container with migrations
/// loaded from a filesystem directory via `FilesystemSource`.
///
/// This is the recommended approach for loading migrations in tests:
/// - Consistent with `manage migrate` behavior
/// - Does not require `collect_migrations!` macro registration
/// - Works reliably in Cargo workspaces when using `env!("CARGO_MANIFEST_DIR")`
///
/// # Arguments
///
/// * `migrations_dir` - Path to the root directory containing migration files
///   organized as `<app_label>/<name>.rs`
///
/// # Example
///
/// ```no_run
/// use reinhardt_testkit::fixtures::postgres_with_migrations_from_dir;
/// use std::sync::Arc;
///
/// #[tokio::test]
/// async fn test_with_filesystem_migrations() {
///     let migrations_dir = format!("{}/migrations", env!("CARGO_MANIFEST_DIR"));
///     let (_container, db) = postgres_with_migrations_from_dir(&migrations_dir)
///         .await
///         .unwrap();
///     // All migrations from the directory are applied
/// }
/// ```
///
#[cfg(feature = "testcontainers")]
pub async fn postgres_with_migrations_from_dir(
	migrations_dir: impl AsRef<std::path::Path>,
) -> Result<(ContainerAsync<GenericImage>, MigrationDatabase), Box<dyn std::error::Error>> {
	let (container, _pool, _port, url) = postgres_container().await;
	let database = apply_postgres_migrations_from_dir(&url, migrations_dir).await?;
	Ok((container, database))
}

/// Apply filesystem migrations to an already-started PostgreSQL database URL.
///
/// The directory contains migration files organized as `<app_label>/<name>.rs`.
/// Like `postgres_with_migrations_from_dir`, this initializes the ORM global
/// connection for model access. Callers must serialize tests sharing that global
/// state and keep their container guard alive while using the returned connection.
/// Available with the `testcontainers` feature for native database tests (P0).
///
/// # Errors
///
/// Returns an error if connecting, loading or applying migrations, initializing
/// the ORM global connection, or registering the guarded connection fails.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::{
///     PostgresContainerConfig, apply_postgres_migrations_from_dir, start_postgres_container,
/// };
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let (_container, _pool, _port, url) = start_postgres_container(
///     PostgresContainerConfig::default().database("test_db"),
/// ).await;
/// let migrations = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
/// let db = apply_postgres_migrations_from_dir(&url, migrations).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "testcontainers")]
pub async fn apply_postgres_migrations_from_dir(
	database_url: &str,
	migrations_dir: impl AsRef<std::path::Path>,
) -> Result<MigrationDatabase, Box<dyn std::error::Error>> {
	use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
	use reinhardt_db::migrations::FilesystemSource;
	use reinhardt_db::migrations::MigrationSource;
	use reinhardt_db::migrations::executor::DatabaseMigrationExecutor;
	use reinhardt_db::orm::DatabaseConnectionLease;

	// Connect to database
	let owner = BackendsConnection::connect_postgres(database_url)
		.await
		.map_err(|e| format!("Failed to connect to PostgreSQL for migrations: {}", e))?;

	// Load migrations from filesystem
	let source = FilesystemSource::new(migrations_dir);
	let migrations = source
		.all_migrations()
		.await
		.map_err(|e| format!("Failed to load migrations from filesystem: {}", e))?;

	if !migrations.is_empty() {
		let mut executor = DatabaseMigrationExecutor::new(owner.clone());
		executor
			.apply_migrations(&migrations)
			.await
			.map_err(|e| format!("Failed to apply migrations: {}", e))?;
	}

	// Initialize the ORM global database connection so that E2E tests
	// using ORM models can access the database without manual setup.
	reinhardt_db::orm::reinitialize_database(database_url)
		.await
		.map_err(|e| format!("Failed to initialize ORM global state: {}", e))?;

	let connection_lease = DatabaseConnectionLease::register(owner)?;
	Ok(MigrationDatabase {
		connection: connection_lease.handle(),
		_connection_lease: connection_lease,
	})
}

// ============================================================================
// RabbitMQ Container Fixtures
// ============================================================================

/// RabbitMQ container fixture for testing message queue operations
///
/// Returns a tuple of (container, port, url) where:
/// - container: The running RabbitMQ container instance
/// - port: The host port mapped to RabbitMQ's AMQP port (5672)
/// - url: AMQP connection URL (e.g., "amqp://localhost:55001/%2f")
///
/// # Example
///
/// ```rust
/// use reinhardt_testkit::fixtures::rabbitmq_container;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_rabbitmq(
///     #[future] rabbitmq_container: (ContainerAsync<GenericImage>, u16, String)
/// ) {
///     let (_container, port, url) = rabbitmq_container.await;
///     // Use RabbitMQ connection
/// }
/// ```
#[fixture]
pub async fn rabbitmq_container() -> (ContainerAsync<GenericImage>, u16, String) {
	const MAX_RETRIES: u32 = 3;
	const RETRY_DELAY_MS: u64 = 2000;

	let mut last_error = None;

	for attempt in 0..MAX_RETRIES {
		match try_start_rabbitmq_container().await {
			Ok(result) => return result,
			Err(e) => {
				eprintln!(
					"RabbitMQ container start attempt {} of {} failed: {:?}",
					attempt + 1,
					MAX_RETRIES,
					e
				);
				last_error = Some(e);

				if attempt < MAX_RETRIES - 1 {
					tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
				}
			}
		}
	}

	panic!(
		"Failed to start RabbitMQ container after {} attempts: {:?}",
		MAX_RETRIES, last_error
	);
}

async fn try_start_rabbitmq_container()
-> Result<(ContainerAsync<GenericImage>, u16, String), Box<dyn std::error::Error>> {
	use testcontainers::core::IntoContainerPort;

	let rabbitmq = GenericImage::new("rabbitmq", "3-management-alpine")
		.with_exposed_port(5672.tcp()) // AMQP port
		.with_exposed_port(15672.tcp()) // Management UI port
		.with_wait_for(WaitFor::message_on_stdout("Server startup complete"))
		.with_startup_timeout(std::time::Duration::from_secs(120))
		.start()
		.await?;

	// Retry getting port with exponential backoff
	let mut port_retry = 0;
	let max_port_retries = 5;
	let port = loop {
		match rabbitmq.get_host_port_ipv4(5672).await {
			Ok(p) => break p,
			Err(_) if port_retry < max_port_retries => {
				port_retry += 1;
				let delay = std::time::Duration::from_millis(100 * 2_u64.pow(port_retry));
				tokio::time::sleep(delay).await;
			}
			Err(e) => {
				return Err(Box::new(std::io::Error::other(format!(
					"Failed to get RabbitMQ port after {} retries: {}",
					max_port_retries, e
				))));
			}
		}
	};

	// RabbitMQ default vhost is "/" which needs to be URL-encoded as "%2f"
	let url = format!("amqp://localhost:{}/%2f", port);

	Ok((rabbitmq, port, url))
}

// ---------------------------------------------------------------------------
// Shared Kafka container (module-/process-scoped, amortized startup)
// ---------------------------------------------------------------------------

/// Process-wide shared `KafkaContainer`, lazily started on first use.
///
/// `KafkaContainer::new()` takes several seconds (image pull + KRaft startup
/// barrier). For test suites that exercise many small Kafka scenarios — e.g.
/// `kafka_error_paths` — paying that cost once per test binary is significantly
/// faster than starting a fresh container per `#[rstest]`.
///
/// The container handle is kept alive for the lifetime of the process via the
/// static `OnceCell`; testcontainers' `Drop` will tear it down when the test
/// binary exits.
///
/// **Caller responsibility:** Topic-name collisions across tests sharing the
/// same broker are NOT prevented by this fixture. Tests MUST generate unique
/// topic names per test (e.g. via `uuid::Uuid::new_v4()` or a counter).
#[cfg(feature = "testcontainers")]
static SHARED_KAFKA: tokio::sync::OnceCell<Arc<crate::containers::KafkaContainer>> =
	tokio::sync::OnceCell::const_new();

/// Return a process-wide shared `KafkaContainer`, starting it on first call.
///
/// Subsequent calls return clones of the same `Arc`, so the underlying broker
/// — and its mapped host port — is reused across all callers in the same test
/// binary.
///
/// See the `SHARED_KAFKA` static for the topic-collision caveat.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::shared_kafka_container;
///
/// # async fn doc() {
/// let kafka = shared_kafka_container().await;
/// let brokers = kafka.brokers();
/// # }
/// ```
#[cfg(feature = "testcontainers")]
pub async fn shared_kafka_container() -> Arc<crate::containers::KafkaContainer> {
	SHARED_KAFKA
		.get_or_init(|| async { Arc::new(crate::containers::KafkaContainer::new().await) })
		.await
		.clone()
}

/// rstest fixture that yields the process-wide shared `KafkaContainer`.
///
/// This is the rstest-idiomatic wrapper around [`shared_kafka_container`].
/// Use this when writing `#[rstest] #[tokio::test]` tests that need a Kafka
/// broker but want to amortize container startup across the whole test binary.
///
/// **Caller responsibility:** generate unique topic names per test — the
/// underlying broker is shared, so two tests using the same topic will see
/// each other's records.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::kafka_container;
/// use reinhardt_testkit::containers::KafkaContainer;
/// use rstest::*;
/// use std::sync::Arc;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_kafka(#[future] kafka_container: Arc<KafkaContainer>) {
///     let kafka = kafka_container.await;
///     let brokers = kafka.brokers();
///     // ... unique topic per test ...
/// }
/// ```
#[cfg(feature = "testcontainers")]
#[fixture]
pub async fn kafka_container() -> Arc<crate::containers::KafkaContainer> {
	shared_kafka_container().await
}

#[cfg(all(test, feature = "testcontainers"))]
mod tests {
	use super::*;
	use reinhardt_query::prelude::{Expr, PostgresQueryBuilder, Query, QueryStatementBuilder};
	use rstest::*;

	#[rstest]
	fn test_postgres_container_config_defaults() {
		// Arrange
		let config = PostgresContainerConfig::default();

		// Act
		let env = config.environment();
		let url = config.database_url(54321);

		// Assert
		assert_eq!(config.image_name, "postgres");
		assert_eq!(config.image_tag, "16-alpine");
		assert_eq!(config.password, None);
		assert_eq!(config.host_port, None);
		assert_eq!(config.command_args, Vec::<String>::new());
		assert_eq!(config.startup_timeout, std::time::Duration::from_secs(120));
		assert_eq!(
			format!("{:?}", config.wait_for),
			format!(
				"{:?}",
				WaitFor::message_on_stderr("database system is ready to accept connections")
			)
		);
		assert_eq!(
			env,
			std::collections::BTreeMap::from([
				("POSTGRES_USER", "postgres"),
				("POSTGRES_DB", "postgres"),
				("POSTGRES_HOST_AUTH_METHOD", "trust"),
			])
		);
		assert_eq!(
			url,
			"postgres://postgres@localhost:54321/postgres?sslmode=disable"
		);
	}

	#[rstest]
	#[case::trust(None)]
	#[case::password(Some("p@ss:/?#% word"))]
	fn test_postgres_container_config_credentials_override_env(#[case] password: Option<&str>) {
		// Arrange
		let mut config = PostgresContainerConfig::default()
			.user("test@user")
			.database("test/db")
			.env("POSTGRES_USER", "wrong_user")
			.env("POSTGRES_DB", "wrong_db")
			.env("POSTGRES_PASSWORD", "wrong_password")
			.env("POSTGRES_HOST_AUTH_METHOD", "wrong_auth")
			.env("TZ", "UTC")
			.env("TZ", "Asia/Tokyo");
		if let Some(password) = password {
			config = config.password(password);
		}

		// Act
		let env = config.environment();
		let url = config.database_url(54321);

		// Assert
		let mut expected = std::collections::BTreeMap::from([
			("POSTGRES_USER", "test@user"),
			("POSTGRES_DB", "test/db"),
			("TZ", "Asia/Tokyo"),
		]);
		let expected_url = if let Some(password) = password {
			expected.insert("POSTGRES_PASSWORD", password);
			"postgres://test%40user:p%40ss%3A%2F%3F%23%25%20word@localhost:54321/test%2Fdb?sslmode=disable"
		} else {
			expected.insert("POSTGRES_HOST_AUTH_METHOD", "trust");
			"postgres://test%40user@localhost:54321/test%2Fdb?sslmode=disable"
		};
		assert_eq!(env, expected);
		assert_eq!(url, expected_url);
	}

	#[rstest]
	fn test_postgres_container_config_builders() {
		// Arrange / Act
		let config = PostgresContainerConfig::default()
			.image("custom/postgres", "test-tag")
			.arg("-c")
			.arg("max_connections=400")
			.args(["-c", "log_statement=all"])
			.host_port(54321)
			.wait_for(WaitFor::Nothing)
			.startup_timeout(std::time::Duration::from_secs(90));

		// Assert
		assert_eq!(config.image_name, "custom/postgres");
		assert_eq!(config.image_tag, "test-tag");
		assert_eq!(
			config.command_args,
			["-c", "max_connections=400", "-c", "log_statement=all"]
		);
		assert_eq!(config.host_port, Some(54321));
		assert!(matches!(config.wait_for, WaitFor::Nothing));
		assert_eq!(config.startup_timeout, std::time::Duration::from_secs(90));
	}

	#[rstest]
	#[tokio::test]
	async fn test_postgres_container_with_defaults(
		#[future] postgres_container_with: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		// Arrange / Act
		let (_container, _pool, port, url) = postgres_container_with.await;

		// Assert
		assert_eq!(
			url,
			format!("postgres://postgres@localhost:{port}/postgres?sslmode=disable")
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_postgres_container_custom_command(
		#[with(PostgresContainerConfig::default().args(["-c", "max_connections=400"]))]
		#[future]
		postgres_container_with: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
	) {
		// Arrange
		let (_container, pool, _port, _url) = postgres_container_with.await;

		// Act: SHOW is a PostgreSQL diagnostic command, not a query-builder statement.
		let max_connections: String = sqlx::query_scalar("SHOW max_connections")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to read max_connections");

		// Assert
		assert_eq!(max_connections, "400");
	}

	#[rstest]
	#[tokio::test]
	async fn test_postgres_container_password_credentials(
		#[with(PostgresContainerConfig::default()
			.user("test@user")
			.password("p@ss:/?#% word")
			.database("test/db")
			.env("POSTGRES_USER", "wrong_user")
			.env("POSTGRES_DB", "wrong_db")
			.env("POSTGRES_PASSWORD", "wrong_password")
			.env("POSTGRES_HOST_AUTH_METHOD", "trust"))]
		#[future]
		postgres_container_with: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
	) {
		use sqlx::Connection;

		// Arrange
		let (_container, pool, port, url) = postgres_container_with.await;
		let query = Query::select()
			.expr(Expr::cust("current_user"))
			.expr(Expr::cust("current_database()"))
			.to_string(PostgresQueryBuilder);
		let wrong_password = url
			.parse::<sqlx::postgres::PgConnectOptions>()
			.expect("Invalid connection URL")
			.password("wrong_password");

		// Act
		let credentials: (String, String) = sqlx::query_as(&query)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to read PostgreSQL credentials");
		let error = sqlx::PgConnection::connect_with(&wrong_password)
			.await
			.expect_err("Incorrect password must not authenticate");

		// Assert
		assert_eq!(credentials, ("test@user".into(), "test/db".into()));
		assert_eq!(
			url,
			format!(
				"postgres://test%40user:p%40ss%3A%2F%3F%23%25%20word@localhost:{port}/test%2Fdb?sslmode=disable"
			)
		);
		assert_eq!(
			error.as_database_error().unwrap().code().as_deref(),
			Some("28P01")
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_postgres_container_fixed_host_port() {
		// Arrange
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let requested_port = listener.local_addr().unwrap().port();
		// Release this reservation so Docker can bind the selected port.
		drop(listener);
		let config = PostgresContainerConfig::default().host_port(requested_port);

		// Act
		let (_container, _pool, port, url) = start_postgres_container(config).await;

		// Assert
		assert_eq!(port, requested_port);
		assert_eq!(
			url,
			format!("postgres://postgres@localhost:{requested_port}/postgres?sslmode=disable")
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_postgres_container_readiness_without_wait_condition() {
		// Arrange
		let config = PostgresContainerConfig::default().wait_for(WaitFor::Nothing);
		let query = Query::select()
			.expr(Expr::val(1))
			.to_string(PostgresQueryBuilder);

		// Act
		let (_container, pool, _port, _url) = start_postgres_container(config).await;
		let value: i32 = sqlx::query_scalar(&query)
			.fetch_one(pool.as_ref())
			.await
			.unwrap();

		// Assert
		assert_eq!(value, 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_nats_container_jetstream_enabled(
		#[future] nats_container: (ContainerAsync<GenericImage>, u16, String),
	) {
		// Arrange
		let (_container, port, url) = nats_container.await;

		// Act
		let info = probe_nats_jetstream(port)
			.await
			.expect("Failed to read JetStream-enabled NATS INFO");

		// Assert
		assert_eq!(url, format!("nats://localhost:{port}"));
		assert_eq!(info["jetstream"].as_bool(), Some(true));
		assert_eq!(info["port"].as_u64(), Some(4222));
	}

	#[rstest]
	fn test_get_pool_config_defaults() {
		// Arrange (uses default env — no TEST_MAX_CONNECTIONS or TEST_ACQUIRE_TIMEOUT_SECS set)

		// Act
		let (max_connections, acquire_timeout) = get_pool_config();

		// Assert
		assert!(
			max_connections > 0,
			"Expected max_connections > 0, got: {}",
			max_connections
		);
		assert!(
			acquire_timeout > 0,
			"Expected acquire_timeout > 0, got: {}",
			acquire_timeout
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_is_port_available() {
		// Arrange
		// Bind to port 0 to get an OS-assigned free port, then release it
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
			.await
			.expect("Failed to bind to random port");
		let port = listener.local_addr().unwrap().port();
		drop(listener);

		// Act
		let available = is_port_available(port).await;

		// Assert
		assert!(available, "Expected released port {} to be available", port);
	}

	#[rstest]
	#[tokio::test]
	async fn test_postgres_container_connects(
		#[future] postgres_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		// Arrange
		let (_container, pool, _port, _url) = postgres_container.await;

		// Act
		let row: (i32,) = sqlx::query_as("SELECT 1")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to execute SELECT 1 on postgres container");

		// Assert
		assert_eq!(row.0, 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_postgres_container_port_nonzero(
		#[future] postgres_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		// Arrange
		let (_container, _pool, port, _url) = postgres_container.await;

		// Act (no-op: port is set at initialization)

		// Assert
		assert!(port > 0, "Expected port to be non-zero, got: {}", port);
	}

	#[rstest]
	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn test_create_test_any_pool(
		#[future] postgres_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		// Arrange
		let (_container, _pool, _port, url) = postgres_container.await;
		sqlx::any::install_default_drivers();

		// Act
		let any_pool = create_test_any_pool(&url).await;

		// Assert
		assert!(
			any_pool.is_ok(),
			"Expected create_test_any_pool to succeed, got: {:?}",
			any_pool.err()
		);
	}
}
