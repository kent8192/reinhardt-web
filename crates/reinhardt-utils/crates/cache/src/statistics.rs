//! Cache statistics and entry information

/// Cache entry information for inspection
#[derive(Debug, Clone)]
pub struct CacheEntryInfo {
	/// The key of the entry
	pub key: String,
	/// Size of the value in bytes
	pub size: usize,
	/// Whether the entry has an expiration time
	pub has_expiry: bool,
	/// Seconds until expiration (if applicable)
	pub ttl_seconds: Option<u64>,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
	/// Number of cache hits
	pub hits: u64,
	/// Number of cache misses
	pub misses: u64,
	/// Total number of requests
	pub total_requests: u64,
	/// Current number of entries in cache
	pub entry_count: u64,
	/// Approximate memory usage in bytes
	pub memory_usage: u64,
}

impl CacheStatistics {
	/// Calculate hit rate (0.0 to 1.0)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheStatistics;
	///
	/// let mut stats = CacheStatistics::default();
	/// stats.hits = 75;
	/// stats.misses = 25;
	/// stats.total_requests = 100;
	///
	/// assert_eq!(stats.hit_rate(), 0.75);
	/// ```
	pub fn hit_rate(&self) -> f64 {
		if self.total_requests == 0 {
			0.0
		} else {
			self.hits as f64 / self.total_requests as f64
		}
	}

	/// Calculate miss rate (0.0 to 1.0)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheStatistics;
	///
	/// let mut stats = CacheStatistics::default();
	/// stats.hits = 75;
	/// stats.misses = 25;
	/// stats.total_requests = 100;
	///
	/// assert_eq!(stats.miss_rate(), 0.25);
	/// ```
	pub fn miss_rate(&self) -> f64 {
		if self.total_requests == 0 {
			0.0
		} else {
			self.misses as f64 / self.total_requests as f64
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_statistics_hit_miss_rate_zero_requests() {
		let stats = CacheStatistics::default();
		assert_eq!(stats.hit_rate(), 0.0);
		assert_eq!(stats.miss_rate(), 0.0);
	}
}
