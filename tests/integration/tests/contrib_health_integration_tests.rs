//! Health check integration tests
//!
//! Integration tests for health check functionality across reinhardt-contrib
//! and reinhardt-static. These tests verify the health monitoring system's
//! ability to report system status across multiple components.
//!
//! Based on production monitoring patterns from Django and other frameworks.

use async_trait::async_trait;
use reinhardt_static::{
    HealthCheck, HealthCheckManager, HealthCheckResult, HealthReport, HealthStatus,
};
use std::sync::Arc;

#[test]
fn test_health_status_display() {
    assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
    assert_eq!(format!("{}", HealthStatus::Degraded), "degraded");
    assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
}

#[test]
fn test_health_check_result_healthy() {
    let result = HealthCheckResult::healthy("database");
    assert_eq!(result.component, "database");
    assert_eq!(result.status, HealthStatus::Healthy);
    assert!(result.message.is_none());
    assert!(result.metadata.is_empty());
}

#[test]
fn test_health_check_result_degraded() {
    let result = HealthCheckResult::degraded("cache", "Slow response");
    assert_eq!(result.component, "cache");
    assert_eq!(result.status, HealthStatus::Degraded);
    assert_eq!(result.message, Some("Slow response".to_string()));
}

#[test]
fn test_health_check_result_unhealthy() {
    let result = HealthCheckResult::unhealthy("database", "Connection failed");
    assert_eq!(result.component, "database");
    assert_eq!(result.status, HealthStatus::Unhealthy);
    assert_eq!(result.message, Some("Connection failed".to_string()));
}

#[test]
fn test_health_check_result_with_metadata() {
    let result = HealthCheckResult::healthy("api")
        .with_metadata("response_time_ms", "45")
        .with_metadata("uptime_hours", "120");

    assert_eq!(result.metadata.len(), 2);
    assert_eq!(
        result.metadata.get("response_time_ms"),
        Some(&"45".to_string())
    );
    assert_eq!(
        result.metadata.get("uptime_hours"),
        Some(&"120".to_string())
    );
}

#[test]
fn test_health_report_all_healthy() {
    let checks = vec![
        HealthCheckResult::healthy("database"),
        HealthCheckResult::healthy("cache"),
        HealthCheckResult::healthy("storage"),
    ];
    let report = HealthReport::new(checks);

    assert_eq!(report.status, HealthStatus::Healthy);
    assert_eq!(report.checks.len(), 3);
    assert!(report.is_healthy());
    assert!(!report.is_unhealthy());
}

#[test]
fn test_health_report_with_degraded() {
    let checks = vec![
        HealthCheckResult::healthy("database"),
        HealthCheckResult::degraded("cache", "High latency"),
    ];
    let report = HealthReport::new(checks);

    assert_eq!(report.status, HealthStatus::Degraded);
    assert!(!report.is_healthy());
    assert!(!report.is_unhealthy());
}

#[test]
fn test_health_report_with_unhealthy() {
    let checks = vec![
        HealthCheckResult::healthy("database"),
        HealthCheckResult::degraded("cache", "Slow"),
        HealthCheckResult::unhealthy("storage", "Disk full"),
    ];
    let report = HealthReport::new(checks);

    assert_eq!(report.status, HealthStatus::Unhealthy);
    assert!(!report.is_healthy());
    assert!(report.is_unhealthy());
}

#[test]
fn test_health_report_empty() {
    let report = HealthReport::new(vec![]);
    assert_eq!(report.status, HealthStatus::Healthy);
    assert_eq!(report.checks.len(), 0);
    assert!(report.is_healthy());
}

// Mock health check for testing
struct MockHealthCheck {
    component: String,
    status: HealthStatus,
}

#[async_trait]
impl HealthCheck for MockHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        match self.status {
            HealthStatus::Healthy => HealthCheckResult::healthy(&self.component),
            HealthStatus::Degraded => HealthCheckResult::degraded(&self.component, "Degraded"),
            HealthStatus::Unhealthy => HealthCheckResult::unhealthy(&self.component, "Unhealthy"),
        }
    }
}

#[tokio::test]
async fn test_health_check_manager_default() {
    // Test: HealthCheckManager can be created with default configuration
    let manager = HealthCheckManager::new();

    // Verify no checks are registered initially
    assert_eq!(manager.count(), 0);

    // Verify running checks on empty manager returns healthy report
    let report = manager.run_checks().await;
    assert_eq!(report.status, HealthStatus::Healthy);
    assert_eq!(report.checks.len(), 0);
    assert!(report.is_healthy());
}

#[tokio::test]
async fn test_health_check_manager_with_single_check() {
    let mut manager = HealthCheckManager::new();

    let check = Arc::new(MockHealthCheck {
        component: "database".to_string(),
        status: HealthStatus::Healthy,
    });
    manager.register("database", check);

    assert_eq!(manager.count(), 1);

    let report = manager.run_checks().await;
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.status, HealthStatus::Healthy);
    assert!(report.is_healthy());
}

#[tokio::test]
async fn test_health_check_manager_with_multiple_checks() {
    let mut manager = HealthCheckManager::new();

    manager.register(
        "database",
        Arc::new(MockHealthCheck {
            component: "database".to_string(),
            status: HealthStatus::Healthy,
        }),
    );
    manager.register(
        "cache",
        Arc::new(MockHealthCheck {
            component: "cache".to_string(),
            status: HealthStatus::Healthy,
        }),
    );

    assert_eq!(manager.count(), 2);

    let report = manager.run_checks().await;
    assert_eq!(report.checks.len(), 2);
    assert!(report.is_healthy());
}

#[tokio::test]
async fn test_health_check_manager_with_failing_check() {
    let mut manager = HealthCheckManager::new();

    manager.register(
        "database",
        Arc::new(MockHealthCheck {
            component: "database".to_string(),
            status: HealthStatus::Unhealthy,
        }),
    );

    let report = manager.run_checks().await;
    assert_eq!(report.status, HealthStatus::Unhealthy);
    assert!(report.is_unhealthy());
}
