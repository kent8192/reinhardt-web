use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use reinhardt_db::backends::schema::{BaseDatabaseSchemaEditor, SchemaEditorResult};

/// Call record for tracking function calls
#[derive(Debug, Clone)]
pub struct CallRecord {
	pub args: Vec<serde_json::Value>,
	pub timestamp: std::time::Instant,
}

/// Mock function tracker
pub struct MockFunction<T> {
	calls: Arc<Mutex<Vec<CallRecord>>>,
	return_values: Arc<Mutex<VecDeque<T>>>,
	default_return: Option<T>,
}

impl<T: Clone> MockFunction<T> {
	/// Create a new mock function
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// assert_eq!(mock.call_count().await, 0);
	/// # });
	/// ```
	pub fn new() -> Self {
		Self {
			calls: Arc::new(Mutex::new(Vec::new())),
			return_values: Arc::new(Mutex::new(VecDeque::new())),
			default_return: None,
		}
	}
	/// Create a mock function with a default return value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::with_default(42);
	/// let result = mock.call(vec![]).await;
	/// assert_eq!(result, Some(42));
	/// # });
	/// ```
	pub fn with_default(default_return: T) -> Self {
		Self {
			calls: Arc::new(Mutex::new(Vec::new())),
			return_values: Arc::new(Mutex::new(VecDeque::new())),
			default_return: Some(default_return),
		}
	}
	/// Queue a return value for the next call
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// mock.returns(42).await;
	///
	/// let result = mock.call(vec![]).await;
	/// assert_eq!(result, Some(42));
	/// # });
	/// ```
	pub async fn returns(&self, value: T) {
		self.return_values.lock().await.push_back(value);
	}
	/// Queue multiple return values for sequential calls
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// mock.returns_many(vec![1, 2, 3]).await;
	///
	/// assert_eq!(mock.call(vec![]).await, Some(1));
	/// assert_eq!(mock.call(vec![]).await, Some(2));
	/// assert_eq!(mock.call(vec![]).await, Some(3));
	/// # });
	/// ```
	pub async fn returns_many(&self, values: Vec<T>) {
		let mut queue = self.return_values.lock().await;
		for value in values {
			queue.push_back(value);
		}
	}
	/// Record a call and return the next queued value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// mock.returns(42).await;
	///
	/// let result = mock.call(vec![json!("arg1"), json!(123)]).await;
	/// assert_eq!(result, Some(42));
	/// assert_eq!(mock.call_count().await, 1);
	/// # });
	/// ```
	pub async fn call(&self, args: Vec<serde_json::Value>) -> Option<T> {
		let record = CallRecord {
			args,
			timestamp: std::time::Instant::now(),
		};
		self.calls.lock().await.push(record);

		let mut queue = self.return_values.lock().await;
		queue.pop_front().or_else(|| self.default_return.clone())
	}
	/// Get the number of times the function has been called
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// assert_eq!(mock.call_count().await, 0);
	///
	/// mock.call(vec![]).await;
	/// assert_eq!(mock.call_count().await, 1);
	/// # });
	/// ```
	pub async fn call_count(&self) -> usize {
		self.calls.lock().await.len()
	}
	/// Check if the function has been called at least once
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// assert!(!mock.was_called().await);
	///
	/// mock.call(vec![]).await;
	/// assert!(mock.was_called().await);
	/// # });
	/// ```
	pub async fn was_called(&self) -> bool {
		self.call_count().await > 0
	}
	/// Check if the function was called with specific arguments
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// mock.call(vec![json!("test"), json!(42)]).await;
	///
	/// assert!(mock.was_called_with(vec![json!("test"), json!(42)]).await);
	/// assert!(!mock.was_called_with(vec![json!("other")]).await);
	/// # });
	/// ```
	pub async fn was_called_with(&self, args: Vec<serde_json::Value>) -> bool {
		let calls = self.calls.lock().await;
		calls.iter().any(|record| record.args == args)
	}
	/// Get all call records for inspection
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// mock.call(vec![json!("arg1")]).await;
	/// mock.call(vec![json!("arg2")]).await;
	///
	/// let calls = mock.get_calls().await;
	/// assert_eq!(calls.len(), 2);
	/// assert_eq!(calls[0].args, vec![json!("arg1")]);
	/// # });
	/// ```
	pub async fn get_calls(&self) -> Vec<CallRecord> {
		self.calls.lock().await.clone()
	}
	/// Reset the mock to its initial state
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// mock.call(vec![]).await;
	/// assert_eq!(mock.call_count().await, 1);
	///
	/// mock.reset().await;
	/// assert_eq!(mock.call_count().await, 0);
	/// # });
	/// ```
	pub async fn reset(&self) {
		self.calls.lock().await.clear();
		self.return_values.lock().await.clear();
	}
	/// Get the arguments from the last function call
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockFunction;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let mock = MockFunction::<i32>::new();
	/// mock.call(vec![json!("first")]).await;
	/// mock.call(vec![json!("last")]).await;
	///
	/// let last_args = mock.last_call_args().await;
	/// assert_eq!(last_args, Some(vec![json!("last")]));
	/// # });
	/// ```
	pub async fn last_call_args(&self) -> Option<Vec<serde_json::Value>> {
		self.calls.lock().await.last().map(|r| r.args.clone())
	}
}

impl<T: Clone> Default for MockFunction<T> {
	fn default() -> Self {
		Self::new()
	}
}

/// Spy for tracking method calls with arguments
pub struct Spy<T> {
	inner: Option<T>,
	calls: Arc<Mutex<Vec<CallRecord>>>,
}

impl<T> Spy<T> {
	/// Create a new spy without wrapping any object
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::Spy;
	///
	/// let spy = Spy::<String>::new();
	/// assert!(spy.inner().is_none());
	/// ```
	pub fn new() -> Self {
		Self {
			inner: None,
			calls: Arc::new(Mutex::new(Vec::new())),
		}
	}
	/// Create a spy that wraps an existing object
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::Spy;
	///
	/// let value = "test".to_string();
	/// let spy = Spy::wrap(value);
	/// assert!(spy.inner().is_some());
	/// ```
	pub fn wrap(inner: T) -> Self {
		Self {
			inner: Some(inner),
			calls: Arc::new(Mutex::new(Vec::new())),
		}
	}
	/// Record a method call with arguments
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::Spy;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let spy = Spy::<String>::new();
	/// spy.record_call(vec![json!("arg1"), json!(42)]).await;
	/// assert_eq!(spy.call_count().await, 1);
	/// # });
	/// ```
	pub async fn record_call(&self, args: Vec<serde_json::Value>) {
		let record = CallRecord {
			args,
			timestamp: std::time::Instant::now(),
		};
		self.calls.lock().await.push(record);
	}
	pub async fn call_count(&self) -> usize {
		self.calls.lock().await.len()
	}
	pub async fn was_called(&self) -> bool {
		self.call_count().await > 0
	}
	/// Check if the spy was called with specific arguments
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::Spy;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let spy = Spy::<String>::new();
	/// spy.record_call(vec![json!("test")]).await;
	///
	/// assert!(spy.was_called_with(vec![json!("test")]).await);
	/// assert!(!spy.was_called_with(vec![json!("other")]).await);
	/// # });
	/// ```
	pub async fn was_called_with(&self, args: Vec<serde_json::Value>) -> bool {
		let calls = self.calls.lock().await;
		calls.iter().any(|record| record.args == args)
	}
	pub async fn get_calls(&self) -> Vec<CallRecord> {
		self.calls.lock().await.clone()
	}
	pub async fn last_call_args(&self) -> Option<Vec<serde_json::Value>> {
		self.calls.lock().await.last().map(|r| r.args.clone())
	}
	/// Reset the spy by clearing all call records
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::Spy;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let spy = Spy::<String>::new();
	/// spy.record_call(vec![json!("test")]).await;
	/// assert_eq!(spy.call_count().await, 1);
	///
	/// spy.reset().await;
	/// assert_eq!(spy.call_count().await, 0);
	/// # });
	/// ```
	pub async fn reset(&self) {
		self.calls.lock().await.clear();
	}
	pub fn inner(&self) -> Option<&T> {
		self.inner.as_ref()
	}
	pub fn into_inner(self) -> Option<T> {
		self.inner
	}
}

impl<T> Default for Spy<T> {
	fn default() -> Self {
		Self::new()
	}
}

/// Dummy cache implementation for testing
///
/// A simple in-memory cache backend that implements the `CacheBackend` trait.
/// Useful for testing cache-dependent code without external dependencies.
///
/// # Examples
///
/// ```
/// use reinhardt_test::mock::DummyCache;
/// use reinhardt_backends::cache::CacheBackend;
/// use std::time::Duration;
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let cache = DummyCache::new();
/// cache.set("key", b"value", Some(Duration::from_secs(60))).await.unwrap();
/// let value = cache.get("key").await.unwrap();
/// assert_eq!(value, Some(b"value".to_vec()));
/// # });
/// ```
pub struct DummyCache {
	storage: Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

impl DummyCache {
	/// Create a new DummyCache instance
	pub fn new() -> Self {
		Self {
			storage: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
		}
	}
}

impl Default for DummyCache {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl reinhardt_backends::cache::CacheBackend for DummyCache {
	async fn get(&self, key: &str) -> reinhardt_backends::cache::CacheResult<Option<Vec<u8>>> {
		Ok(self.storage.lock().unwrap().get(key).cloned())
	}

	async fn set(
		&self,
		key: &str,
		value: &[u8],
		_ttl: Option<std::time::Duration>,
	) -> reinhardt_backends::cache::CacheResult<()> {
		self.storage
			.lock()
			.unwrap()
			.insert(key.to_string(), value.to_vec());
		Ok(())
	}

	async fn delete(&self, key: &str) -> reinhardt_backends::cache::CacheResult<bool> {
		Ok(self.storage.lock().unwrap().remove(key).is_some())
	}

	async fn exists(&self, key: &str) -> reinhardt_backends::cache::CacheResult<bool> {
		Ok(self.storage.lock().unwrap().contains_key(key))
	}

	async fn clear(&self) -> reinhardt_backends::cache::CacheResult<()> {
		self.storage.lock().unwrap().clear();
		Ok(())
	}

	async fn get_many(
		&self,
		keys: &[String],
	) -> reinhardt_backends::cache::CacheResult<Vec<Option<Vec<u8>>>> {
		let storage = self.storage.lock().unwrap();
		Ok(keys.iter().map(|k| storage.get(k).cloned()).collect())
	}

	async fn set_many(
		&self,
		items: &[(String, Vec<u8>)],
		_ttl: Option<std::time::Duration>,
	) -> reinhardt_backends::cache::CacheResult<()> {
		let mut storage = self.storage.lock().unwrap();
		for (key, value) in items {
			storage.insert(key.clone(), value.clone());
		}
		Ok(())
	}

	async fn delete_many(&self, keys: &[String]) -> reinhardt_backends::cache::CacheResult<usize> {
		let mut storage = self.storage.lock().unwrap();
		let mut count = 0;
		for key in keys {
			if storage.remove(key).is_some() {
				count += 1;
			}
		}
		Ok(count)
	}
}

// ============================================================================
// Mock Redis Cluster Cache
// ============================================================================

/// Mock Redis Cluster cache implementation for testing
///
/// Simulates a 3-node Redis Cluster with CRC16-based key sharding.
/// This mock backend provides a lightweight alternative to TestContainers
/// for testing Redis Cluster functionality without requiring actual Redis instances.
///
/// # Features
///
/// - **3-node cluster simulation**: Keys are distributed across 3 nodes based on hash slots
/// - **CRC16 hash slot calculation**: Uses the same algorithm as real Redis Cluster (0-16383)
/// - **Key prefix support**: Namespace isolation similar to RedisClusterCache
/// - **In-memory storage**: Fast, no external dependencies
///
/// # Slot Distribution
///
/// - Node 0: slots 0-5460
/// - Node 1: slots 5461-10922
/// - Node 2: slots 10923-16383
///
/// # Examples
///
/// ```
/// use reinhardt_test::mock::MockRedisClusterCache;
/// use reinhardt_cache::Cache;
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let cache = MockRedisClusterCache::new();
/// cache.set("user:123", &"John Doe", None).await.unwrap();
/// let value: Option<String> = cache.get("user:123").await.unwrap();
/// assert_eq!(value, Some("John Doe".to_string()));
/// # });
/// ```
///
/// # Limitations
///
/// - TTL is accepted but not enforced (keys don't expire)
/// - No actual network communication or cluster topology management
/// - Failover and replica functionality are not simulated
pub struct MockRedisClusterCache {
	/// Storage for each of the 3 cluster nodes
	/// Index 0: Node 0 (slots 0-5460)
	/// Index 1: Node 1 (slots 5461-10922)
	/// Index 2: Node 2 (slots 10923-16383)
	nodes: Arc<std::sync::Mutex<[std::collections::HashMap<String, Vec<u8>>; 3]>>,
	/// Key prefix for namespacing
	key_prefix: String,
}

impl MockRedisClusterCache {
	/// Create a new MockRedisClusterCache instance
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockRedisClusterCache;
	///
	/// let cache = MockRedisClusterCache::new();
	/// ```
	pub fn new() -> Self {
		Self {
			nodes: Arc::new(std::sync::Mutex::new([
				std::collections::HashMap::new(),
				std::collections::HashMap::new(),
				std::collections::HashMap::new(),
			])),
			key_prefix: String::new(),
		}
	}

	/// Create a new instance with a key prefix
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::mock::MockRedisClusterCache;
	///
	/// let cache = MockRedisClusterCache::with_prefix("myapp");
	/// ```
	pub fn with_prefix(prefix: impl Into<String>) -> Self {
		Self {
			nodes: Arc::new(std::sync::Mutex::new([
				std::collections::HashMap::new(),
				std::collections::HashMap::new(),
				std::collections::HashMap::new(),
			])),
			key_prefix: prefix.into(),
		}
	}

	/// Build the full key with prefix
	fn build_key(&self, key: &str) -> String {
		if self.key_prefix.is_empty() {
			key.to_string()
		} else {
			format!("{}:{}", self.key_prefix, key)
		}
	}

	/// Calculate CRC16 hash slot for a key (0-16383)
	///
	/// Uses the same algorithm as Redis Cluster for key distribution.
	fn calculate_slot(key: &str) -> u16 {
		// Extract hash tag if present (e.g., "user:{123}" -> "123")
		let hash_key = if let Some(start) = key.find('{') {
			if let Some(end) = key[start + 1..].find('}') {
				let tag_end = start + 1 + end;
				if tag_end > start + 1 {
					&key[start + 1..tag_end]
				} else {
					key
				}
			} else {
				key
			}
		} else {
			key
		};

		// CRC16 calculation (XMODEM variant used by Redis)
		let mut crc: u16 = 0;
		for &byte in hash_key.as_bytes() {
			crc ^= (byte as u16) << 8;
			for _ in 0..8 {
				if (crc & 0x8000) != 0 {
					crc = (crc << 1) ^ 0x1021;
				} else {
					crc <<= 1;
				}
			}
		}
		crc % 16384 // Redis uses 16384 slots (0-16383)
	}

	/// Determine which node (0-2) should handle a given slot
	fn slot_to_node(slot: u16) -> usize {
		match slot {
			0..=5460 => 0,      // Node 0
			5461..=10922 => 1,  // Node 1
			10923..=16383 => 2, // Node 2
			_ => unreachable!("Invalid slot number"),
		}
	}

	/// Get the storage node for a given key
	fn get_node(&self, key: &str) -> usize {
		let slot = Self::calculate_slot(key);
		Self::slot_to_node(slot)
	}
}

impl Default for MockRedisClusterCache {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl reinhardt_backends::cache::CacheBackend for MockRedisClusterCache {
	async fn get(&self, key: &str) -> reinhardt_backends::cache::CacheResult<Option<Vec<u8>>> {
		let full_key = self.build_key(key);
		let node_idx = self.get_node(&full_key);
		let nodes = self.nodes.lock().unwrap();
		Ok(nodes[node_idx].get(&full_key).cloned())
	}

	async fn set(
		&self,
		key: &str,
		value: &[u8],
		_ttl: Option<std::time::Duration>,
	) -> reinhardt_backends::cache::CacheResult<()> {
		let full_key = self.build_key(key);
		let node_idx = self.get_node(&full_key);
		let mut nodes = self.nodes.lock().unwrap();
		nodes[node_idx].insert(full_key, value.to_vec());
		Ok(())
	}

	async fn delete(&self, key: &str) -> reinhardt_backends::cache::CacheResult<bool> {
		let full_key = self.build_key(key);
		let node_idx = self.get_node(&full_key);
		let mut nodes = self.nodes.lock().unwrap();
		Ok(nodes[node_idx].remove(&full_key).is_some())
	}

	async fn exists(&self, key: &str) -> reinhardt_backends::cache::CacheResult<bool> {
		let full_key = self.build_key(key);
		let node_idx = self.get_node(&full_key);
		let nodes = self.nodes.lock().unwrap();
		Ok(nodes[node_idx].contains_key(&full_key))
	}

	async fn clear(&self) -> reinhardt_backends::cache::CacheResult<()> {
		let mut nodes = self.nodes.lock().unwrap();
		if self.key_prefix.is_empty() {
			// Clear all nodes if no prefix
			for node in nodes.iter_mut() {
				node.clear();
			}
		} else {
			// Clear only keys with the prefix
			let prefix_pattern = format!("{}:", self.key_prefix);
			for node in nodes.iter_mut() {
				node.retain(|k, _| !k.starts_with(&prefix_pattern));
			}
		}
		Ok(())
	}

	async fn get_many(
		&self,
		keys: &[String],
	) -> reinhardt_backends::cache::CacheResult<Vec<Option<Vec<u8>>>> {
		let nodes = self.nodes.lock().unwrap();
		let results: Vec<Option<Vec<u8>>> = keys
			.iter()
			.map(|k| {
				let full_key = self.build_key(k);
				let node_idx = self.get_node(&full_key);
				nodes[node_idx].get(&full_key).cloned()
			})
			.collect();
		Ok(results)
	}

	async fn set_many(
		&self,
		items: &[(String, Vec<u8>)],
		_ttl: Option<std::time::Duration>,
	) -> reinhardt_backends::cache::CacheResult<()> {
		let mut nodes = self.nodes.lock().unwrap();
		for (key, value) in items {
			let full_key = self.build_key(key);
			let node_idx = self.get_node(&full_key);
			nodes[node_idx].insert(full_key, value.clone());
		}
		Ok(())
	}

	async fn delete_many(&self, keys: &[String]) -> reinhardt_backends::cache::CacheResult<usize> {
		let mut nodes = self.nodes.lock().unwrap();
		let mut count = 0;
		for key in keys {
			let full_key = self.build_key(key);
			let node_idx = self.get_node(&full_key);
			if nodes[node_idx].remove(&full_key).is_some() {
				count += 1;
			}
		}
		Ok(count)
	}
}

// ============================================================================
// Handler Mocks
// ============================================================================

/// Simple handler wrapper for testing
///
/// Provides a convenient way to create handlers from closures for testing purposes.
/// The handler function can be any closure that takes a [`Request`] and returns a
/// [`Result<Response>`].
///
/// # Examples
///
/// ## Basic usage
///
/// ```no_run
/// use reinhardt_test::mock::SimpleHandler;
/// use reinhardt_apps::{Request, Response};
/// use reinhardt_types::Handler;
///
/// let handler = SimpleHandler::new(|req: Request| {
///     Ok(Response::ok().with_body("Hello, World!"))
/// });
///
/// // Use handler in tests
/// ```
///
/// ## With path-based routing
///
/// ```no_run
/// use reinhardt_test::mock::SimpleHandler;
/// use reinhardt_apps::{Request, Response};
///
/// let handler = SimpleHandler::new(|req: Request| {
///     match req.path() {
///         "/" => Ok(Response::ok().with_body("Home")),
///         "/api" => Ok(Response::ok().with_body(r#"{"status": "ok"}"#)),
///         _ => Ok(Response::not_found().with_body("Not Found")),
///     }
/// });
/// ```
///
/// ## With custom logic
///
/// ```no_run
/// use reinhardt_test::mock::SimpleHandler;
/// use reinhardt_apps::{Request, Response};
/// use std::sync::{Arc, Mutex};
///
/// let call_count = Arc::new(Mutex::new(0));
/// let call_count_clone = call_count.clone();
///
/// let handler = SimpleHandler::new(move |req: Request| {
///     let mut count = call_count_clone.lock().unwrap();
///     *count += 1;
///     Ok(Response::ok().with_body(format!("Call count: {}", *count)))
/// });
/// ```
pub struct SimpleHandler<F>
where
	F: Fn(reinhardt_apps::Request) -> reinhardt_apps::Result<reinhardt_apps::Response>
		+ Send
		+ Sync
		+ 'static,
{
	handler_fn: F,
}

impl<F> SimpleHandler<F>
where
	F: Fn(reinhardt_apps::Request) -> reinhardt_apps::Result<reinhardt_apps::Response>
		+ Send
		+ Sync
		+ 'static,
{
	/// Create a new SimpleHandler with the given handler function
	///
	/// # Arguments
	///
	/// * `handler_fn` - A closure that processes requests and returns responses
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_test::mock::SimpleHandler;
	/// use reinhardt_apps::{Request, Response};
	///
	/// let handler = SimpleHandler::new(|req| {
	///     Ok(Response::ok().with_body("Success"))
	/// });
	/// ```
	pub fn new(handler_fn: F) -> Self {
		Self { handler_fn }
	}
}

#[async_trait::async_trait]
impl<F> reinhardt_types::Handler for SimpleHandler<F>
where
	F: Fn(reinhardt_apps::Request) -> reinhardt_apps::Result<reinhardt_apps::Response>
		+ Send
		+ Sync
		+ 'static,
{
	async fn handle(
		&self,
		request: reinhardt_apps::Request,
	) -> reinhardt_apps::Result<reinhardt_apps::Response> {
		(self.handler_fn)(request)
	}
}

// ============================================================================
// Schema Editor Mocks
// ============================================================================

/// Mock schema editor for testing database migration operations
///
/// A simple mock implementation of `BaseDatabaseSchemaEditor` that doesn't
/// execute actual SQL but allows testing of schema modification logic.
///
/// # Examples
///
/// ```
/// use reinhardt_test::mock::MockSchemaEditor;
/// use reinhardt_db_backends::schema::BaseDatabaseSchemaEditor;
/// use sea_query::PostgresQueryBuilder;
///
/// let editor = MockSchemaEditor::new();
/// let stmt = editor.create_table_statement("users", &[
///     ("id", "INTEGER PRIMARY KEY"),
///     ("name", "VARCHAR(100)"),
/// ]);
/// let sql = stmt.to_string(PostgresQueryBuilder);
/// assert!(sql.contains("CREATE TABLE"));
/// ```
pub struct MockSchemaEditor;

impl MockSchemaEditor {
	/// Create a new MockSchemaEditor instance
	pub fn new() -> Self {
		Self
	}
}

impl Default for MockSchemaEditor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl BaseDatabaseSchemaEditor for MockSchemaEditor {
	async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
		// Mock implementation - doesn't execute anything
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_mock_function() {
		let mock = MockFunction::<i32>::new();

		mock.returns(42).await;
		mock.returns(100).await;

		let result1 = mock.call(vec![serde_json::json!(1)]).await;
		assert_eq!(result1, Some(42));

		let result2 = mock.call(vec![serde_json::json!(2)]).await;
		assert_eq!(result2, Some(100));

		assert_eq!(mock.call_count().await, 2);
		assert!(mock.was_called().await);
	}

	#[tokio::test]
	async fn test_mock_default() {
		let mock = MockFunction::with_default(99);

		let result = mock.call(vec![]).await;
		assert_eq!(result, Some(99));
	}

	#[tokio::test]
	async fn test_spy() {
		use serde_json::json;

		let spy: Spy<String> = Spy::new();

		spy.record_call(vec![json!("arg1")]).await;
		spy.record_call(vec![json!("arg2")]).await;

		assert_eq!(spy.call_count().await, 2);
		assert!(spy.was_called().await);
		assert!(spy.was_called_with(vec![json!("arg1")]).await);
	}

	#[tokio::test]
	async fn test_mock_reset() {
		let mock = MockFunction::<i32>::new();
		mock.call(vec![]).await;
		assert_eq!(mock.call_count().await, 1);

		mock.reset().await;
		assert_eq!(mock.call_count().await, 0);
	}
}
