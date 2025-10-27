//! Database Integration Tests
//!
//! These tests only run when reinhardt is available from crates.io.
//! The conditional compilation is handled by build.rs.

use example_test_macros::example_test;
use std::env;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
mod tests_with_reinhardt {
    use super::*;
    use reinhardt::prelude::*;

    /// Database connection test
    #[example_test(version = "^0.1")]
    async fn test_database_connection() {
        // Check if DATABASE_URL environment variable is set
        let database_url =
            env::var("DATABASE_URL").expect("DATABASE_URL must be set for database tests");

        assert!(database_url.contains("postgres"), "Should use PostgreSQL");

        // Actual connection test (using reinhardt's API)
        let db = reinhardt::Database::connect(&database_url).await;
        assert!(db.is_ok(), "Failed to connect to database");

        println!("✅ Database connection successful");
    }

    /// Table existence test
    #[example_test(version = "^0.1")]
    async fn test_users_table_exists() {
        // Check if users table exists
        let db = get_test_database().await;
        let result = db.query("SELECT * FROM users LIMIT 1").await;
        assert!(result.is_ok(), "users table should exist");

        println!("✅ Users table exists");
    }

    /// CRUD operations test
    #[example_test(version = ">=0.1.0, <0.2.0")]
    async fn test_crud_operations() {
        let db = get_test_database().await;

        // Create
        let user = User {
            id: None,
            name: "Test User".into(),
            email: "test@example.com".into(),
        };
        let created = db.insert(&user).await;
        assert!(created.is_ok(), "Failed to create user");

        // Read
        let users = db
            .query("SELECT * FROM users WHERE email = $1", &["test@example.com"])
            .await;
        assert!(users.is_ok(), "Failed to read users");

        // Update
        let updated = db
            .execute(
                "UPDATE users SET name = $1 WHERE email = $2",
                &["Updated User", "test@example.com"],
            )
            .await;
        assert!(updated.is_ok(), "Failed to update user");

        // Delete
        let deleted = db
            .execute(
                "DELETE FROM users WHERE email = $1",
                &["test@example.com"],
            )
            .await;
        assert!(deleted.is_ok(), "Failed to delete user");

        println!("✅ CRUD operations successful");
    }

    async fn get_test_database() -> reinhardt::Database {
        let database_url =
            env::var("DATABASE_URL").expect("DATABASE_URL must be set for database tests");
        reinhardt::Database::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[derive(Debug)]
    struct User {
        id: Option<i64>,
        name: String,
        email: String,
    }
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
mod tests_without_reinhardt {
    use super::*;

    /// Placeholder test that always passes when reinhardt is unavailable
    #[example_test(version = "^0.1")]
    async fn test_placeholder() {
        println!("⚠️  Database tests require reinhardt from crates.io");
        println!("   Tests will be enabled once reinhardt 0.1.x is published");
    }
}
