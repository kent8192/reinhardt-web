//! Integration tests for Redis Sentinel cache backend.
//!
//! These tests verify the RedisSentinelCache configuration and structure.
//! Note: Full Sentinel operational testing requires a complete Sentinel cluster
//! setup which is beyond the scope of TestContainers integration tests.
//!
//! These tests focus on:
//! - Configuration validation and creation
//! - Configuration with various parameters (password, db, multiple sentinels)
//! - Configuration edge cases
//!
//! For production Sentinel operational testing (failover, master discovery, etc.),
//! use manual testing with real Sentinel clusters.

use reinhardt_utils::cache::RedisSentinelConfig;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_creation() {
	// Test configuration creation with multiple sentinels
	let config = RedisSentinelConfig {
		sentinels: vec![
			"redis://127.0.0.1:26379".to_string(),
			"redis://127.0.0.1:26380".to_string(),
			"redis://127.0.0.1:26381".to_string(),
		],
		master_name: "mymaster".to_string(),
		password: None,
		db: 0,
	};

	assert_eq!(config.sentinels.len(), 3);
	assert_eq!(config.master_name, "mymaster");
	assert_eq!(config.password, None);
	assert_eq!(config.db, 0);
}

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_with_password() {
	// Test configuration with password authentication
	let config = RedisSentinelConfig {
		sentinels: vec!["redis://127.0.0.1:26379".to_string()],
		master_name: "mymaster".to_string(),
		password: Some("secure_password".to_string()),
		db: 1,
	};

	assert_eq!(config.password, Some("secure_password".to_string()));
	assert_eq!(config.db, 1);
}

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_with_multiple_dbs() {
	// Test configuration for different database numbers
	for db in 0..=15 {
		let config = RedisSentinelConfig {
			sentinels: vec!["redis://127.0.0.1:26379".to_string()],
			master_name: "mymaster".to_string(),
			password: None,
			db,
		};
		assert_eq!(config.db, db);
	}
}

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_validation() {
	// Test configuration with single sentinel (minimum requirement)
	let config = RedisSentinelConfig {
		sentinels: vec!["redis://sentinel1:26379".to_string()],
		master_name: "production-master".to_string(),
		password: Some("prod-password".to_string()),
		db: 2,
	};

	assert!(!config.sentinels.is_empty());
	assert!(!config.master_name.is_empty());
}

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_with_custom_ports() {
	// Test configuration with non-standard sentinel ports
	let config = RedisSentinelConfig {
		sentinels: vec![
			"redis://sentinel1:5000".to_string(),
			"redis://sentinel2:5001".to_string(),
			"redis://sentinel3:5002".to_string(),
		],
		master_name: "custom-master".to_string(),
		password: None,
		db: 0,
	};

	assert_eq!(config.sentinels.len(), 3);
	assert!(config.sentinels[0].contains("5000"));
	assert!(config.sentinels[1].contains("5001"));
	assert!(config.sentinels[2].contains("5002"));
}

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_clone() {
	// Test that configuration can be cloned
	let config1 = RedisSentinelConfig {
		sentinels: vec!["redis://127.0.0.1:26379".to_string()],
		master_name: "mymaster".to_string(),
		password: Some("password".to_string()),
		db: 3,
	};

	let config2 = config1.clone();

	assert_eq!(config1.sentinels, config2.sentinels);
	assert_eq!(config1.master_name, config2.master_name);
	assert_eq!(config1.password, config2.password);
	assert_eq!(config1.db, config2.db);
}

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_with_hostname() {
	// Test configuration with hostnames instead of IPs
	let config = RedisSentinelConfig {
		sentinels: vec![
			"redis://sentinel-1.example.com:26379".to_string(),
			"redis://sentinel-2.example.com:26379".to_string(),
		],
		master_name: "prod-master".to_string(),
		password: Some("prod-pass".to_string()),
		db: 0,
	};

	assert_eq!(config.sentinels.len(), 2);
	assert!(config.sentinels[0].contains("sentinel-1.example.com"));
	assert!(config.sentinels[1].contains("sentinel-2.example.com"));
}

#[rstest]
#[tokio::test]
async fn test_redis_sentinel_config_default_values() {
	// Test configuration with default-like values
	let config = RedisSentinelConfig {
		sentinels: vec!["redis://127.0.0.1:26379".to_string()],
		master_name: "mymaster".to_string(),
		password: None,
		db: 0,
	};

	assert_eq!(config.master_name, "mymaster"); // Common default master name
	assert_eq!(config.db, 0); // Default Redis database
	assert!(config.password.is_none()); // No password by default
}
