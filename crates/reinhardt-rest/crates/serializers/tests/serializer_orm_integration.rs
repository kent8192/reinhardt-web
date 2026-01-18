//! Integration tests for Serializers + ORM integration
//!
//! This test file verifies the integration between ModelSerializer and ORM QuerySet:
//! - Database retrieval and serialization
//! - Deserialization and database saving
//! - Create and update operations via ORM
//!
//! ## Test Coverage
//!
//! 1. **ModelSerializer ↔ ORM QuerySet Integration**
//!    - Create operation via QuerySet
//!    - Update operation via Manager
//!    - Retrieve and serialize from database
//!
//! 2. **Data Validation and Transformation**
//!    - Field validation before database save
//!    - Type conversion during serialization
//!    - Error handling for invalid data
//!
//! ## Fixtures
//!
//! - `postgres_container`: PostgreSQL container for ORM integration tests

use reinhardt_db::orm::manager::init_database;
use reinhardt_db::orm::query::QuerySet;
use reinhardt_rest::serializers::queryset_integration::SerializerSaveMixin;
use reinhardt_rest::serializers::{SerializerError, ValidatorError};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_orm::{ConnectionTrait, Database, DbErr};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

/// Test user model for serializer integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
	pub id: Option<i64>,
	pub username: String,
	pub email: String,
	pub age: i32,
	pub is_active: bool,
}

reinhardt_test::impl_test_model!(User, i64, "users");

/// User serializer implementing SerializerSaveMixin
struct UserSerializer;

impl UserSerializer {
	async fn validate_username(username: &str) -> Result<(), SerializerError> {
		if username.len() < 3 {
			return Err(SerializerError::Validation(ValidatorError::Custom {
				message: format!(
					"Username must be at least 3 characters, got {}",
					username.len()
				),
			}));
		}
		if username.len() > 50 {
			return Err(SerializerError::Validation(ValidatorError::Custom {
				message: format!(
					"Username must be at most 50 characters, got {}",
					username.len()
				),
			}));
		}
		Ok(())
	}

	async fn validate_email(email: &str) -> Result<(), SerializerError> {
		if !email.contains('@') {
			return Err(SerializerError::Validation(ValidatorError::Custom {
				message: "Email must contain @".to_string(),
			}));
		}
		Ok(())
	}

	async fn validate_age(age: i32) -> Result<(), SerializerError> {
		if age < 0 {
			return Err(SerializerError::Validation(ValidatorError::Custom {
				message: "Age must be non-negative".to_string(),
			}));
		}
		if age > 150 {
			return Err(SerializerError::Validation(ValidatorError::Custom {
				message: "Age must be at most 150".to_string(),
			}));
		}
		Ok(())
	}
}

#[async_trait::async_trait]
impl SerializerSaveMixin for UserSerializer {
	type Model = User;

	async fn validate_for_create(data: &Value) -> Result<(), SerializerError> {
		if let Some(username) = data.get("username").and_then(|v| v.as_str()) {
			Self::validate_username(username).await?;
		}

		if let Some(email) = data.get("email").and_then(|v| v.as_str()) {
			Self::validate_email(email).await?;
		}

		if let Some(age) = data.get("age").and_then(|v| v.as_i64()) {
			Self::validate_age(age as i32).await?;
		}

		Ok(())
	}

	async fn validate_for_update(
		data: &Value,
		instance: Option<&Self::Model>,
	) -> Result<(), SerializerError> {
		if let Some(user) = instance {
			// Don't allow username changes for active users
			if user.is_active && data.get("username").is_some() {
				return Err(SerializerError::Validation(ValidatorError::Custom {
					message: "Cannot change username for active users".to_string(),
				}));
			}
		}

		// Validate individual fields
		if let Some(username) = data.get("username").and_then(|v| v.as_str()) {
			Self::validate_username(username).await?;
		}

		if let Some(email) = data.get("email").and_then(|v| v.as_str()) {
			Self::validate_email(email).await?;
		}

		if let Some(age) = data.get("age").and_then(|v| v.as_i64()) {
			Self::validate_age(age as i32).await?;
		}

		Ok(())
	}
}

/// Helper to create users table
async fn create_users_table(database_url: &str) -> Result<(), DbErr> {
	let conn = Database::connect(database_url).await?;

	conn.execute_unprepared(
		"CREATE TABLE IF NOT EXISTS users (
			id BIGSERIAL PRIMARY KEY,
			username TEXT NOT NULL,
			email TEXT NOT NULL,
			age INTEGER NOT NULL,
			is_active BOOLEAN NOT NULL DEFAULT TRUE
		)",
	)
	.await?;

	Ok(())
}

/// Test creating a user via SerializerSaveMixin::create
#[rstest]
#[tokio::test]
async fn test_serializer_create_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Create user via serializer
	let data = json!({
		"username": "alice",
		"email": "alice@example.com",
		"age": 30,
		"is_active": true
	});

	let user = UserSerializer::create(data).await.unwrap();

	assert_eq!(user.username, "alice");
	assert_eq!(user.email, "alice@example.com");
	assert_eq!(user.age, 30);
	assert!(user.is_active);
	assert!(user.id.is_some());
}

/// Test updating a user via SerializerSaveMixin::update
#[rstest]
#[tokio::test]
async fn test_serializer_update_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Create initial user
	let create_data = json!({
		"username": "bob",
		"email": "bob@example.com",
		"age": 25,
		"is_active": false
	});

	let user = UserSerializer::create(create_data).await.unwrap();

	// Update user
	let update_data = json!({
		"email": "bob.new@example.com",
		"age": 26
	});

	let updated_user = UserSerializer::update(user.clone(), update_data)
		.await
		.unwrap();

	assert_eq!(updated_user.username, "bob");
	assert_eq!(updated_user.email, "bob.new@example.com");
	assert_eq!(updated_user.age, 26);
	assert!(!updated_user.is_active);
	assert_eq!(updated_user.id, user.id);
}

/// Test validation failure on create
#[rstest]
#[tokio::test]
async fn test_serializer_create_validation_failure(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Invalid username (too short)
	let data = json!({
		"username": "ab",
		"email": "test@example.com",
		"age": 30,
		"is_active": true
	});

	let result = UserSerializer::create(data).await;
	assert!(result.is_err());
	match result {
		Err(SerializerError::Validation(ValidatorError::Custom { message })) => {
			assert!(message.contains("at least 3 characters"));
		}
		_ => panic!("Expected ValidationError"),
	}
}

/// Test validation failure on update (active user username change)
#[rstest]
#[tokio::test]
async fn test_serializer_update_validation_failure(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Create active user
	let create_data = json!({
		"username": "charlie",
		"email": "charlie@example.com",
		"age": 28,
		"is_active": true
	});

	let user = UserSerializer::create(create_data).await.unwrap();

	// Try to change username of active user
	let update_data = json!({
		"username": "charlie_new"
	});

	let result = UserSerializer::update(user, update_data).await;
	assert!(result.is_err());
	match result {
		Err(SerializerError::Validation(ValidatorError::Custom { message })) => {
			assert!(message.contains("Cannot change username for active users"));
		}
		_ => panic!("Expected ValidationError"),
	}
}

/// Test retrieving and serializing data from database
#[rstest]
#[tokio::test]
async fn test_serializer_retrieve_and_serialize(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Create user via serializer
	let data = json!({
		"username": "david",
		"email": "david@example.com",
		"age": 35,
		"is_active": true
	});

	let created_user = UserSerializer::create(data).await.unwrap();

	// Retrieve via ORM QuerySet
	let queryset = QuerySet::<User>::new();
	let users = queryset.all().await.unwrap();

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].username, "david");
	assert_eq!(users[0].email, "david@example.com");
	assert_eq!(users[0].age, 35);
	assert!(users[0].is_active);
	assert_eq!(users[0].id, created_user.id);

	// Serialize to JSON
	let serialized = serde_json::to_value(&users[0]).unwrap();
	assert_eq!(serialized["username"], "david");
	assert_eq!(serialized["email"], "david@example.com");
	assert_eq!(serialized["age"], 35);
	assert!(serialized["is_active"].as_bool().unwrap());
}

/// Test save method (create path)
#[rstest]
#[tokio::test]
async fn test_serializer_save_create(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Save without instance (create path)
	let data = json!({
		"username": "eve",
		"email": "eve@example.com",
		"age": 32,
		"is_active": false
	});

	let user = UserSerializer::save(data, None).await.unwrap();

	assert_eq!(user.username, "eve");
	assert_eq!(user.email, "eve@example.com");
	assert_eq!(user.age, 32);
	assert!(!user.is_active);
	assert!(user.id.is_some());
}

/// Test save method (update path)
#[rstest]
#[tokio::test]
async fn test_serializer_save_update(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Create user first
	let create_data = json!({
		"username": "frank",
		"email": "frank@example.com",
		"age": 40,
		"is_active": false
	});

	let user = UserSerializer::create(create_data).await.unwrap();

	// Save with instance (update path)
	let update_data = json!({
		"email": "frank.updated@example.com",
		"age": 41
	});

	let updated = UserSerializer::save(update_data, Some(user.clone()))
		.await
		.unwrap();

	assert_eq!(updated.username, "frank");
	assert_eq!(updated.email, "frank.updated@example.com");
	assert_eq!(updated.age, 41);
	assert!(!updated.is_active);
	assert_eq!(updated.id, user.id);
}

/// Test multiple users creation and retrieval
#[rstest]
#[tokio::test]
async fn test_serializer_multiple_users(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Create table
	create_users_table(&database_url).await.unwrap();
	init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Create multiple users
	let users_data = vec![
		json!({
			"username": "user1",
			"email": "user1@example.com",
			"age": 20,
			"is_active": true
		}),
		json!({
			"username": "user2",
			"email": "user2@example.com",
			"age": 25,
			"is_active": false
		}),
		json!({
			"username": "user3",
			"email": "user3@example.com",
			"age": 30,
			"is_active": true
		}),
	];

	for data in users_data {
		UserSerializer::create(data).await.unwrap();
	}

	// Retrieve all users
	let queryset = QuerySet::<User>::new();
	let users = queryset.all().await.unwrap();

	assert_eq!(users.len(), 3);

	// Verify usernames
	let usernames: Vec<String> = users.iter().map(|u| u.username.clone()).collect();
	assert!(usernames.contains(&"user1".to_string()));
	assert!(usernames.contains(&"user2".to_string()));
	assert!(usernames.contains(&"user3".to_string()));

	// Verify serialization of all users
	let serialized_users: Vec<Value> = users
		.iter()
		.map(|u| serde_json::to_value(u).unwrap())
		.collect();
	assert_eq!(serialized_users.len(), 3);
}
