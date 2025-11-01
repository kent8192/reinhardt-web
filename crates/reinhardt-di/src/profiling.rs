//! Performance profiling for dependency injection
//!
//! This module provides tools to profile and measure the performance of dependency
//! injection operations, helping identify bottlenecks during development.
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_di::profiling::{DependencyProfiler, ProfileReport};
//!
//! let mut profiler = DependencyProfiler::new();
//! profiler.start_resolve("Database");
//! // ... perform resolution ...
//! profiler.end_resolve("Database");
//!
//! let report = profiler.generate_report();
//! println!("{}", report.to_string());
//! ```

#[cfg(feature = "dev-tools")]
use std::collections::HashMap;
#[cfg(feature = "dev-tools")]
use std::time::{Duration, Instant};

/// A profiler for tracking dependency injection performance
#[cfg(feature = "dev-tools")]
#[derive(Debug)]
pub struct DependencyProfiler {
	records: HashMap<String, Vec<ProfileRecord>>,
	active: HashMap<String, Instant>,
}

/// A single profile record for a dependency resolution
#[cfg(feature = "dev-tools")]
#[derive(Debug, Clone)]
pub struct ProfileRecord {
	/// Name of the dependency
	pub name: String,
	/// Time taken to resolve
	pub duration: Duration,
	/// Timestamp when resolution started
	pub timestamp: Instant,
	/// Whether the dependency was cached
	pub cached: bool,
}

/// Summary report of profiling data
#[cfg(feature = "dev-tools")]
#[derive(Debug, Clone)]
pub struct ProfileReport {
	/// Per-dependency statistics
	pub dependencies: HashMap<String, DependencyStats>,
	/// Total number of resolutions
	pub total_resolutions: usize,
	/// Total time spent resolving dependencies
	pub total_duration: Duration,
}

/// Statistics for a single dependency
#[cfg(feature = "dev-tools")]
#[derive(Debug, Clone)]
pub struct DependencyStats {
	/// Dependency name
	pub name: String,
	/// Number of times resolved
	pub count: usize,
	/// Number of cache hits
	pub cache_hits: usize,
	/// Average resolution time
	pub avg_duration: Duration,
	/// Minimum resolution time
	pub min_duration: Duration,
	/// Maximum resolution time
	pub max_duration: Duration,
	/// Total time spent resolving this dependency
	pub total_duration: Duration,
}

#[cfg(feature = "dev-tools")]
impl DependencyProfiler {
	/// Create a new profiler
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	///
	/// let profiler = DependencyProfiler::new();
	/// ```
	pub fn new() -> Self {
		Self {
			records: HashMap::new(),
			active: HashMap::new(),
		}
	}

	/// Start timing a dependency resolution
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.start_resolve("Database");
	/// ```
	pub fn start_resolve(&mut self, name: impl Into<String>) {
		let name = name.into();
		self.active.insert(name, Instant::now());
	}

	/// End timing a dependency resolution
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	/// use std::thread::sleep;
	/// use std::time::Duration;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.start_resolve("Database");
	/// sleep(Duration::from_millis(10));
	/// profiler.end_resolve("Database");
	/// ```
	pub fn end_resolve(&mut self, name: impl Into<String>) {
		self.end_resolve_with_cache(name, false);
	}

	/// End timing with cache status information
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.start_resolve("Database");
	/// profiler.end_resolve_with_cache("Database", true);
	/// ```
	pub fn end_resolve_with_cache(&mut self, name: impl Into<String>, cached: bool) {
		let name = name.into();
		if let Some(start) = self.active.remove(&name) {
			let duration = start.elapsed();
			let record = ProfileRecord {
				name: name.clone(),
				duration,
				timestamp: start,
				cached,
			};

			self.records
				.entry(name)
				.or_insert_with(Vec::new)
				.push(record);
		}
	}

	/// Record a dependency resolution with explicit duration
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	/// use std::time::Duration;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.record("Database", Duration::from_millis(15), false);
	/// ```
	pub fn record(&mut self, name: impl Into<String>, duration: Duration, cached: bool) {
		let name = name.into();
		let record = ProfileRecord {
			name: name.clone(),
			duration,
			timestamp: Instant::now(),
			cached,
		};

		self.records
			.entry(name)
			.or_insert_with(Vec::new)
			.push(record);
	}

	/// Generate a profiling report
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	/// use std::time::Duration;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.record("Database", Duration::from_millis(10), false);
	/// profiler.record("Database", Duration::from_millis(5), true);
	///
	/// let report = profiler.generate_report();
	/// assert_eq!(report.total_resolutions, 2);
	/// ```
	pub fn generate_report(&self) -> ProfileReport {
		let mut dependencies = HashMap::new();
		let mut total_resolutions = 0;
		let mut total_duration = Duration::ZERO;

		for (name, records) in &self.records {
			let count = records.len();
			let cache_hits = records.iter().filter(|r| r.cached).count();

			let mut durations: Vec<Duration> = records.iter().map(|r| r.duration).collect();
			durations.sort();

			let min_duration = durations.first().copied().unwrap_or(Duration::ZERO);
			let max_duration = durations.last().copied().unwrap_or(Duration::ZERO);
			let total_dep_duration: Duration = durations.iter().sum();
			let avg_duration = if count > 0 {
				total_dep_duration / count as u32
			} else {
				Duration::ZERO
			};

			dependencies.insert(
				name.clone(),
				DependencyStats {
					name: name.clone(),
					count,
					cache_hits,
					avg_duration,
					min_duration,
					max_duration,
					total_duration: total_dep_duration,
				},
			);

			total_resolutions += count;
			total_duration += total_dep_duration;
		}

		ProfileReport {
			dependencies,
			total_resolutions,
			total_duration,
		}
	}

	/// Clear all profiling data
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	/// use std::time::Duration;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.record("Database", Duration::from_millis(10), false);
	/// profiler.clear();
	///
	/// let report = profiler.generate_report();
	/// assert_eq!(report.total_resolutions, 0);
	/// ```
	pub fn clear(&mut self) {
		self.records.clear();
		self.active.clear();
	}
}

#[cfg(feature = "dev-tools")]
impl Default for DependencyProfiler {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(feature = "dev-tools")]
impl ProfileReport {
	/// Format the report as a human-readable string
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	/// use std::time::Duration;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.record("Database", Duration::from_millis(10), false);
	///
	/// let report = profiler.generate_report();
	/// let output = report.to_string();
	/// assert!(output.contains("Database"));
	/// ```
	pub fn to_string(&self) -> String {
		let mut output = String::from("=== Dependency Injection Profile Report ===\n\n");
		output.push_str(&format!("Total Resolutions: {}\n", self.total_resolutions));
		output.push_str(&format!(
			"Total Time: {:.2}ms\n\n",
			self.total_duration.as_secs_f64() * 1000.0
		));

		let mut deps: Vec<_> = self.dependencies.values().collect();
		deps.sort_by(|a, b| b.total_duration.cmp(&a.total_duration));

		output.push_str("Per-Dependency Statistics:\n");
		output.push_str(&format!(
			"{:<30} {:>10} {:>10} {:>10} {:>10} {:>10}\n",
			"Name", "Count", "Cache Hits", "Avg (ms)", "Min (ms)", "Max (ms)"
		));
		output.push_str(&"-".repeat(90));
		output.push('\n');

		for stats in deps {
			output.push_str(&format!(
				"{:<30} {:>10} {:>10} {:>10.2} {:>10.2} {:>10.2}\n",
				stats.name,
				stats.count,
				stats.cache_hits,
				stats.avg_duration.as_secs_f64() * 1000.0,
				stats.min_duration.as_secs_f64() * 1000.0,
				stats.max_duration.as_secs_f64() * 1000.0,
			));
		}

		output
	}

	/// Get the slowest dependencies
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	/// use std::time::Duration;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.record("Fast", Duration::from_millis(1), false);
	/// profiler.record("Slow", Duration::from_millis(100), false);
	///
	/// let report = profiler.generate_report();
	/// let slowest = report.slowest_dependencies(1);
	/// assert_eq!(slowest[0].name, "Slow");
	/// ```
	pub fn slowest_dependencies(&self, n: usize) -> Vec<&DependencyStats> {
		let mut deps: Vec<_> = self.dependencies.values().collect();
		deps.sort_by(|a, b| b.avg_duration.cmp(&a.avg_duration));
		deps.into_iter().take(n).collect()
	}

	/// Get dependencies with low cache hit rates
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_di::profiling::DependencyProfiler;
	/// use std::time::Duration;
	///
	/// let mut profiler = DependencyProfiler::new();
	/// profiler.record("A", Duration::from_millis(10), true);
	/// profiler.record("A", Duration::from_millis(10), true);
	/// profiler.record("B", Duration::from_millis(10), false);
	/// profiler.record("B", Duration::from_millis(10), false);
	///
	/// let report = profiler.generate_report();
	/// let low_cache = report.low_cache_hit_dependencies(0.5);
	/// assert!(low_cache.iter().any(|s| s.name == "B"));
	/// ```
	pub fn low_cache_hit_dependencies(&self, threshold: f64) -> Vec<&DependencyStats> {
		self.dependencies
			.values()
			.filter(|s| {
				if s.count == 0 {
					false
				} else {
					(s.cache_hits as f64 / s.count as f64) < threshold
				}
			})
			.collect()
	}
}
