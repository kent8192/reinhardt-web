//! Common utilities and setup for validator integration tests
//!
//! This module provides shared functionality for testing validators
//! in integration with ORM and Serializers.

use reinhardt_core::{macros::model, validators::ValidationResult};
use reinhardt_db::{
	DatabaseConnection,
	orm::{FilterOperator, FilterValue, Model},
};
use std::sync::Arc;
use testcontainers::{
	GenericImage, ImageExt,
	core::{ContainerPort, WaitFor},
	runners::AsyncRunner,
};

/// Test database setup and management with TestContainers
pub struct TestDatabase {
	pub connection: Arc<DatabaseConnection>,
	/// Container is managed by the TestDatabase and should not be directly accessed.
	/// This field is public to allow fixture construction in integration tests.
	pub _container: Option<testcontainers::ContainerAsync<GenericImage>>,
}

impl TestDatabase {
	/// Create a new test database connection with automatic PostgreSQL container
	///
	/// Uses TestContainers to automatically start a PostgreSQL container for testing.
	/// Falls back to TEST_DATABASE_URL if the container fails to start.
	///
	/// **Note**: This does NOT apply migrations. Each test fixture should call
	/// `apply_basic_test_migrations()` from the migrations module.
	pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
		// Try to use TestContainers first
		let (database_url, container): (String, Option<_>) =
			if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
				// Use existing database if TEST_DATABASE_URL is set
				(url, None)
			} else {
				// Start PostgreSQL container using TestContainers
				let postgres_image = GenericImage::new("postgres", "17-alpine")
					.with_wait_for(WaitFor::message_on_stderr(
						"database system is ready to accept connections",
					))
					.with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust");

				let container = postgres_image.start().await?;
				let port = container
					.get_host_port_ipv4(ContainerPort::Tcp(5432))
					.await?;
				let url = format!("postgres://postgres@127.0.0.1:{}/postgres", port);
				(url, Some(container))
			};

		// Create DatabaseConnection
		let connection = DatabaseConnection::connect(&database_url)
			.await
			.map_err(|e| {
				format!(
					"Failed to connect to test database at '{}'. Error: {}",
					database_url, e
				)
			})?;

		Ok(Self {
			connection: Arc::new(connection),
			_container: container,
		})
	}

	/// Clean up test data (TRUNCATE tables, preserve schema)
	///
	/// Uses TRUNCATE instead of DROP to preserve schema for other tests.
	/// Resets IDENTITY columns and cascades to dependent tables.
	pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
		// Use raw SQL for TRUNCATE (not available in ORM yet)

		let _ = self
			.connection
			.execute(
				"TRUNCATE TABLE test_orders, test_comments, test_posts, test_products, test_users RESTART IDENTITY CASCADE",
				vec![],
			)
			.await;
		Ok(())
	}

	/// Insert test user using ORM
	pub async fn insert_user(
		&self,
		username: &str,
		email: &str,
	) -> Result<i32, Box<dyn std::error::Error>> {
		let user = TestUser::new(username.to_string(), email.to_string());
		let manager = TestUser::objects();
		let created = manager.create(&user).await?;
		Ok(created.id())
	}

	/// Check if username exists using ORM
	pub async fn username_exists(
		&self,
		username: &str,
	) -> Result<bool, Box<dyn std::error::Error>> {
		let manager = TestUser::objects();
		let count = manager
			.filter(
				"username",
				FilterOperator::Eq,
				FilterValue::String(username.to_string()),
			)
			.count()
			.await?;
		Ok(count > 0)
	}

	/// Check if email exists using ORM
	pub async fn email_exists(&self, email: &str) -> Result<bool, Box<dyn std::error::Error>> {
		let manager = TestUser::objects();
		let count = manager
			.filter(
				"email",
				FilterOperator::Eq,
				FilterValue::String(email.to_string()),
			)
			.count()
			.await?;
		Ok(count > 0)
	}

	/// Insert test product using ORM
	pub async fn insert_product(
		&self,
		name: &str,
		code: &str,
		price: f64,
		stock: i32,
	) -> Result<i32, Box<dyn std::error::Error>> {
		let product = TestProduct::new(name.to_string(), code.to_string(), price, stock);
		let manager = TestProduct::objects();
		let created = manager.create(&product).await?;
		Ok(created.id())
	}

	/// Check if user exists by ID using ORM
	pub async fn user_exists(&self, user_id: i32) -> Result<bool, Box<dyn std::error::Error>> {
		let manager = TestUser::objects();
		let count = manager
			.filter(
				"id",
				FilterOperator::Eq,
				FilterValue::Integer(user_id as i64),
			)
			.count()
			.await?;
		Ok(count > 0)
	}

	/// Check if product exists by ID using ORM
	pub async fn product_exists(
		&self,
		product_id: i32,
	) -> Result<bool, Box<dyn std::error::Error>> {
		let manager = TestProduct::objects();
		let count = manager
			.filter(
				"id",
				FilterOperator::Eq,
				FilterValue::Integer(product_id as i64),
			)
			.count()
			.await?;
		Ok(count > 0)
	}

	/// Insert test order (with FK constraints) using ORM
	pub async fn insert_order(
		&self,
		user_id: i32,
		product_id: i32,
		quantity: i32,
	) -> Result<i32, Box<dyn std::error::Error>> {
		let order = TestOrder::new(user_id, product_id, quantity);
		let manager = TestOrder::objects();
		let created = manager.create(&order).await?;
		Ok(created.id())
	}
}

/// Test user model for validation tests
#[model(table_name = "test_users")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestUser {
	#[field(primary_key = true)]
	id: i32,
	#[field(max_length = 100)]
	username: String,
	#[field(max_length = 255)]
	email: String,
}

impl TestUser {
	pub fn with_id(mut self, id: i32) -> Self {
		self.id = id;
		self
	}
}

/// Test product model for validation tests
#[model(table_name = "test_products")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestProduct {
	#[field(primary_key = true)]
	id: i32,
	#[field(max_length = 200)]
	name: String,
	#[field(max_length = 50)]
	code: String,
	price: f64,
	stock: i32,
}

/// Test order model for validation tests
#[model(table_name = "test_orders")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestOrder {
	#[field(primary_key = true)]
	id: i32,
	user_id: i32,
	product_id: i32,
	quantity: i32,
}

/// Helper function to validate and assert expected result
pub fn assert_validation_result<T: std::fmt::Debug>(
	result: ValidationResult<T>,
	should_pass: bool,
	error_message_contains: Option<&str>,
) {
	if should_pass {
		assert!(
			result.is_ok(),
			"Expected validation to pass, but got error: {:?}",
			result.err()
		);
	} else {
		assert!(
			result.is_err(),
			"Expected validation to fail, but it passed"
		);
		if let Some(msg) = error_message_contains {
			let error = result.unwrap_err();
			let error_str = error.to_string();
			assert!(
				error_str.contains(msg),
				"Expected error message to contain '{}', but got: {}",
				msg,
				error_str
			);
		}
	}
}
