//! ModelSerializer validator integration tests
//!
//! Tests for UniqueValidator and UniqueTogetherValidator with database integration

use reinhardt_integration_tests::{cleanup_test_tables, setup_test_db};
use reinhardt_orm::Model;
use reinhardt_serializers::{ModelSerializer, UniqueTogetherValidator, UniqueValidator};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Row};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestUser {
    id: Option<i64>,
    username: String,
    email: String,
}

impl Model for TestUser {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "test_users"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestArticle {
    id: Option<i64>,
    slug: String,
    year: i32,
    title: String,
}

impl Model for TestArticle {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "test_articles"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

async fn create_test_users_table(pool: &Pool<Postgres>) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS test_users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(255) UNIQUE NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create test_users table");
}

async fn create_test_articles_table(pool: &Pool<Postgres>) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS test_articles (
            id SERIAL PRIMARY KEY,
            slug VARCHAR(255) NOT NULL,
            year INTEGER NOT NULL,
            title VARCHAR(255) NOT NULL,
            UNIQUE(slug, year)
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create test_articles table");
}

async fn insert_test_user(pool: &Pool<Postgres>, username: &str, email: &str) -> i64 {
    sqlx::query("INSERT INTO test_users (username, email) VALUES ($1, $2) RETURNING id")
        .bind(username)
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("Failed to insert test user")
        .get("id")
}

async fn insert_test_article(pool: &Pool<Postgres>, slug: &str, year: i32, title: &str) -> i64 {
    sqlx::query("INSERT INTO test_articles (slug, year, title) VALUES ($1, $2, $3) RETURNING id")
        .bind(slug)
        .bind(year)
        .bind(title)
        .fetch_one(pool)
        .await
        .expect("Failed to insert test article")
        .get("id")
}

#[tokio::test]
async fn test_unique_validator_for_new_instance() {
    let pool = setup_test_db().await;
    create_test_users_table(&pool).await;

    // Insert existing user
    insert_test_user(&pool, "alice", "alice@example.com").await;

    let validator = UniqueValidator::<TestUser>::new("username");

    // Check that existing username fails validation
    let result = validator.validate(&pool, "alice", None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .message
        .contains("test_users with this username already exists"));

    // Check that new username passes validation
    let result = validator.validate(&pool, "bob", None).await;
    assert!(result.is_ok());

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_unique_validator_for_update() {
    let pool = setup_test_db().await;
    create_test_users_table(&pool).await;

    // Insert two users
    let alice_id = insert_test_user(&pool, "alice", "alice@example.com").await;
    insert_test_user(&pool, "bob", "bob@example.com").await;

    let validator = UniqueValidator::<TestUser>::new("username");

    // Updating alice to keep her own username should pass
    let result = validator.validate(&pool, "alice", Some(&alice_id)).await;
    assert!(result.is_ok());

    // Updating alice to bob's username should fail
    let result = validator.validate(&pool, "bob", Some(&alice_id)).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .message
        .contains("test_users with this username already exists"));

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_unique_validator_email_field() {
    let pool = setup_test_db().await;
    create_test_users_table(&pool).await;

    // Insert existing user
    insert_test_user(&pool, "alice", "alice@example.com").await;

    let validator = UniqueValidator::<TestUser>::new("email");

    // Check that existing email fails validation
    let result = validator.validate(&pool, "alice@example.com", None).await;
    assert!(result.is_err());

    // Check that new email passes validation
    let result = validator.validate(&pool, "bob@example.com", None).await;
    assert!(result.is_ok());

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_unique_together_validator_for_new_instance() {
    let pool = setup_test_db().await;
    create_test_articles_table(&pool).await;

    // Insert existing article
    insert_test_article(&pool, "my-article", 2025, "My Article").await;

    let validator = UniqueTogetherValidator::<TestArticle>::new(vec!["slug", "year"]);

    // Check that existing combination fails validation
    let mut values = HashMap::new();
    values.insert("slug".to_string(), "my-article".to_string());
    values.insert("year".to_string(), "2025".to_string());
    let result = validator.validate(&pool, &values, None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .message
        .contains("The fields slug, year must make a unique set"));

    // Check that different year passes validation
    values.insert("year".to_string(), "2024".to_string());
    let result = validator.validate(&pool, &values, None).await;
    assert!(result.is_ok());

    // Check that different slug passes validation
    values.insert("slug".to_string(), "other-article".to_string());
    values.insert("year".to_string(), "2025".to_string());
    let result = validator.validate(&pool, &values, None).await;
    assert!(result.is_ok());

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_unique_together_validator_for_update() {
    let pool = setup_test_db().await;
    create_test_articles_table(&pool).await;

    // Insert two articles
    let article1_id = insert_test_article(&pool, "my-article", 2025, "My Article").await;
    insert_test_article(&pool, "other-article", 2025, "Other Article").await;

    let validator = UniqueTogetherValidator::<TestArticle>::new(vec!["slug", "year"]);

    // Updating article1 to keep its own slug/year should pass
    let mut values = HashMap::new();
    values.insert("slug".to_string(), "my-article".to_string());
    values.insert("year".to_string(), "2025".to_string());
    let result = validator.validate(&pool, &values, Some(&article1_id)).await;
    assert!(result.is_ok());

    // Updating article1 to use existing slug/year combination should fail
    values.insert("slug".to_string(), "other-article".to_string());
    let result = validator.validate(&pool, &values, Some(&article1_id)).await;
    assert!(result.is_err());

    cleanup_test_tables(&pool).await;
}

#[tokio::test]
async fn test_model_serializer_validate() {
    let pool = setup_test_db().await;
    create_test_users_table(&pool).await;

    let serializer = ModelSerializer::<TestUser>::new();

    // Basic validation should pass
    let user = TestUser {
        id: None,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let result = serializer.validate(&user);
    assert!(result.is_ok());

    cleanup_test_tables(&pool).await;
}
