use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

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
