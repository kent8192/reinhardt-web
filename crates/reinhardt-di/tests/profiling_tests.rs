//! Tests for dependency injection profiling (dev-tools feature)

#![cfg(feature = "dev-tools")]

use reinhardt_di::profiling::DependencyProfiler;
use rstest::*;
use std::time::Duration;

/// Test that profiler accurately tracks dependency resolution time
#[rstest]
#[tokio::test]
async fn test_profiler_tracks_resolution_time() {
	let mut profiler = DependencyProfiler::new();

	profiler.start_resolve("Database");
	tokio::time::sleep(Duration::from_millis(10)).await;
	profiler.end_resolve("Database");

	let report = profiler.generate_report();
	assert_eq!(report.total_resolutions, 1);
	assert!(report.total_duration >= Duration::from_millis(10));

	let stats = report.dependencies.get("Database").unwrap();
	assert_eq!(stats.count, 1);
	assert!(stats.avg_duration >= Duration::from_millis(10));
}

/// Test profiler tracking multiple different dependencies
#[rstest]
#[tokio::test]
async fn test_profiler_multiple_dependencies() {
	let mut profiler = DependencyProfiler::new();

	// Track Database dependency
	profiler.start_resolve("Database");
	tokio::time::sleep(Duration::from_millis(5)).await;
	profiler.end_resolve("Database");

	// Track Cache dependency
	profiler.start_resolve("Cache");
	tokio::time::sleep(Duration::from_millis(3)).await;
	profiler.end_resolve("Cache");

	// Track Service dependency
	profiler.start_resolve("Service");
	tokio::time::sleep(Duration::from_millis(2)).await;
	profiler.end_resolve("Service");

	let report = profiler.generate_report();
	assert_eq!(report.total_resolutions, 3);
	assert_eq!(report.dependencies.len(), 3);

	let db_stats = report.dependencies.get("Database").unwrap();
	assert_eq!(db_stats.count, 1);

	let cache_stats = report.dependencies.get("Cache").unwrap();
	assert_eq!(cache_stats.count, 1);

	let service_stats = report.dependencies.get("Service").unwrap();
	assert_eq!(service_stats.count, 1);
}

/// Test profiler tracking nested dependency resolution
#[rstest]
#[tokio::test]
async fn test_profiler_nested_dependencies() {
	let mut profiler = DependencyProfiler::new();

	// Outer dependency
	profiler.start_resolve("UserService");
	tokio::time::sleep(Duration::from_millis(2)).await;

	// Inner dependencies (nested)
	profiler.start_resolve("Database");
	tokio::time::sleep(Duration::from_millis(5)).await;
	profiler.end_resolve("Database");

	profiler.start_resolve("Cache");
	tokio::time::sleep(Duration::from_millis(3)).await;
	profiler.end_resolve("Cache");

	profiler.end_resolve("UserService");

	let report = profiler.generate_report();
	assert_eq!(report.total_resolutions, 3);

	// All dependencies should be tracked
	assert!(report.dependencies.contains_key("UserService"));
	assert!(report.dependencies.contains_key("Database"));
	assert!(report.dependencies.contains_key("Cache"));
}

/// Test profiler report generation and statistics
#[rstest]
#[tokio::test]
async fn test_profiler_report_generation() {
	let mut profiler = DependencyProfiler::new();

	// Record multiple resolutions of the same dependency
	profiler.record("Database", Duration::from_millis(10), false);
	profiler.record("Database", Duration::from_millis(5), true);
	profiler.record("Database", Duration::from_millis(20), false);

	let report = profiler.generate_report();
	assert_eq!(report.total_resolutions, 3);

	let db_stats = report.dependencies.get("Database").unwrap();
	assert_eq!(db_stats.count, 3);
	assert_eq!(db_stats.cache_hits, 1);
	assert_eq!(db_stats.min_duration, Duration::from_millis(5));
	assert_eq!(db_stats.max_duration, Duration::from_millis(20));

	// Test slowest dependencies query
	let slowest = report.slowest_dependencies(1);
	assert_eq!(slowest.len(), 1);
	assert_eq!(slowest[0].name, "Database");

	// Test low cache hit rate detection
	let low_cache = report.low_cache_hit_dependencies(0.5);
	assert_eq!(low_cache.len(), 1);
	assert_eq!(low_cache[0].name, "Database");
}

/// Test profiler reset/clear functionality
#[rstest]
#[tokio::test]
async fn test_profiler_reset() {
	let mut profiler = DependencyProfiler::new();

	// Record some data
	profiler.record("Database", Duration::from_millis(10), false);
	profiler.record("Cache", Duration::from_millis(5), false);

	let report_before = profiler.generate_report();
	assert_eq!(report_before.total_resolutions, 2);

	// Clear profiler
	profiler.clear();

	let report_after = profiler.generate_report();
	assert_eq!(report_after.total_resolutions, 0);
	assert_eq!(report_after.dependencies.len(), 0);
	assert_eq!(report_after.total_duration, Duration::ZERO);
}
