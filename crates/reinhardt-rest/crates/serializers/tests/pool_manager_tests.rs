//! Tests for Connection Pool Manager
//!
//! These tests verify that ConnectionPoolManager provides thread-safe
//! connection pool management for high-concurrency scenarios.

#[cfg(feature = "django-compat")]
mod pool_tests {
    use reinhardt_serializers::ConnectionPoolManager;

    #[test]
    fn test_pool_manager_initialization() {
        // Clear any existing pool
        ConnectionPoolManager::clear();

        // Initially not initialized
        assert!(!ConnectionPoolManager::is_initialized());

        // Note: Actual pool initialization requires database connection,
        // which is tested in integration tests with TestContainers
    }

    #[test]
    fn test_pool_manager_clear() {
        // Clear should work even when not initialized
        ConnectionPoolManager::clear();
        assert!(!ConnectionPoolManager::is_initialized());

        // After clearing, should not be initialized
        ConnectionPoolManager::clear();
        assert!(!ConnectionPoolManager::is_initialized());
    }

    #[test]
    fn test_pool_manager_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        // Clear any existing pool
        ConnectionPoolManager::clear();

        // Spawn multiple threads trying to check initialization
        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    // All threads should see the same state
                    ConnectionPoolManager::is_initialized()
                })
            })
            .collect();

        // Wait for all threads
        let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All threads should return the same result
        assert!(results.iter().all(|&r| r == results[0]));
    }

    #[test]
    fn test_default_pool_config() {
        use reinhardt_serializers::pool_manager::default_pool_config;

        let config = default_pool_config();

        // Verify default values
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert!(config.acquire_timeout.as_secs() == 5);
    }
}

#[cfg(not(feature = "django-compat"))]
mod no_pool_tests {
    #[test]
    fn test_pool_manager_without_feature() {
        // ConnectionPoolManager should still be importable
        // but most functionality is not available without django-compat
        use reinhardt_serializers::ConnectionPoolManager;
        let _manager = ConnectionPoolManager::default();
    }
}
