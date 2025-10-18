//! Tests for Hybrid Property DML Operations
//! Based on SQLAlchemy's DMLTest class

use reinhardt_hybrid::HybridProperty;
use reinhardt_orm::hybrid_dml::{InsertBuilder, UpdateBuilder};

/// Simple model for testing
#[derive(Debug)]
struct TestModel {
    id: i32,
    x: i32,
}

/// Point type for expanded hybrid testing
#[derive(Debug, Clone)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[test]
fn test_single_plain_update_values() {
    // Test simple UPDATE with hybrid property
    let x_plain = HybridProperty::new(|m: &TestModel| m.x);

    let builder = UpdateBuilder::new("a").set_hybrid("x", &x_plain, "10");

    let (sql, params) = builder.build();
    assert_eq!(sql, "UPDATE a SET x=?");
    assert_eq!(params, vec!["10"]);
}

#[test]
fn test_single_plain_insert_values() {
    // Test simple INSERT with hybrid property
    let x_plain = HybridProperty::new(|m: &TestModel| m.x);

    let builder = InsertBuilder::new("a").hybrid_value("x", &x_plain, "10");

    let (sql, params) = builder.build();
    assert_eq!(sql, "INSERT INTO a (x) VALUES (?)");
    assert_eq!(params, vec!["10"]);
}

#[test]
fn test_single_plain_bulk() {
    // Test bulk operations with simple hybrid property
    // This verifies that SQL generation works correctly for bulk operations
    let x_plain = HybridProperty::new(|m: &TestModel| m.x);

    // Test bulk INSERT
    let insert_builder = InsertBuilder::new("a")
        .hybrid_value("x", &x_plain, "5")
        .value("id", "1");

    let (sql, params) = insert_builder.build();
    assert!(sql.contains("INSERT INTO a"));
    assert!(sql.contains("x"));
    assert!(sql.contains("id"));
    assert_eq!(params.len(), 2);
    assert!(params.contains(&"5".to_string()));
    assert!(params.contains(&"1".to_string()));

    // Test bulk UPDATE
    let update_builder = UpdateBuilder::new("a")
        .set_hybrid("x", &x_plain, "10")
        .where_clause("id = 1");

    let (sql, params) = update_builder.build();
    assert_eq!(sql, "UPDATE a SET x=? WHERE id = 1");
    assert_eq!(params, vec!["10"]);
}

#[test]
fn test_expand_plain_update_values() {
    // Test UPDATE with expanded hybrid property (Point -> x, y)
    let builder = UpdateBuilder::new("a")
        .set_expanded(vec![("x", "5"), ("y", "6")])
        .where_clause("a.x = 10 AND a.y = 12");

    let (sql, params) = builder.build();
    assert!(sql.starts_with("UPDATE a SET"));
    assert!(sql.contains("x=?"));
    assert!(sql.contains("y=?"));
    assert!(sql.contains("WHERE a.x = 10 AND a.y = 12"));
    assert_eq!(params.len(), 2);
    assert!(params.contains(&"5".to_string()));
    assert!(params.contains(&"6".to_string()));
}

#[test]
fn test_expand_update_insert_values() {
    // Test INSERT with expanded hybrid property
    let builder = InsertBuilder::new("a").expanded_hybrid(vec![("x", "5"), ("y", "6")]);

    let (sql, params) = builder.build();
    assert!(sql.starts_with("INSERT INTO a"));
    assert!(sql.contains("x"));
    assert!(sql.contains("y"));
    assert_eq!(params.len(), 2);
    assert!(params.contains(&"5".to_string()));
    assert!(params.contains(&"6".to_string()));
}

#[test]
fn test_expand_update_update_values() {
    // Test UPDATE with expanded values
    let builder = UpdateBuilder::new("a").set_expanded(vec![("x", "5"), ("y", "6")]);

    let (sql, params) = builder.build();
    assert!(sql.contains("UPDATE a SET"));
    assert!(sql.contains("x=?"));
    assert!(sql.contains("y=?"));
    assert_eq!(params.len(), 2);
}

#[test]
fn test_derived_update_insert_values() {
    // Test INSERT with derived values
    let builder = InsertBuilder::new("a")
        .value("x", "5")
        .value("y", "6")
        .value("z", "11"); // derived: x + y

    let (sql, params) = builder.build();
    assert!(sql.contains("INSERT INTO a"));
    assert!(sql.contains("x"));
    assert!(sql.contains("y"));
    assert!(sql.contains("z"));
    assert_eq!(params.len(), 3);
}

#[test]
fn test_derived_update_update_values() {
    // Test UPDATE with derived values
    let builder = UpdateBuilder::new("a")
        .set("x", "5")
        .set("y", "6")
        .set("z", "11"); // derived: x + y

    let (sql, params) = builder.build();
    assert!(sql.contains("UPDATE a SET"));
    assert!(sql.contains("x=?"));
    assert!(sql.contains("y=?"));
    assert!(sql.contains("z=?"));
    assert_eq!(params.len(), 3);
}

#[test]
fn test_multiple_columns_insert() {
    // Test INSERT with multiple columns
    let builder = InsertBuilder::new("person")
        .value("first_name", "Dr.")
        .value("last_name", "No");

    let (sql, params) = builder.build();
    assert!(sql.contains("INSERT INTO person"));
    assert!(sql.contains("first_name"));
    assert!(sql.contains("last_name"));
    assert_eq!(params.len(), 2);
}

#[test]
fn test_multiple_columns_update() {
    // Test UPDATE with multiple columns
    let builder = UpdateBuilder::new("person")
        .set("first_name", "Dr.")
        .set("last_name", "No");

    let (sql, params) = builder.build();
    assert!(sql.contains("UPDATE person SET"));
    assert!(sql.contains("first_name=?"));
    assert!(sql.contains("last_name=?"));
    assert_eq!(params.len(), 2);
}

#[test]
fn test_update_with_where_clause() {
    // Test UPDATE with WHERE clause
    let builder = UpdateBuilder::new("person")
        .set("first_name", "Jane")
        .where_clause("id = 1");

    let (sql, params) = builder.build();
    assert!(sql.contains("UPDATE person SET first_name=? WHERE id = 1"));
    assert_eq!(params, vec!["Jane"]);
}

// NOTE: The following tests require actual database execution
// and are marked as integration tests

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use super::*;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_expand_dml_bulk_insert() {
        // Create in-memory database
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create table
        sqlx::query(
            "CREATE TABLE a (
                id INTEGER PRIMARY KEY,
                x INTEGER,
                y INTEGER
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert with expanded hybrid
        let builder = InsertBuilder::new("a")
            .value("id", "1")
            .expanded_hybrid(vec![("x", "3"), ("y", "4")]);

        let (sql, params) = builder.build();

        println!("SQL: {}", sql);
        println!("Params: {:?}", params);

        // Execute the query
        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }

        let result = query.execute(&pool).await;
        if let Err(e) = &result {
            println!("Error: {}", e);
        }
        assert!(result.is_ok());

        // Verify the data
        let row: (i32, i32, i32) = sqlx::query_as("SELECT id, x, y FROM a WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row, (1, 3, 4));
    }

    #[tokio::test]
    async fn test_expand_dml_bulk_update() {
        // Create in-memory database
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create table and insert data
        sqlx::query(
            "CREATE TABLE a (
                id INTEGER PRIMARY KEY,
                x INTEGER,
                y INTEGER
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("INSERT INTO a (id, x, y) VALUES (1, 3, 4)")
            .execute(&pool)
            .await
            .unwrap();

        // Update with expanded hybrid
        let builder = UpdateBuilder::new("a")
            .set_expanded(vec![("x", "10"), ("y", "9")])
            .where_clause("id = 1");

        let (sql, params) = builder.build();

        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }

        let result = query.execute(&pool).await;
        assert!(result.is_ok());

        // Verify the data
        let row: (i32, i32, i32) = sqlx::query_as("SELECT id, x, y FROM a WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row, (1, 10, 9));
    }

    #[tokio::test]
    async fn test_derived_dml_bulk() {
        // Create in-memory database
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create table
        sqlx::query(
            "CREATE TABLE a (
                id INTEGER PRIMARY KEY,
                x INTEGER,
                y INTEGER,
                z INTEGER
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert with derived value
        let builder = InsertBuilder::new("a")
            .value("id", "1")
            .value("x", "3")
            .value("y", "4")
            .value("z", "7"); // derived: x + y

        let (sql, params) = builder.build();

        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }

        let result = query.execute(&pool).await;
        assert!(result.is_ok());

        // Verify the data
        let row: (i32, i32, i32, i32) = sqlx::query_as("SELECT id, x, y, z FROM a WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row, (1, 3, 4, 7));
    }
}
