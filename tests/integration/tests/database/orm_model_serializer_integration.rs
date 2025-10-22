// ORM ModelSerializer Integration Tests with TestContainers
// Tests the integration between reinhardt-orm and reinhardt-serializers
// Uses shared PostgreSQL container with TRUNCATE-based cleanup for efficiency

use reinhardt_orm::Model;
use reinhardt_serializers::{
    DefaultModelSerializer, ModelSerializer, ModelSerializerBuilder, RelationshipStrategy,
    Serializer,
};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Pool, Postgres, Row};
use std::env;
use std::sync::Arc;
use testcontainers::{core::ContainerPort, runners::AsyncRunner, GenericImage, ImageExt};
use tokio::sync::OnceCell;

// Test model: User
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct User {
    id: Option<i64>,
    username: String,
    email: String,
    age: i32,
    is_active: bool,
}

impl Model for User {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "users"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

// Test model: Article
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Article {
    id: Option<i64>,
    title: String,
    content: String,
    author_id: i64,
    published: bool,
}

impl Model for Article {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "articles"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

// Global shared database pool and container (initialized once for all tests)
static DB_POOL: OnceCell<Arc<PgPool>> = OnceCell::const_new();
static CONTAINER: OnceCell<
    Arc<testcontainers::core::ContainerAsync<testcontainers::GenericImage>>,
> = OnceCell::const_new();

// Get or initialize the database pool
async fn get_db_pool() -> Arc<PgPool> {
    DB_POOL
        .get_or_init(|| async {
            let pool = setup_test_db_once().await;
            Arc::new(pool)
        })
        .await
        .clone()
}

// Retry database connection with exponential backoff
async fn retry_connect(url: &str, max_retries: u32) -> Pool<Postgres> {
    for i in 0..max_retries {
        tokio::time::sleep(tokio::time::Duration::from_millis(200 * (i + 1) as u64)).await;
        if let Ok(pool) = sqlx::pool::PoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(60))
            .idle_timeout(std::time::Duration::from_secs(1800))
            .max_lifetime(std::time::Duration::from_secs(3600))
            .connect_with(url.parse().unwrap())
            .await
        {
            return pool;
        }
    }
    panic!(
        "Failed to connect to testcontainer database after {} retries",
        max_retries
    );
}

// Create database tables
async fn create_tables(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            username VARCHAR(255) NOT NULL UNIQUE,
            email VARCHAR(255) NOT NULL,
            age INTEGER NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create users table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS articles (
            id BIGSERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            author_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            published BOOLEAN NOT NULL DEFAULT false
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create articles table");
}

// Setup test database once (called by OnceCell initialization)
async fn setup_test_db_once() -> PgPool {
    // Check for external database URL (for CI/CD)
    if let Ok(database_url) = env::var("TEST_DATABASE_URL") {
        let pool = sqlx::pool::PoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(60))
            .idle_timeout(std::time::Duration::from_secs(1800))
            .max_lifetime(std::time::Duration::from_secs(3600))
            .connect_with(database_url.parse().unwrap())
            .await
            .expect("Failed to connect to test database");
        create_tables(&pool).await;
        return pool;
    }

    // Start PostgreSQL container (only once for all tests)
    let container = GenericImage::new("postgres", "17-alpine")
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust") // No password required
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    // Store container reference to keep it alive
    CONTAINER
        .set(Arc::new(container))
        .expect("Container already initialized");

    let container_ref = CONTAINER.get().unwrap();
    let port = container_ref
        .get_host_port_ipv4(ContainerPort::Tcp(5432))
        .await
        .expect("Failed to get container port");

    let database_url = format!("postgres://postgres@localhost:{}/postgres", port);

    // Wait for database to be ready with retry logic
    let pool = retry_connect(&database_url, 10).await;

    // Create tables
    create_tables(&pool).await;

    pool
}

// Cleanup tables before each test with timeout handling
async fn cleanup_tables(pool: &PgPool) {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        sqlx::query("TRUNCATE users, articles RESTART IDENTITY CASCADE").execute(pool),
    )
    .await;

    match result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => panic!("Failed to cleanup tables: {}", e),
        Err(_) => {
            // Timeout occurred, try a simpler cleanup
            let _ = sqlx::query("DELETE FROM articles").execute(pool).await;
            let _ = sqlx::query("DELETE FROM users").execute(pool).await;
        }
    }
}

#[serial]
#[tokio::test]
async fn test_model_serializer_create_with_db() {
    cleanup_tables(&*get_db_pool().await).await;
    let serializer = DefaultModelSerializer::<User>::new();

    let user = User {
        id: None,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        age: 25,
        is_active: true,
    };

    // Insert into database
    let row = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&user.username)
    .bind(&user.email)
    .bind(user.age)
    .bind(user.is_active)
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    let id: i64 = row.get("id");

    // Verify serialization works
    let mut created_user = user.clone();
    created_user.id = Some(id);

    let serialized = Serializer::serialize(&serializer, &created_user).unwrap();
    assert!(serialized.len() > 0);

    let json_str = String::from_utf8(serialized).unwrap();
    assert!(json_str.contains("testuser"));
    assert!(json_str.contains("test@example.com"));
}

#[serial]
#[tokio::test]
async fn test_model_serializer_update_with_db() {
    cleanup_tables(&*get_db_pool().await).await;
    let serializer = DefaultModelSerializer::<User>::new();

    // Create initial user
    let row = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("olduser")
    .bind("old@example.com")
    .bind(30)
    .bind(true)
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    let id: i64 = row.get("id");

    let mut user = User {
        id: Some(id),
        username: "olduser".to_string(),
        email: "old@example.com".to_string(),
        age: 30,
        is_active: true,
    };

    // Update
    let updated_data = User {
        id: Some(id),
        username: "newuser".to_string(),
        email: "new@example.com".to_string(),
        age: 35,
        is_active: false,
    };

    sqlx::query(
        "UPDATE users SET username = $1, email = $2, age = $3, is_active = $4 WHERE id = $5",
    )
    .bind(&updated_data.username)
    .bind(&updated_data.email)
    .bind(updated_data.age)
    .bind(updated_data.is_active)
    .bind(id)
    .execute(&*get_db_pool().await)
    .await
    .unwrap();

    let result = serializer.update(&mut user, updated_data.clone());
    assert!(result.is_ok());
    assert_eq!(user, updated_data);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_relationship_primary_key() {
    cleanup_tables(&*get_db_pool().await).await;

    // Create user
    let user_row = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("author")
    .bind("author@example.com")
    .bind(40)
    .bind(true)
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    let user_id: i64 = user_row.get("id");

    // Create article with relationship
    let article = Article {
        id: None,
        title: "Test Article".to_string(),
        content: "Article content".to_string(),
        author_id: user_id,
        published: true,
    };

    let serializer = ModelSerializerBuilder::<Article>::new()
        .relationship_strategy(RelationshipStrategy::PrimaryKey)
        .build();

    let serialized = Serializer::serialize(&serializer, &article).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains(&user_id.to_string()));
    assert!(json_str.contains("Test Article"));
}

#[serial]
#[tokio::test]
async fn test_model_serializer_depth_configuration() {
    cleanup_tables(&*get_db_pool().await).await;

    let serializer_depth_0 = ModelSerializerBuilder::<User>::new().depth(0).build();

    let serializer_depth_2 = ModelSerializerBuilder::<User>::new().depth(2).build();

    let user = User {
        id: Some(1),
        username: "depthtest".to_string(),
        email: "depth@example.com".to_string(),
        age: 28,
        is_active: true,
    };

    let serialized_0 = Serializer::serialize(&serializer_depth_0, &user).unwrap();
    let serialized_2 = Serializer::serialize(&serializer_depth_2, &user).unwrap();

    // Both should serialize the basic fields
    assert!(serialized_0.len() > 0);
    assert!(serialized_2.len() > 0);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_validation() {
    cleanup_tables(&*get_db_pool().await).await;

    let serializer = ModelSerializerBuilder::<User>::new()
        .validate_unique(true)
        .validate_unique_together(true)
        .build();

    let user = User {
        id: Some(1),
        username: "validuser".to_string(),
        email: "valid@example.com".to_string(),
        age: 25,
        is_active: true,
    };

    let result = Serializer::serialize(&serializer, &user);
    assert!(result.is_ok());
}

#[serial]
#[tokio::test]
async fn test_model_serializer_complex_query() {
    cleanup_tables(&*get_db_pool().await).await;

    // Create user
    let user_row = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("complexuser")
    .bind("complex@example.com")
    .bind(30)
    .bind(true)
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    let user_id: i64 = user_row.get("id");

    // Create multiple articles
    for i in 1..=3 {
        sqlx::query(
            "INSERT INTO articles (title, content, author_id, published) VALUES ($1, $2, $3, $4)",
        )
        .bind(format!("Article {}", i))
        .bind(format!("Content {}", i))
        .bind(user_id)
        .bind(i % 2 == 0)
        .execute(&*get_db_pool().await)
        .await
        .unwrap();
    }

    // Query articles
    let articles: Vec<(i64, String, String, i64, bool)> = sqlx::query_as(
        "SELECT id, title, content, author_id, published FROM articles WHERE author_id = $1",
    )
    .bind(user_id)
    .fetch_all(&*get_db_pool().await)
    .await
    .unwrap();

    assert_eq!(articles.len(), 3);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_nested_serialization() {
    cleanup_tables(&*get_db_pool().await).await;

    let serializer = ModelSerializerBuilder::<Article>::new()
        .relationship_strategy(RelationshipStrategy::Nested)
        .depth(1)
        .build();

    let article = Article {
        id: Some(1),
        title: "Nested Test".to_string(),
        content: "Testing nested serialization".to_string(),
        author_id: 1,
        published: true,
    };

    let serialized = Serializer::serialize(&serializer, &article).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("Nested Test"));
    assert!(json_str.contains("author_id"));
}

#[serial]
#[tokio::test]
async fn test_model_serializer_partial_update() {
    cleanup_tables(&*get_db_pool().await).await;
    let serializer = DefaultModelSerializer::<User>::new();

    // Create user
    let row = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("partialuser")
    .bind("partial@example.com")
    .bind(26)
    .bind(true)
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    let id: i64 = row.get("id");

    let mut user = User {
        id: Some(id),
        username: "partialuser".to_string(),
        email: "partial@example.com".to_string(),
        age: 26,
        is_active: true,
    };

    // Partial update (only email)
    sqlx::query("UPDATE users SET email = $1 WHERE id = $2")
        .bind("updated@example.com")
        .bind(id)
        .execute(&*get_db_pool().await)
        .await
        .unwrap();

    let updated_data = User {
        id: Some(id),
        username: "partialuser".to_string(),
        email: "updated@example.com".to_string(),
        age: 26,
        is_active: true,
    };

    let result = serializer.update(&mut user, updated_data.clone());
    assert!(result.is_ok());
    assert_eq!(user.email, "updated@example.com");
}

#[serial]
#[tokio::test]
async fn test_model_serializer_transaction_rollback() {
    cleanup_tables(&*get_db_pool().await).await;

    let pool = get_db_pool().await;
    let mut tx = pool.begin().await.unwrap();

    // Insert user in transaction
    let result = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("txuser")
    .bind("tx@example.com")
    .bind(27)
    .bind(true)
    .fetch_one(&mut *tx)
    .await;

    assert!(result.is_ok());

    // Rollback
    tx.rollback().await.unwrap();

    // Verify user was not created
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = 'txuser'")
        .fetch_one(&*get_db_pool().await)
        .await
        .unwrap();

    assert_eq!(count, 0);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_transaction_commit() {
    cleanup_tables(&*get_db_pool().await).await;

    let pool = get_db_pool().await;
    let mut tx = pool.begin().await.unwrap();

    // Insert user in transaction
    let row = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("commituser")
    .bind("commit@example.com")
    .bind(28)
    .bind(true)
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    let id: i64 = row.get("id");

    // Commit
    tx.commit().await.unwrap();

    // Verify user was created
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&*get_db_pool().await)
        .await
        .unwrap();

    assert_eq!(count, 1);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_unique_constraint() {
    cleanup_tables(&*get_db_pool().await).await;

    // Create first user
    sqlx::query("INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4)")
        .bind("uniqueuser")
        .bind("unique@example.com")
        .bind(29)
        .bind(true)
        .execute(&*get_db_pool().await)
        .await
        .unwrap();

    // Try to create duplicate username
    let result =
        sqlx::query("INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4)")
            .bind("uniqueuser") // Duplicate username
            .bind("different@example.com")
            .bind(30)
            .bind(true)
            .execute(&*get_db_pool().await)
            .await;

    assert!(result.is_err());
}

#[serial]
#[tokio::test]
async fn test_model_serializer_foreign_key_constraint() {
    cleanup_tables(&*get_db_pool().await).await;

    // Try to create article with non-existent author
    let result = sqlx::query(
        "INSERT INTO articles (title, content, author_id, published) VALUES ($1, $2, $3, $4)",
    )
    .bind("Orphan Article")
    .bind("No author")
    .bind(99999) // Non-existent author_id
    .bind(false)
    .execute(&*get_db_pool().await)
    .await;

    assert!(result.is_err());
}

#[serial]
#[tokio::test]
async fn test_model_serializer_cascading_delete() {
    cleanup_tables(&*get_db_pool().await).await;

    // Create user
    let user_row = sqlx::query(
        "INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind("cascadeuser")
    .bind("cascade@example.com")
    .bind(31)
    .bind(true)
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    let user_id: i64 = user_row.get("id");

    // Create article
    sqlx::query(
        "INSERT INTO articles (title, content, author_id, published) VALUES ($1, $2, $3, $4)",
    )
    .bind("Cascade Test")
    .bind("Will be deleted")
    .bind(user_id)
    .bind(true)
    .execute(&*get_db_pool().await)
    .await
    .unwrap();

    // Delete user (should cascade to articles due to ON DELETE CASCADE)
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&*get_db_pool().await)
        .await
        .unwrap();

    // Verify articles were deleted
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE author_id = $1")
        .bind(user_id)
        .fetch_one(&*get_db_pool().await)
        .await
        .unwrap();

    assert_eq!(count, 0);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_json_round_trip() {
    cleanup_tables(&*get_db_pool().await).await;
    let serializer = DefaultModelSerializer::<User>::new();

    let user = User {
        id: Some(42),
        username: "jsonuser".to_string(),
        email: "json@example.com".to_string(),
        age: 32,
        is_active: true,
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let json_value: serde_json::Value = serde_json::from_slice(&serialized).unwrap();

    assert_eq!(json_value["id"], 42);
    assert_eq!(json_value["username"], "jsonuser");
    assert_eq!(json_value["email"], "json@example.com");
    assert_eq!(json_value["age"], 32);
    assert_eq!(json_value["is_active"], true);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_empty_result_set() {
    cleanup_tables(&*get_db_pool().await).await;

    let users: Vec<(i64, String, String, i32, bool)> =
        sqlx::query_as("SELECT id, username, email, age, is_active FROM users WHERE id = -1")
            .fetch_all(&*get_db_pool().await)
            .await
            .unwrap();

    assert_eq!(users.len(), 0);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_ordering() {
    cleanup_tables(&*get_db_pool().await).await;

    // Create users with different ages
    for age in [25, 30, 20, 35, 28] {
        sqlx::query("INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4)")
            .bind(format!("order{}", age))
            .bind(format!("order{}@example.com", age))
            .bind(age)
            .bind(true)
            .execute(&*get_db_pool().await)
            .await
            .unwrap();
    }

    // Query with ORDER BY
    let users: Vec<(i32,)> =
        sqlx::query_as("SELECT age FROM users WHERE username LIKE 'order%' ORDER BY age ASC")
            .fetch_all(&*get_db_pool().await)
            .await
            .unwrap();

    let ages: Vec<i32> = users.into_iter().map(|(age,)| age).collect();
    assert_eq!(ages, vec![20, 25, 28, 30, 35]);
}

#[serial]
#[tokio::test]
async fn test_model_serializer_filtering() {
    cleanup_tables(&*get_db_pool().await).await;

    // Create active and inactive users
    for (username, is_active) in [
        ("active1", true),
        ("active2", true),
        ("inactive1", false),
        ("inactive2", false),
    ] {
        sqlx::query("INSERT INTO users (username, email, age, is_active) VALUES ($1, $2, $3, $4)")
            .bind(username)
            .bind(format!("{}@example.com", username))
            .bind(25)
            .bind(is_active)
            .execute(&*get_db_pool().await)
            .await
            .unwrap();
    }

    // Query only active users
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE is_active = true AND username LIKE 'active%'",
    )
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    assert_eq!(count, 2);

    // Query only inactive users
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE is_active = false AND username LIKE 'inactive%'",
    )
    .fetch_one(&*get_db_pool().await)
    .await
    .unwrap();

    assert_eq!(count, 2);
}
