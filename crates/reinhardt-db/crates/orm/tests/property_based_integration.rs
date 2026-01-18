//! Property-Based Integration Tests
//!
//! Tests critical database invariants using property-based testing with reinhardt-orm:
//! - Transaction commit → Committed state
//! - Soft delete → is_deleted=true
//! - Timestamp invariants: updated_at >= created_at
//! - Count aggregates: COUNT(*) >= COUNT(column)
//! - UNIQUE constraint violations always fail
//! - CASCADE DELETE → children deleted
//!
//! **Test Categories**: Property-based invariant verification, constraint enforcement
//!
//! **Fixtures Used**:
//! - postgres_container: PostgreSQL database container

use chrono::{DateTime, Utc};
use reinhardt_db::orm::manager::{get_connection, init_database};
use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue};
use reinhardt_db::orm::{Model, SoftDeletable, Timestamped};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Model Definitions (using reinhardt-orm Model trait)
// ============================================================================

/// User model with soft delete and timestamp support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
	pub id: Option<i32>,
	pub email: String,
	pub is_deleted: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl User {
	pub fn new(email: String) -> Self {
		let now = Utc::now();
		Self {
			id: None,
			email,
			is_deleted: false,
			created_at: now,
			updated_at: now,
		}
	}
}

reinhardt_test::impl_test_model!(User, i32, "users", "users");

impl SoftDeletable for User {
	fn deleted_at(&self) -> Option<DateTime<Utc>> {
		if self.is_deleted {
			Some(self.updated_at)
		} else {
			None
		}
	}

	fn set_deleted_at(&mut self, time: Option<DateTime<Utc>>) {
		self.is_deleted = time.is_some();
		if let Some(t) = time {
			self.updated_at = t;
		}
	}
}

impl Timestamped for User {
	fn created_at(&self) -> DateTime<Utc> {
		self.created_at
	}

	fn updated_at(&self) -> DateTime<Utc> {
		self.updated_at
	}

	fn set_updated_at(&mut self, time: DateTime<Utc>) {
		self.updated_at = time;
	}
}

/// Post model with soft delete and timestamp support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
	pub id: Option<i32>,
	pub user_id: i32,
	pub title: String,
	pub is_deleted: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl Post {
	pub fn new(user_id: i32, title: String) -> Self {
		let now = Utc::now();
		Self {
			id: None,
			user_id,
			title,
			is_deleted: false,
			created_at: now,
			updated_at: now,
		}
	}
}

reinhardt_test::impl_test_model!(Post, i32, "posts", "posts");

impl SoftDeletable for Post {
	fn deleted_at(&self) -> Option<DateTime<Utc>> {
		if self.is_deleted {
			Some(self.updated_at)
		} else {
			None
		}
	}

	fn set_deleted_at(&mut self, time: Option<DateTime<Utc>>) {
		self.is_deleted = time.is_some();
		if let Some(t) = time {
			self.updated_at = t;
		}
	}
}

impl Timestamped for Post {
	fn created_at(&self) -> DateTime<Utc> {
		self.created_at
	}

	fn updated_at(&self) -> DateTime<Utc> {
		self.updated_at
	}

	fn set_updated_at(&mut self, time: DateTime<Utc>) {
		self.updated_at = time;
	}
}

/// Comment model with timestamp support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
	pub id: Option<i32>,
	pub post_id: i32,
	pub user_id: i32,
	pub content: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl Comment {
	pub fn new(post_id: i32, user_id: i32, content: String) -> Self {
		let now = Utc::now();
		Self {
			id: None,
			post_id,
			user_id,
			content,
			created_at: now,
			updated_at: now,
		}
	}
}

reinhardt_test::impl_test_model!(Comment, i32, "comments", "comments");

impl Timestamped for Comment {
	fn created_at(&self) -> DateTime<Utc> {
		self.created_at
	}

	fn updated_at(&self) -> DateTime<Utc> {
		self.updated_at
	}

	fn set_updated_at(&mut self, time: DateTime<Utc>) {
		self.updated_at = time;
	}
}

/// Product model with timestamp support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
	pub id: Option<i32>,
	pub name: String,
	pub price: Option<f64>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl Product {
	pub fn new(name: String, price: Option<f64>) -> Self {
		let now = Utc::now();
		Self {
			id: None,
			name,
			price,
			created_at: now,
			updated_at: now,
		}
	}
}

reinhardt_test::impl_test_model!(Product, i32, "products", "products");

impl Timestamped for Product {
	fn created_at(&self) -> DateTime<Utc> {
		self.created_at
	}

	fn updated_at(&self) -> DateTime<Utc> {
		self.updated_at
	}

	fn set_updated_at(&mut self, time: DateTime<Utc>) {
		self.updated_at = time;
	}
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create users table with soft delete support
async fn setup_users_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			email VARCHAR(255) NOT NULL UNIQUE,
			is_deleted BOOLEAN NOT NULL DEFAULT false,
			created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create users table");
}

/// Create posts table with soft delete and FK to users
async fn setup_posts_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS posts (
			id SERIAL PRIMARY KEY,
			user_id INTEGER NOT NULL,
			title TEXT NOT NULL,
			is_deleted BOOLEAN NOT NULL DEFAULT false,
			created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
			CONSTRAINT fk_posts_user_id FOREIGN KEY (user_id)
				REFERENCES users(id) ON DELETE CASCADE
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create posts table");
}

/// Create comments table with FK to posts and users
async fn setup_comments_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS comments (
			id SERIAL PRIMARY KEY,
			post_id INTEGER NOT NULL,
			user_id INTEGER NOT NULL,
			content TEXT NOT NULL,
			created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
			CONSTRAINT fk_comments_post_id FOREIGN KEY (post_id)
				REFERENCES posts(id) ON DELETE CASCADE,
			CONSTRAINT fk_comments_user_id FOREIGN KEY (user_id)
				REFERENCES users(id) ON DELETE CASCADE
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create comments table");
}

/// Create products table with optional price field
async fn setup_products_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS products (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			price DOUBLE PRECISION,
			created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create products table");
}

// ============================================================================
// Property-Based Tests
// ============================================================================

/// Test that transaction commit changes state from uncommitted to committed
///
/// **Test Intent**: Verify that when a transaction commits successfully,
/// the affected data persists and is visible to subsequent queries
///
/// **Integration Point**: Manager create → Data persistence
///
/// **Not Testing**: Rollback behavior, isolation levels
#[rstest]
#[tokio::test]
async fn test_transaction_commit_persists_state(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	// Insert user using Manager API
	let manager = User::objects();
	let user = User::new("user@example.com".to_string());
	let created_user = manager
		.create_with_conn(&conn, &user)
		.await
		.expect("Failed to create user");

	// Verify user exists after commit
	let user_id = created_user.id.expect("User should have ID");
	let found_users = manager
		.get(user_id)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch user");

	assert_eq!(
		found_users.len(),
		1,
		"User should exist after transaction commit"
	);
	assert_eq!(found_users[0].email, "user@example.com");
}

/// Test that soft delete sets the is_deleted flag
///
/// **Test Intent**: Verify that soft deleting a record correctly sets is_deleted=true
///
/// **Integration Point**: SoftDeletable trait → is_deleted flag
///
/// **Not Testing**: Physical deletion, permanent removal
#[rstest]
#[tokio::test]
async fn test_soft_delete_sets_is_deleted_flag(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();

	// Create user
	let mut user = User::new("user@example.com".to_string());
	user = manager
		.create_with_conn(&conn, &user)
		.await
		.expect("Failed to create user");

	// Verify user is not deleted initially
	assert!(!user.is_deleted(), "User should not be deleted initially");

	// Soft delete user
	user.set_deleted_at(Some(Utc::now()));
	user = manager
		.update_with_conn(&conn, &user)
		.await
		.expect("Failed to soft delete user");

	// Verify is_deleted flag is set
	assert!(user.is_deleted(), "User should be soft deleted");
	assert_eq!(user.is_deleted, true, "is_deleted flag should be true");
}

/// Test timestamp invariant: updated_at >= created_at
///
/// **Test Intent**: Verify that updated_at is always >= created_at
///
/// **Integration Point**: Timestamped trait → Timestamp consistency
///
/// **Not Testing**: Timezone handling, precision
#[rstest]
#[tokio::test]
async fn test_updated_at_gte_created_at_invariant(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();

	// Create user
	let mut user = User::new("user@example.com".to_string());
	user = manager
		.create_with_conn(&conn, &user)
		.await
		.expect("Failed to create user");

	// Verify created_at <= updated_at initially
	assert!(
		user.updated_at() >= user.created_at(),
		"updated_at should be >= created_at at creation"
	);

	// Update user
	tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
	user.email = "updated@example.com".to_string();
	user.set_updated_at(Utc::now());
	user = manager
		.update_with_conn(&conn, &user)
		.await
		.expect("Failed to update user");

	// Verify updated_at >= created_at after update
	assert!(
		user.updated_at() >= user.created_at(),
		"updated_at should be >= created_at after update"
	);
}

/// Test COUNT(*) >= COUNT(column) invariant
///
/// **Test Intent**: Verify that COUNT(*) is always >= COUNT(column) for nullable columns
///
/// **Integration Point**: Aggregation functions → NULL handling
///
/// **Not Testing**: Other aggregate functions, complex queries
#[rstest]
#[tokio::test]
async fn test_count_star_gte_count_column(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();

	// Insert products: some with price, some without
	manager
		.create_with_conn(&conn, &Product::new("Product 1".to_string(), Some(10.0)))
		.await
		.expect("Failed to create product");
	manager
		.create_with_conn(&conn, &Product::new("Product 2".to_string(), None))
		.await
		.expect("Failed to create product");
	manager
		.create_with_conn(&conn, &Product::new("Product 3".to_string(), Some(20.0)))
		.await
		.expect("Failed to create product");

	// Count all products
	let all_products = manager
		.all()
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch all products");
	let count_star = all_products.len();

	// Count products with non-null price
	let count_price = all_products.iter().filter(|p| p.price.is_some()).count();

	// Verify COUNT(*) >= COUNT(price)
	assert!(
		count_star >= count_price,
		"COUNT(*) should be >= COUNT(price): {} >= {}",
		count_star,
		count_price
	);
	assert_eq!(count_star, 3, "Should have 3 total products");
	assert_eq!(count_price, 2, "Should have 2 products with price");
}

/// Test that UNIQUE constraint violations always fail
///
/// **Test Intent**: Verify that attempting to insert duplicate values in UNIQUE column fails
///
/// **Integration Point**: UNIQUE constraint → Error on violation
///
/// **Not Testing**: Other constraints, composite unique keys
#[rstest]
#[tokio::test]
async fn test_unique_constraint_always_fails_on_violation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();

	// Insert first user
	let user1 = User::new("duplicate@example.com".to_string());
	manager
		.create_with_conn(&conn, &user1)
		.await
		.expect("Failed to create first user");

	// Attempt to insert user with same email (should fail)
	let user2 = User::new("duplicate@example.com".to_string());
	let result = manager.create_with_conn(&conn, &user2).await;

	// Verify constraint violation
	assert!(
		result.is_err(),
		"Duplicate email should violate UNIQUE constraint"
	);
}

/// Test that CASCADE DELETE removes child records
///
/// **Test Intent**: Verify that deleting a parent record automatically
/// deletes all child records when CASCADE is configured
///
/// **Integration Point**: Foreign key constraint with CASCADE → Child deletion
///
/// **Not Testing**: Restrict constraints, set null behavior
#[rstest]
#[tokio::test]
async fn test_cascade_delete_removes_children(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;
	setup_posts_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let user_manager = User::objects();
	let post_manager = Post::objects();

	// Insert user
	let user = User::new("author@example.com".to_string());
	let created_user = user_manager
		.create_with_conn(&conn, &user)
		.await
		.expect("Failed to create user");
	let user_id = created_user.id.expect("User should have ID");

	// Insert 3 posts for that user
	for i in 0..3 {
		let post = Post::new(user_id, format!("Post {}", i));
		post_manager
			.create_with_conn(&conn, &post)
			.await
			.expect("Failed to create post");
	}

	// Verify posts exist
	let user_id_filter = Filter::new(
		"user_id".to_string(),
		FilterOperator::Eq,
		FilterValue::Int(user_id as i64),
	);
	let posts_before = post_manager
		.filter_by(user_id_filter.clone())
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch posts");
	assert_eq!(posts_before.len(), 3, "Should have 3 posts before deletion");

	// Delete user (should cascade)
	user_manager
		.delete_with_conn(&conn, user_id)
		.await
		.expect("Failed to delete user");

	// Verify all posts are deleted
	let posts_after = post_manager
		.filter_by(user_id_filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch posts after deletion");
	assert_eq!(
		posts_after.len(),
		0,
		"All posts should be deleted via CASCADE"
	);
}

/// Test multi-level CASCADE DELETE
///
/// **Test Intent**: Verify that CASCADE DELETE works across multiple levels
/// (User → Posts → Comments)
///
/// **Integration Point**: Multi-level foreign key CASCADE → Deep deletion
///
/// **Not Testing**: Circular references, self-referencing cascades
#[rstest]
#[tokio::test]
async fn test_cascade_delete_multi_level(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;
	setup_posts_table(pool.as_ref()).await;
	setup_comments_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let user_manager = User::objects();
	let post_manager = Post::objects();
	let comment_manager = Comment::objects();

	// Create user
	let user = User::new("author@example.com".to_string());
	let created_user = user_manager
		.create_with_conn(&conn, &user)
		.await
		.expect("Failed to create user");
	let user_id = created_user.id.expect("User should have ID");

	// Create post
	let post = Post::new(user_id, "Post Title".to_string());
	let created_post = post_manager
		.create_with_conn(&conn, &post)
		.await
		.expect("Failed to create post");
	let post_id = created_post.id.expect("Post should have ID");

	// Create 2 comments
	for i in 0..2 {
		let comment = Comment::new(post_id, user_id, format!("Comment {}", i));
		comment_manager
			.create_with_conn(&conn, &comment)
			.await
			.expect("Failed to create comment");
	}

	// Verify comments exist
	let post_id_filter = Filter::new(
		"post_id".to_string(),
		FilterOperator::Eq,
		FilterValue::Int(post_id as i64),
	);
	let comments_before = comment_manager
		.filter_by(post_id_filter.clone())
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch comments");
	assert_eq!(
		comments_before.len(),
		2,
		"Should have 2 comments before deletion"
	);

	// Delete user (should cascade to posts and comments)
	user_manager
		.delete_with_conn(&conn, user_id)
		.await
		.expect("Failed to delete user");

	// Verify all comments are deleted
	let comments_after = comment_manager
		.filter_by(post_id_filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch comments after deletion");
	assert_eq!(
		comments_after.len(),
		0,
		"All comments should be deleted via multi-level CASCADE"
	);
}

/// Test that soft delete filters only include active records
///
/// **Test Intent**: Verify that filtering by is_deleted=false returns only active records
///
/// **Integration Point**: SoftDeletable trait + filtering → Active records only
///
/// **Not Testing**: Deleted record retrieval, restore operations
#[rstest]
#[tokio::test]
async fn test_soft_delete_filters_active_records(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();

	// Create 3 users
	let mut user1 = User::new("user1@example.com".to_string());
	let mut user2 = User::new("user2@example.com".to_string());
	let mut user3 = User::new("user3@example.com".to_string());

	user1 = manager
		.create_with_conn(&conn, &user1)
		.await
		.expect("Failed to create user1");
	user2 = manager
		.create_with_conn(&conn, &user2)
		.await
		.expect("Failed to create user2");
	user3 = manager
		.create_with_conn(&conn, &user3)
		.await
		.expect("Failed to create user3");

	// Soft delete user2
	user2.set_deleted_at(Some(Utc::now()));
	manager
		.update_with_conn(&conn, &user2)
		.await
		.expect("Failed to soft delete user2");

	// Filter active users only
	let active_filter = Filter::new(
		"is_deleted".to_string(),
		FilterOperator::Eq,
		FilterValue::Bool(false),
	);
	let active_users = manager
		.filter_by(active_filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch active users");

	// Verify only 2 active users
	assert_eq!(active_users.len(), 2, "Should have 2 active users");

	let active_ids: Vec<i32> = active_users.iter().filter_map(|u| u.id).collect();
	assert!(active_ids.contains(&user1.id.unwrap()));
	assert!(active_ids.contains(&user3.id.unwrap()));
	assert!(!active_ids.contains(&user2.id.unwrap()));
}

/// Test that timestamps are created on insert
///
/// **Test Intent**: Verify that created_at and updated_at are set on record creation
///
/// **Integration Point**: Timestamped trait → Automatic timestamp creation
///
/// **Not Testing**: Manual timestamp override, timezone handling
#[rstest]
#[tokio::test]
async fn test_timestamps_created_on_insert(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();

	// Create user
	let user = User::new("user@example.com".to_string());
	let created_user = manager
		.create_with_conn(&conn, &user)
		.await
		.expect("Failed to create user");

	// Verify timestamps are set
	assert!(created_user.created_at() > DateTime::<Utc>::MIN_UTC);
	assert!(created_user.updated_at() > DateTime::<Utc>::MIN_UTC);
	assert!(created_user.updated_at() >= created_user.created_at());
}

/// Test that committed records are visible across different connections
///
/// **Test Intent**: Verify that committed data is immediately visible to other connections
///
/// **Integration Point**: Transaction isolation → Read committed visibility
///
/// **Not Testing**: Uncommitted data visibility, specific isolation levels
#[rstest]
#[tokio::test]
async fn test_records_visible_across_connections(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();

	// Insert user
	let user = User::new("user@example.com".to_string());
	let created_user = manager
		.create_with_conn(&conn, &user)
		.await
		.expect("Failed to create user");
	let user_id = created_user.id.expect("User should have ID");

	// Fetch using the same connection (should see the record)
	let found_users = manager
		.get(user_id)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch user");
	assert_eq!(found_users.len(), 1, "Should find user in same connection");

	// Reinitialize connection (simulating new connection)
	reinhardt_db::orm::manager::reinitialize_database(&url)
		.await
		.expect("Failed to reinitialize database");
	let new_conn = get_connection()
		.await
		.expect("Failed to get new connection");

	// Fetch using new connection (should still see the committed record)
	let found_users_new = manager
		.get(user_id)
		.all_with_db(&new_conn)
		.await
		.expect("Failed to fetch user with new connection");
	assert_eq!(
		found_users_new.len(),
		1,
		"Should find user in new connection (committed data is visible)"
	);
}
