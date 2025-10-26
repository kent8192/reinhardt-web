//! Integration tests for composite primary key query execution
//!
//! Tests the `get_composite()` method implementation that executes
//! database queries for records with composite primary keys.

use reinhardt_macros::Model;
use reinhardt_orm::{composite_pk::PkValue, manager::init_database, Model as _, QuerySet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};

/// Test model with composite primary key
#[derive(Model, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[model(app_label = "test_app", table_name = "post_tags")]
struct PostTag {
    #[field(primary_key = true)]
    post_id: i64,

    #[field(primary_key = true)]
    tag_id: i64,

    #[field(max_length = 200)]
    description: String,
}

/// Another test model with composite primary key
#[derive(Model, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[model(app_label = "test_app", table_name = "user_roles")]
struct UserRole {
    #[field(primary_key = true)]
    user_id: i64,

    #[field(primary_key = true)]
    role_id: i64,

    #[field(max_length = 100, null = true)]
    granted_by: Option<String>,
}

/// Set up test database and create tables
async fn setup_test_db() -> (testcontainers::ContainerAsync<GenericImage>, String) {
    // Start PostgreSQL container
    let postgres = GenericImage::new("postgres", "16-alpine")
        .with_env_var("POSTGRES_PASSWORD", "test")
        .with_env_var("POSTGRES_DB", "test_db")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get PostgreSQL port");

    let database_url = format!("postgres://postgres:test@localhost:{}/test_db", port);

    // Initialize database connection
    init_database(&database_url)
        .await
        .expect("Failed to initialize database");

    // Create tables
    let conn = reinhardt_orm::manager::get_connection()
        .await
        .expect("Failed to get connection");

    // Create post_tags table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS post_tags (
            post_id BIGINT NOT NULL,
            tag_id BIGINT NOT NULL,
            description VARCHAR(200) NOT NULL,
            PRIMARY KEY (post_id, tag_id)
        )",
    )
    .await
    .expect("Failed to create post_tags table");

    // Create user_roles table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_roles (
            user_id BIGINT NOT NULL,
            role_id BIGINT NOT NULL,
            granted_by VARCHAR(100),
            PRIMARY KEY (user_id, role_id)
        )",
    )
    .await
    .expect("Failed to create user_roles table");

    (postgres, database_url)
}

#[tokio::test]
async fn test_get_composite_success() {
    let (_container, _url) = setup_test_db().await;

    // Insert test data
    let conn = reinhardt_orm::manager::get_connection()
        .await
        .expect("Failed to get connection");

    conn.execute(
        "INSERT INTO post_tags (post_id, tag_id, description) VALUES (1, 10, 'First tag')",
    )
    .await
    .expect("Failed to insert test data");

    // Query using composite primary key
    let queryset = QuerySet::<PostTag>::new();
    let mut pk_values = HashMap::new();
    pk_values.insert("post_id".to_string(), PkValue::Int(1));
    pk_values.insert("tag_id".to_string(), PkValue::Int(10));

    let result = queryset.get_composite(&pk_values).await;
    assert!(result.is_ok(), "Query should succeed");

    let post_tag = result.unwrap();
    assert_eq!(post_tag.post_id, 1);
    assert_eq!(post_tag.tag_id, 10);
    assert_eq!(post_tag.description, "First tag");
}

#[tokio::test]
async fn test_get_composite_not_found() {
    let (_container, _url) = setup_test_db().await;

    // Query for non-existent record
    let queryset = QuerySet::<PostTag>::new();
    let mut pk_values = HashMap::new();
    pk_values.insert("post_id".to_string(), PkValue::Int(999));
    pk_values.insert("tag_id".to_string(), PkValue::Int(999));

    let result = queryset.get_composite(&pk_values).await;
    assert!(result.is_err(), "Query should fail for non-existent record");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("No record found"),
        "Error should indicate record not found"
    );
}

#[tokio::test]
async fn test_get_composite_missing_pk_field() {
    let (_container, _url) = setup_test_db().await;

    // Query with missing primary key field
    let queryset = QuerySet::<PostTag>::new();
    let mut pk_values = HashMap::new();
    pk_values.insert("post_id".to_string(), PkValue::Int(1));
    // Missing tag_id

    let result = queryset.get_composite(&pk_values).await;
    assert!(result.is_err(), "Query should fail with missing PK field");

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("validation failed") || error.to_string().contains("missing"),
        "Error should indicate validation failure: {}",
        error
    );
}

#[tokio::test]
async fn test_get_composite_with_optional_field() {
    let (_container, _url) = setup_test_db().await;

    // Insert test data
    let conn = reinhardt_orm::manager::get_connection()
        .await
        .expect("Failed to get connection");

    conn.execute("INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (1, 5, 'admin')")
        .await
        .expect("Failed to insert test data");

    conn.execute("INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (2, 5, NULL)")
        .await
        .expect("Failed to insert test data with NULL");

    // Query record with optional field present
    let queryset = QuerySet::<UserRole>::new();
    let mut pk_values = HashMap::new();
    pk_values.insert("user_id".to_string(), PkValue::Int(1));
    pk_values.insert("role_id".to_string(), PkValue::Int(5));

    let result = queryset.get_composite(&pk_values).await;
    assert!(result.is_ok(), "Query should succeed");

    let user_role = result.unwrap();
    assert_eq!(user_role.user_id, 1);
    assert_eq!(user_role.role_id, 5);
    assert_eq!(user_role.granted_by, Some("admin".to_string()));

    // Query record with optional field NULL
    let mut pk_values_null = HashMap::new();
    pk_values_null.insert("user_id".to_string(), PkValue::Int(2));
    pk_values_null.insert("role_id".to_string(), PkValue::Int(5));

    let result_null = queryset.get_composite(&pk_values_null).await;
    assert!(result_null.is_ok(), "Query should succeed for NULL field");

    let user_role_null = result_null.unwrap();
    assert_eq!(user_role_null.user_id, 2);
    assert_eq!(user_role_null.role_id, 5);
    assert_eq!(user_role_null.granted_by, None);
}

#[tokio::test]
async fn test_get_composite_multiple_records() {
    let (_container, _url) = setup_test_db().await;

    // Insert multiple records with same partial key (shouldn't happen with proper PK)
    // This tests the error handling for unexpected database states
    let conn = reinhardt_orm::manager::get_connection()
        .await
        .expect("Failed to get connection");

    conn.execute("INSERT INTO post_tags (post_id, tag_id, description) VALUES (10, 20, 'First')")
        .await
        .expect("Failed to insert test data");

    conn.execute("INSERT INTO post_tags (post_id, tag_id, description) VALUES (10, 21, 'Second')")
        .await
        .expect("Failed to insert test data");

    // Query should succeed for unique composite key
    let queryset = QuerySet::<PostTag>::new();
    let mut pk_values = HashMap::new();
    pk_values.insert("post_id".to_string(), PkValue::Int(10));
    pk_values.insert("tag_id".to_string(), PkValue::Int(20));

    let result = queryset.get_composite(&pk_values).await;
    assert!(
        result.is_ok(),
        "Query should succeed for unique composite PK"
    );

    let post_tag = result.unwrap();
    assert_eq!(post_tag.post_id, 10);
    assert_eq!(post_tag.tag_id, 20);
    assert_eq!(post_tag.description, "First");
}

#[tokio::test]
async fn test_get_composite_string_pk() {
    let (_container, _url) = setup_test_db().await;

    // Create table with string PK component
    let conn = reinhardt_orm::manager::get_connection()
        .await
        .expect("Failed to get connection");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS string_composite (
            category VARCHAR(50) NOT NULL,
            item_id BIGINT NOT NULL,
            value TEXT,
            PRIMARY KEY (category, item_id)
        )",
    )
    .await
    .expect("Failed to create string_composite table");

    conn.execute(
        "INSERT INTO string_composite (category, item_id, value) VALUES ('electronics', 100, 'Laptop')",
    )
    .await
    .expect("Failed to insert test data");

    // Define temporary model
    #[derive(Model, Serialize, Deserialize, Clone, Debug)]
    #[model(app_label = "test_app", table_name = "string_composite")]
    struct StringComposite {
        #[field(primary_key = true, max_length = 50)]
        category: String,

        #[field(primary_key = true)]
        item_id: i64,

        #[field(null = true)]
        value: Option<String>,
    }

    // Query with string PK
    let queryset = QuerySet::<StringComposite>::new();
    let mut pk_values = HashMap::new();
    pk_values.insert(
        "category".to_string(),
        PkValue::String("electronics".to_string()),
    );
    pk_values.insert("item_id".to_string(), PkValue::Int(100));

    let result = queryset.get_composite(&pk_values).await;
    assert!(result.is_ok(), "Query should succeed with string PK");

    let record = result.unwrap();
    assert_eq!(record.category, "electronics");
    assert_eq!(record.item_id, 100);
    assert_eq!(record.value, Some("Laptop".to_string()));
}
