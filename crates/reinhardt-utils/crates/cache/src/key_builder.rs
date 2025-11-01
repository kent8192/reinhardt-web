//! Cache key builder for generating cache keys

/// Cache key builder for generating cache keys
#[derive(Clone)]
pub struct CacheKeyBuilder {
	prefix: String,
	version: u32,
}

impl CacheKeyBuilder {
	/// Create a new cache key builder with the given prefix
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheKeyBuilder;
	///
	/// let builder = CacheKeyBuilder::new("myapp");
	/// assert_eq!(builder.build("user"), "myapp:1:user");
	/// ```
	pub fn new(prefix: impl Into<String>) -> Self {
		Self {
			prefix: prefix.into(),
			version: 1,
		}
	}
	/// Set the version for cache key namespacing
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheKeyBuilder;
	///
	/// let builder = CacheKeyBuilder::new("myapp").with_version(2);
	/// assert_eq!(builder.build("user"), "myapp:2:user");
	/// ```
	pub fn with_version(mut self, version: u32) -> Self {
		self.version = version;
		self
	}
	/// Build a cache key with prefix and version
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheKeyBuilder;
	///
	/// let builder = CacheKeyBuilder::new("app").with_version(3);
	/// let key = builder.build("user:123");
	/// assert_eq!(key, "app:3:user:123");
	/// ```
	pub fn build(&self, key: &str) -> String {
		format!("{}:{}:{}", self.prefix, self.version, key)
	}
	/// Build multiple cache keys at once
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheKeyBuilder;
	///
	/// let builder = CacheKeyBuilder::new("app");
	/// let keys = builder.build_many(&["user", "session", "token"]);
	/// assert_eq!(keys, vec!["app:1:user", "app:1:session", "app:1:token"]);
	/// ```
	pub fn build_many(&self, keys: &[&str]) -> Vec<String> {
		keys.iter().map(|k| self.build(k)).collect()
	}
}

impl Default for CacheKeyBuilder {
	fn default() -> Self {
		Self::new("app")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_cache_key_builder() {
		let builder = CacheKeyBuilder::new("myapp").with_version(2);

		assert_eq!(builder.build("user:123"), "myapp:2:user:123");

		let keys = builder.build_many(&["key1", "key2"]);
		assert_eq!(keys, vec!["myapp:2:key1", "myapp:2:key2"]);
	}
}
