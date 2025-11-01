//! Common test utilities and helpers

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize logging for tests (call once)
pub fn init_test_logging() {
	INIT.call_once(|| {
		let _ = env_logger::builder().is_test(true).try_init();
	});
}

#[cfg(feature = "dynamic-redis")]
pub mod redis_helpers {
	use reinhardt_settings::backends::RedisBackend;
	use testcontainers::{Container, GenericImage, ImageExt, runners::SyncRunner};

	pub struct RedisContainer {
		pub container: Container<GenericImage>,
		pub url: String,
	}

	/// Start a Redis container for testing
	pub fn start_redis() -> RedisContainer {
		let container = GenericImage::new("redis", "7-alpine")
			.with_wait_for(testcontainers::core::WaitFor::message_on_stdout(
				"Ready to accept connections",
			))
			.start()
			.expect("Failed to start Redis container");

		let port = container.get_host_port_ipv4(6379);
		let url = format!("redis://127.0.0.1:{}", port);

		RedisContainer { container, url }
	}

	/// Create a Redis backend for testing
	pub async fn create_redis_backend(url: &str) -> RedisBackend {
		RedisBackend::new(url)
			.await
			.expect("Failed to create Redis backend")
	}
}

#[cfg(feature = "dynamic-database")]
pub mod database_helpers {
	use reinhardt_settings::backends::DatabaseBackend;
	use testcontainers::{Container, GenericImage, ImageExt, runners::SyncRunner};

	pub struct PostgresContainer {
		pub container: Container<GenericImage>,
		pub url: String,
	}

	/// Start a PostgreSQL container for testing
	pub fn start_postgres() -> PostgresContainer {
		let container = GenericImage::new("postgres", "16-alpine")
			.with_env_var("POSTGRES_PASSWORD", "test")
			.with_env_var("POSTGRES_USER", "test")
			.with_env_var("POSTGRES_DB", "test")
			.with_wait_for(testcontainers::core::WaitFor::message_on_stderr(
				"database system is ready to accept connections",
			))
			.start()
			.expect("Failed to start PostgreSQL container");

		let port = container.get_host_port_ipv4(5432);
		let url = format!("postgres://test:test@127.0.0.1:{}/test", port);

		PostgresContainer { container, url }
	}

	/// Create a database backend for testing
	pub async fn create_database_backend(url: &str) -> DatabaseBackend {
		DatabaseBackend::new(url)
			.await
			.expect("Failed to create database backend")
	}

	/// Create an in-memory SQLite database for testing
	pub async fn create_sqlite_backend() -> DatabaseBackend {
		DatabaseBackend::new("sqlite::memory:")
			.await
			.expect("Failed to create SQLite backend")
	}
}

/// Generate a random test key
pub fn random_test_key() -> String {
	use uuid::Uuid;
	format!("test_key_{}", Uuid::new_v4().simple())
}

/// Generate test configuration data
pub fn test_config_value(value: &str) -> serde_json::Value {
	serde_json::json!({
		"value": value,
		"timestamp": chrono::Utc::now().to_rfc3339(),
	})
}
