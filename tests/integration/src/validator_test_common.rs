//! Common utilities and setup for validator integration tests
//!
//! This module provides shared functionality for testing validators
//! in integration with ORM and Serializers.

// use reinhardt_orm::connection::Connection; // Not available in current version
// use reinhardt_orm::model::Model; // Not used in current tests
use reinhardt_validators::ValidationResult;
use sqlx::PgPool;
use std::sync::Arc;

/// Test database setup and management
pub struct TestDatabase {
    pub pool: Arc<PgPool>,
}

impl TestDatabase {
    /// Create a new test database connection
    ///
    /// # Usage
    ///
    /// Set TEST_DATABASE_URL environment variable to use an existing database:
    /// ```bash
    /// TEST_DATABASE_URL=postgres://postgres@localhost:5432/postgres cargo test
    /// ```
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost:5432/postgres".to_string());

        let pool = PgPool::connect(&database_url).await?;
        Ok(Self {
            pool: Arc::new(pool),
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

#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt_validators::{EmailValidator, MaxLengthValidator, MinLengthValidator};

    #[test]
    fn test_validation_result_helper() {
        let validator = MinLengthValidator::new(5);

        // Test passing validation
        let result = validator.validate("hello");
        assert_validation_result(result, true, None);

        // Test failing validation
        let result = validator.validate("hi");
        assert_validation_result(result, false, Some("too short"));
    }

    #[test]
    fn test_test_user_creation() {
        let user = TestUser::new("testuser".to_string(), "test@example.com".to_string());
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert!(user.id.is_none());

        let user_with_id = user.with_id(42);
        assert_eq!(user_with_id.id, Some(42));
    }

    #[test]
    fn test_test_product_creation() {
        let product =
            TestProduct::new("Test Product".to_string(), "PROD001".to_string(), 99.99, 10);
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.code, "PROD001");
        assert_eq!(product.price, 99.99);
        assert_eq!(product.stock, 10);
    }
}
