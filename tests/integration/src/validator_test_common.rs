//! Common utilities and setup for validator integration tests
//!
//! This module provides shared functionality for testing validators
//! in integration with ORM and Serializers.

// use reinhardt_orm::connection::Connection; // Not available in current version
// use reinhardt_orm::model::Model; // Not used in current tests
use reinhardt_validators::ValidationResult;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage, ImageExt};

/// Test database setup and management with TestContainers
pub struct TestDatabase {
    pub pool: Arc<PgPool>,
    _container: testcontainers::ContainerAsync<GenericImage>,
}

impl TestDatabase {
    /// Create a new test database connection with automatic PostgreSQL container
    ///
    /// Uses TestContainers to automatically start a PostgreSQL container for testing.
    /// Falls back to TEST_DATABASE_URL if the container fails to start.
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        use sqlx::postgres::PgPoolOptions;
        use std::time::Duration;

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
                let port = container.get_host_port_ipv4(5432).await?;
                let url = format!("postgres://postgres@127.0.0.1:{}/postgres", port);
                (url, Some(container))
            };

        // Configure pool with appropriate timeouts for testing
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(30))
            .max_lifetime(Duration::from_secs(300))
            .connect(&database_url)
            .await
            .map_err(|e| {
                format!(
                    "Failed to connect to test database at '{}'. Error: {}",
                    database_url, e
                )
            })?;

        // If we have a container, wrap it; otherwise create a dummy container
        let final_container = if let Some(c) = container {
            c
        } else {
            // For external database connections, create a dummy container
            // This won't actually be used, but satisfies the struct field requirement
            GenericImage::new("postgres", "17-alpine").start().await?
        };

        Ok(Self {
            pool: Arc::new(pool),
            _container: final_container,
        })
    }

    /// Create test tables for validator tests
    pub async fn setup_tables(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create users table for uniqueness tests
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_users (
                id SERIAL PRIMARY KEY,
                username VARCHAR(100) UNIQUE NOT NULL,
                email VARCHAR(255) UNIQUE NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(self.pool.as_ref())
        .await?;

        // Create products table for validation tests
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_products (
                id SERIAL PRIMARY KEY,
                name VARCHAR(200) NOT NULL,
                code VARCHAR(50) UNIQUE NOT NULL,
                price DECIMAL(10, 2) NOT NULL,
                stock INTEGER NOT NULL CHECK (stock >= 0),
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(self.pool.as_ref())
        .await?;

        // Create orders table for relationship tests
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_orders (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL REFERENCES test_users(id),
                product_id INTEGER NOT NULL REFERENCES test_products(id),
                quantity INTEGER NOT NULL CHECK (quantity > 0),
                order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, product_id)
            )
            "#,
        )
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    /// Clean up test data
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("DROP TABLE IF EXISTS test_orders CASCADE")
            .execute(self.pool.as_ref())
            .await?;
        sqlx::query("DROP TABLE IF EXISTS test_products CASCADE")
            .execute(self.pool.as_ref())
            .await?;
        sqlx::query("DROP TABLE IF EXISTS test_users CASCADE")
            .execute(self.pool.as_ref())
            .await?;
        Ok(())
    }

    /// Insert test user
    pub async fn insert_user(
        &self,
        username: &str,
        email: &str,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let row: (i32,) =
            sqlx::query_as("INSERT INTO test_users (username, email) VALUES ($1, $2) RETURNING id")
                .bind(username)
                .bind(email)
                .fetch_one(self.pool.as_ref())
                .await?;
        Ok(row.0)
    }

    /// Check if username exists
    pub async fn username_exists(
        &self,
        username: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM test_users WHERE username = $1")
            .bind(username)
            .fetch_one(self.pool.as_ref())
            .await?;
        Ok(count.0 > 0)
    }

    /// Check if email exists
    pub async fn email_exists(&self, email: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM test_users WHERE email = $1")
            .bind(email)
            .fetch_one(self.pool.as_ref())
            .await?;
        Ok(count.0 > 0)
    }
}

/// Test user model for validation tests
#[derive(Debug, Clone)]
pub struct TestUser {
    pub id: Option<i32>,
    pub username: String,
    pub email: String,
}

impl TestUser {
    pub fn new(username: String, email: String) -> Self {
        Self {
            id: None,
            username,
            email,
        }
    }

    pub fn with_id(mut self, id: i32) -> Self {
        self.id = Some(id);
        self
    }
}

/// Test product model for validation tests
#[derive(Debug, Clone)]
pub struct TestProduct {
    pub id: Option<i32>,
    pub name: String,
    pub code: String,
    pub price: f64,
    pub stock: i32,
}

impl TestProduct {
    pub fn new(name: String, code: String, price: f64, stock: i32) -> Self {
        Self {
            id: None,
            name,
            code,
            price,
            stock,
        }
    }
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
