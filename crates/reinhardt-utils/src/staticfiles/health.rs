//! Health Check System
//!
//! Provides health check functionality for monitoring application status,
//! similar to Django's health check framework and FastAPI's health endpoints.

use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Health status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
	/// Component is functioning normally
	Healthy,
	/// Component is degraded but still functional
	Degraded,
	/// Component is not functioning
	Unhealthy,
}

impl fmt::Display for HealthStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			HealthStatus::Healthy => write!(f, "healthy"),
			HealthStatus::Degraded => write!(f, "degraded"),
			HealthStatus::Unhealthy => write!(f, "unhealthy"),
		}
	}
}

/// Result of a single health check
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
	/// Name of the component being checked
	pub component: String,
	/// Current status of the component
	pub status: HealthStatus,
	/// Optional message providing additional details
	pub message: Option<String>,
	/// Optional metadata (e.g., response time, error count)
	pub metadata: HashMap<String, String>,
}

impl HealthCheckResult {
	/// Create a healthy result
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::HealthCheckResult;
	///
	/// let result = HealthCheckResult::healthy("database");
	/// assert_eq!(result.component, "database");
	/// ```
	pub fn healthy(component: impl Into<String>) -> Self {
		Self {
			component: component.into(),
			status: HealthStatus::Healthy,
			message: None,
			metadata: HashMap::new(),
		}
	}

	/// Create a degraded result
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::HealthCheckResult;
	///
	/// let result = HealthCheckResult::degraded("cache", "High latency detected");
	/// assert_eq!(result.component, "cache");
	/// ```
	pub fn degraded(component: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			component: component.into(),
			status: HealthStatus::Degraded,
			message: Some(message.into()),
			metadata: HashMap::new(),
		}
	}

	/// Create an unhealthy result
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::HealthCheckResult;
	///
	/// let result = HealthCheckResult::unhealthy("database", "Connection failed");
	/// assert_eq!(result.component, "database");
	/// ```
	pub fn unhealthy(component: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			component: component.into(),
			status: HealthStatus::Unhealthy,
			message: Some(message.into()),
			metadata: HashMap::new(),
		}
	}

	/// Add metadata to the result
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::HealthCheckResult;
	///
	/// let result = HealthCheckResult::healthy("api")
	///     .with_metadata("response_time_ms", "45");
	/// ```
	pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.metadata.insert(key.into(), value.into());
		self
	}
}

/// Overall health report
#[derive(Debug, Clone)]
pub struct HealthReport {
	/// Overall status (worst status among all checks)
	pub status: HealthStatus,
	/// Individual check results
	pub checks: Vec<HealthCheckResult>,
}

impl HealthReport {
	/// Create a new health report from check results
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::{HealthReport, HealthCheckResult};
	///
	/// let checks = vec![
	///     HealthCheckResult::healthy("database"),
	///     HealthCheckResult::healthy("cache"),
	/// ];
	/// let report = HealthReport::new(checks);
	/// ```
	pub fn new(checks: Vec<HealthCheckResult>) -> Self {
		let status = checks
			.iter()
			.map(|c| c.status)
			.max_by_key(|s| match s {
				HealthStatus::Healthy => 0,
				HealthStatus::Degraded => 1,
				HealthStatus::Unhealthy => 2,
			})
			.unwrap_or(HealthStatus::Healthy);

		Self { status, checks }
	}

	/// Check if the overall status is healthy
	pub fn is_healthy(&self) -> bool {
		self.status == HealthStatus::Healthy
	}

	/// Check if any component is unhealthy
	pub fn is_unhealthy(&self) -> bool {
		self.status == HealthStatus::Unhealthy
	}
}

/// Trait for implementing health checks
#[async_trait]
pub trait HealthCheck: Send + Sync {
	/// Perform the health check
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::{HealthCheck, HealthCheckResult};
	/// use async_trait::async_trait;
	///
	/// struct DatabaseHealthCheck;
	///
	/// #[async_trait]
	/// impl HealthCheck for DatabaseHealthCheck {
	///     async fn check(&self) -> HealthCheckResult {
	///         HealthCheckResult::healthy("database")
	///     }
	/// }
	/// ```
	async fn check(&self) -> HealthCheckResult;
}

/// Marker trait for cache-related health checks
pub trait CacheHealthCheck: HealthCheck {}

/// Marker trait for database-related health checks
pub trait DatabaseHealthCheck: HealthCheck {}

/// Manager for registering and running health checks
///
/// # Examples
///
/// ```
/// use reinhardt_utils::staticfiles::health::{HealthCheckManager, HealthCheck, HealthCheckResult};
/// use async_trait::async_trait;
/// use std::sync::Arc;
///
/// struct MyHealthCheck;
///
/// #[async_trait]
/// impl HealthCheck for MyHealthCheck {
///     async fn check(&self) -> HealthCheckResult {
///         HealthCheckResult::healthy("my_component")
///     }
/// }
///
/// # tokio_test::block_on(async {
/// let mut manager = HealthCheckManager::new();
/// manager.register("my_check", Arc::new(MyHealthCheck));
///
/// let report = manager.run_checks().await;
/// assert!(report.is_healthy());
/// # });
/// ```
#[derive(Default)]
pub struct HealthCheckManager {
	checks: HashMap<String, Arc<dyn HealthCheck>>,
}

impl HealthCheckManager {
	/// Create a new health check manager
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::HealthCheckManager;
	///
	/// let manager = HealthCheckManager::new();
	/// ```
	pub fn new() -> Self {
		Self {
			checks: HashMap::new(),
		}
	}

	/// Register a health check
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::{HealthCheckManager, HealthCheck, HealthCheckResult};
	/// use async_trait::async_trait;
	/// use std::sync::Arc;
	///
	/// struct DatabaseCheck;
	///
	/// #[async_trait]
	/// impl HealthCheck for DatabaseCheck {
	///     async fn check(&self) -> HealthCheckResult {
	///         HealthCheckResult::healthy("database")
	///     }
	/// }
	///
	/// let mut manager = HealthCheckManager::new();
	/// manager.register("database", Arc::new(DatabaseCheck));
	/// ```
	pub fn register(&mut self, name: impl Into<String>, check: Arc<dyn HealthCheck>) {
		self.checks.insert(name.into(), check);
	}

	/// Get the number of registered checks
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::HealthCheckManager;
	///
	/// let manager = HealthCheckManager::new();
	/// assert_eq!(manager.count(), 0);
	/// ```
	pub fn count(&self) -> usize {
		self.checks.len()
	}

	/// Run all registered health checks
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::staticfiles::health::{HealthCheckManager, HealthCheck, HealthCheckResult};
	/// use async_trait::async_trait;
	/// use std::sync::Arc;
	///
	/// struct SimpleCheck;
	///
	/// #[async_trait]
	/// impl HealthCheck for SimpleCheck {
	///     async fn check(&self) -> HealthCheckResult {
	///         HealthCheckResult::healthy("simple")
	///     }
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let mut manager = HealthCheckManager::new();
	/// manager.register("simple", Arc::new(SimpleCheck));
	///
	/// let report = manager.run_checks().await;
	/// assert_eq!(report.checks.len(), 1);
	/// # });
	/// ```
	pub async fn run_checks(&self) -> HealthReport {
		let mut results = Vec::new();

		for check in self.checks.values() {
			let result = check.check().await;
			results.push(result);
		}

		HealthReport::new(results)
	}
}
