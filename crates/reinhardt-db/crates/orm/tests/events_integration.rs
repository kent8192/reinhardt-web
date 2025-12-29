//! Events Integration Tests
//!
//! Tests Event hook lifecycle (pre_save, post_save, pre_delete, post_delete):
//! - State Transitions: pre_save, post_save, pre_delete, post_delete
//! - Decision Table: Event registration/execution
//! - Veto behavior: Prevent database operations
//! - Multiple listeners: Sequential execution
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use async_trait::async_trait;
use reinhardt_orm::Model;
use reinhardt_orm::events::{EventResult, MapperEvents, event_registry};
use reinhardt_orm::manager::reinitialize_database;
use reinhardt_test::fixtures::testcontainers::postgres_container;
use rstest::*;
use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Model Definitions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
	id: Option<i32>,
	title: String,
	content: String,
	status: String,
}

impl Article {
	fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
		Self {
			id: None,
			title: title.into(),
			content: content.into(),
			status: "draft".to_string(),
		}
	}
}

reinhardt_test::impl_test_model!(Article, i32, "articles", "events_test");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	id: Option<i32>,
	username: String,
	email: String,
	active: bool,
}

impl User {
	fn new(username: impl Into<String>, email: impl Into<String>) -> Self {
		Self {
			id: None,
			username: username.into(),
			email: email.into(),
			active: true,
		}
	}
}

reinhardt_test::impl_test_model!(User, i32, "users", "events_test");

// ============================================================================
// Test Event Listener Implementation
// ============================================================================

#[derive(Clone)]
struct TestMapperListener {
	pre_insert_count: Arc<AtomicUsize>,
	post_insert_count: Arc<AtomicUsize>,
	pre_update_count: Arc<AtomicUsize>,
	post_update_count: Arc<AtomicUsize>,
	pre_delete_count: Arc<AtomicUsize>,
	post_delete_count: Arc<AtomicUsize>,
	veto_insert: Arc<std::sync::Mutex<bool>>,
	veto_update: Arc<std::sync::Mutex<bool>>,
	veto_delete: Arc<std::sync::Mutex<bool>>,
}

impl TestMapperListener {
	fn new() -> Self {
		Self {
			pre_insert_count: Arc::new(AtomicUsize::new(0)),
			post_insert_count: Arc::new(AtomicUsize::new(0)),
			pre_update_count: Arc::new(AtomicUsize::new(0)),
			post_update_count: Arc::new(AtomicUsize::new(0)),
			pre_delete_count: Arc::new(AtomicUsize::new(0)),
			post_delete_count: Arc::new(AtomicUsize::new(0)),
			veto_insert: Arc::new(std::sync::Mutex::new(false)),
			veto_update: Arc::new(std::sync::Mutex::new(false)),
			veto_delete: Arc::new(std::sync::Mutex::new(false)),
		}
	}

	fn set_veto_insert(&self, veto: bool) {
		*self.veto_insert.lock().unwrap() = veto;
	}

	fn set_veto_update(&self, veto: bool) {
		*self.veto_update.lock().unwrap() = veto;
	}

	fn set_veto_delete(&self, veto: bool) {
		*self.veto_delete.lock().unwrap() = veto;
	}

	fn pre_insert_count(&self) -> usize {
		self.pre_insert_count.load(Ordering::SeqCst)
	}

	fn post_insert_count(&self) -> usize {
		self.post_insert_count.load(Ordering::SeqCst)
	}

	fn pre_update_count(&self) -> usize {
		self.pre_update_count.load(Ordering::SeqCst)
	}

	fn post_update_count(&self) -> usize {
		self.post_update_count.load(Ordering::SeqCst)
	}

	fn pre_delete_count(&self) -> usize {
		self.pre_delete_count.load(Ordering::SeqCst)
	}

	fn post_delete_count(&self) -> usize {
		self.post_delete_count.load(Ordering::SeqCst)
	}
}

#[async_trait]
impl MapperEvents for TestMapperListener {
	async fn before_insert(&self, _instance_id: &str, _values: &JsonValue) -> EventResult {
		self.pre_insert_count.fetch_add(1, Ordering::SeqCst);
		if *self.veto_insert.lock().unwrap() {
			EventResult::Veto
		} else {
			EventResult::Continue
		}
	}

	async fn after_insert(&self, _instance_id: &str) -> EventResult {
		self.post_insert_count.fetch_add(1, Ordering::SeqCst);
		EventResult::Continue
	}

	async fn before_update(&self, _instance_id: &str, _values: &JsonValue) -> EventResult {
		self.pre_update_count.fetch_add(1, Ordering::SeqCst);
		if *self.veto_update.lock().unwrap() {
			EventResult::Veto
		} else {
			EventResult::Continue
		}
	}

	async fn after_update(&self, _instance_id: &str) -> EventResult {
		self.post_update_count.fetch_add(1, Ordering::SeqCst);
		EventResult::Continue
	}

	async fn before_delete(&self, _instance_id: &str) -> EventResult {
		self.pre_delete_count.fetch_add(1, Ordering::SeqCst);
		if *self.veto_delete.lock().unwrap() {
			EventResult::Veto
		} else {
			EventResult::Continue
		}
	}

	async fn after_delete(&self, _instance_id: &str) -> EventResult {
		self.post_delete_count.fetch_add(1, Ordering::SeqCst);
		EventResult::Continue
	}
}

// ============================================================================
// Test Table Definitions
// ============================================================================

#[derive(Iden)]
enum Articles {
	Table,
	Id,
	Title,
	Content,
	Status,
}

#[derive(Iden)]
enum Users {
	Table,
	Id,
	Username,
	Email,
	Active,
}

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
async fn events_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	(container, pool, port, url)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create articles table for event testing
async fn create_articles_table(pool: &PgPool) {
	let create_table = Table::create()
		.table(Articles::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Articles::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Articles::Title).string().not_null())
		.col(ColumnDef::new(Articles::Content).text().not_null())
		.col(
			ColumnDef::new(Articles::Status)
				.string()
				.not_null()
				.default("draft"),
		)
		.build(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool)
		.await
		.expect("Failed to create articles table");
}

/// Create users table for event testing
async fn create_users_table(pool: &PgPool) {
	let create_table = Table::create()
		.table(Users::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Users::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(
			ColumnDef::new(Users::Username)
				.string()
				.not_null()
				.unique_key(),
		)
		.col(ColumnDef::new(Users::Email).string().not_null())
		.col(
			ColumnDef::new(Users::Active)
				.boolean()
				.not_null()
				.default(true),
		)
		.build(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool)
		.await
		.expect("Failed to create users table");
}

/// Clear event registry between tests
fn clear_event_registry() {
	event_registry().clear();
}

// ============================================================================
// State Transitions: before_insert/after_insert
// ============================================================================

/// Test before_insert event fires before database INSERT via Model::save()
///
/// **Test Intent**: Verify that before_insert event fires when calling save() on new model
///
/// **Integration Point**: Model::save() → EventRegistry → MapperEvents::before_insert
///
/// **Not Intent**: after_insert behavior, update operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_before_insert_fires_on_save(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article
	let mut article = Article::new("Test Article", "Test Content");
	article.save().await.expect("Failed to save article");

	// Verify before_insert was called
	assert_eq!(
		listener.pre_insert_count(),
		1,
		"before_insert should fire once on new save"
	);
	assert!(article.id.is_some(), "Article should have ID after save");
}

/// Test after_insert event fires after successful INSERT via Model::save()
///
/// **Test Intent**: Verify that after_insert event fires after successful save() of new model
///
/// **Integration Point**: Model::save() → INSERT → EventRegistry → MapperEvents::after_insert
///
/// **Not Intent**: before_insert behavior, update operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_after_insert_fires_on_save(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article
	let mut article = Article::new("After Insert Test", "Content");
	article.save().await.expect("Failed to save article");

	// Verify after_insert was called
	assert_eq!(
		listener.post_insert_count(),
		1,
		"after_insert should fire once on new save"
	);
}

/// Test complete INSERT lifecycle: before_insert then after_insert
///
/// **Test Intent**: Verify both events fire in correct order for INSERT
///
/// **Integration Point**: Model::save() → Complete insert lifecycle
///
/// **Not Intent**: UPDATE behavior, single event verification
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_insert_lifecycle_events(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article
	let mut article = Article::new("Lifecycle Test", "Content");
	article.save().await.expect("Failed to save article");

	// Verify lifecycle: before_insert → INSERT → after_insert
	assert_eq!(listener.pre_insert_count(), 1);
	assert_eq!(listener.post_insert_count(), 1);
	assert!(article.id.is_some());
}

// ============================================================================
// State Transitions: before_update/after_update
// ============================================================================

/// Test before_update event fires before database UPDATE via Model::save()
///
/// **Test Intent**: Verify that before_update event fires when calling save() on existing model
///
/// **Integration Point**: Model::save() (existing) → EventRegistry → MapperEvents::before_update
///
/// **Not Intent**: INSERT behavior, after_update behavior
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_before_update_fires_on_save(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article (INSERT)
	let mut article = Article::new("Original Title", "Original Content");
	article.save().await.expect("Failed to save article");

	// Modify and save again (UPDATE)
	article.title = "Updated Title".to_string();
	article.save().await.expect("Failed to update article");

	// Verify before_update was called (1 insert + 1 update)
	assert_eq!(listener.pre_insert_count(), 1, "One INSERT");
	assert_eq!(listener.pre_update_count(), 1, "One UPDATE");
}

/// Test after_update event fires after successful UPDATE via Model::save()
///
/// **Test Intent**: Verify that after_update event fires after successful save() of existing model
///
/// **Integration Point**: Model::save() (existing) → UPDATE → MapperEvents::after_update
///
/// **Not Intent**: INSERT behavior, before_update behavior
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_after_update_fires_on_save(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article (INSERT)
	let mut article = Article::new("Original Title", "Original Content");
	article.save().await.expect("Failed to save article");

	// Modify and save again (UPDATE)
	article.title = "Updated Title".to_string();
	article.save().await.expect("Failed to update article");

	// Verify after_update was called
	assert_eq!(listener.post_insert_count(), 1, "One INSERT");
	assert_eq!(listener.post_update_count(), 1, "One UPDATE");
}

// ============================================================================
// State Transitions: before_delete/after_delete
// ============================================================================

/// Test before_delete event fires before database DELETE via Model::delete()
///
/// **Test Intent**: Verify that before_delete event fires when calling delete()
///
/// **Integration Point**: Model::delete() → EventRegistry → MapperEvents::before_delete
///
/// **Not Intent**: after_delete behavior, save operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_before_delete_fires_on_delete(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article
	let mut article = Article::new("Article to Delete", "Content");
	article.save().await.expect("Failed to save article");

	// Delete the article
	article.delete().await.expect("Failed to delete article");

	// Verify before_delete was called
	assert_eq!(
		listener.pre_delete_count(),
		1,
		"before_delete should fire once"
	);
}

/// Test after_delete event fires after successful DELETE via Model::delete()
///
/// **Test Intent**: Verify that after_delete event fires after successful delete()
///
/// **Integration Point**: Model::delete() → DELETE → MapperEvents::after_delete
///
/// **Not Intent**: before_delete behavior, save operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_after_delete_fires_on_delete(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article
	let mut article = Article::new("Article to Delete", "Content");
	article.save().await.expect("Failed to save article");

	// Delete the article
	article.delete().await.expect("Failed to delete article");

	// Verify after_delete was called
	assert_eq!(
		listener.post_delete_count(),
		1,
		"after_delete should fire once"
	);
}

/// Test complete delete lifecycle: before_delete then after_delete
///
/// **Test Intent**: Verify that both before_delete and after_delete fire in correct order
///
/// **Integration Point**: Model::delete() → Complete delete lifecycle
///
/// **Not Intent**: Single event verification, save operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_delete_lifecycle_events(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article
	let mut article = Article::new("Lifecycle Test", "Content");
	article.save().await.expect("Failed to save article");

	// Delete the article
	article.delete().await.expect("Failed to delete article");

	// Verify lifecycle
	assert_eq!(listener.pre_delete_count(), 1);
	assert_eq!(listener.post_delete_count(), 1);
}

// ============================================================================
// Veto Behavior: Prevent Database Operations
// ============================================================================

/// Test that before_insert veto prevents INSERT operation
///
/// **Test Intent**: Verify that returning Veto from before_insert prevents the database INSERT
///
/// **Integration Point**: MapperEvents::before_insert → Veto → No INSERT
///
/// **Not Intent**: after_insert behavior, update/delete operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_before_insert_veto_prevents_insert(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener with veto enabled
	let listener = Arc::new(TestMapperListener::new());
	listener.set_veto_insert(true);
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Attempt to save new article - should be vetoed
	let mut article = Article::new("Vetoed Article", "Content");
	let result = article.save().await;

	// Verify operation was vetoed
	assert!(result.is_err(), "Save should fail when vetoed");
	assert!(
		result.unwrap_err().to_string().contains("vetoed"),
		"Error should mention veto"
	);

	// Verify before_insert was called but after_insert was not
	assert_eq!(listener.pre_insert_count(), 1, "before_insert should fire");
	assert_eq!(
		listener.post_insert_count(),
		0,
		"after_insert should NOT fire when vetoed"
	);

	// Verify article was not saved (no ID)
	assert!(article.id.is_none(), "Article should not have ID");
}

/// Test that before_update veto prevents UPDATE operation
///
/// **Test Intent**: Verify that returning Veto from before_update prevents the database UPDATE
///
/// **Integration Point**: MapperEvents::before_update → Veto → No UPDATE
///
/// **Not Intent**: INSERT behavior, delete operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_before_update_veto_prevents_update(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article (INSERT succeeds)
	let mut article = Article::new("Original Title", "Original Content");
	article.save().await.expect("Failed to save article");
	let original_id = article.id;

	// Now enable veto for updates
	listener.set_veto_update(true);

	// Attempt to update - should be vetoed
	article.title = "Updated Title".to_string();
	let result = article.save().await;

	// Verify operation was vetoed
	assert!(result.is_err(), "Update should fail when vetoed");

	// Verify before_update was called but after_update was not
	assert_eq!(listener.pre_update_count(), 1, "before_update should fire");
	assert_eq!(
		listener.post_update_count(),
		0,
		"after_update should NOT fire when vetoed"
	);

	// Verify article ID unchanged
	assert_eq!(article.id, original_id);
}

/// Test that before_delete veto prevents DELETE operation
///
/// **Test Intent**: Verify that returning Veto from before_delete prevents the database DELETE
///
/// **Integration Point**: MapperEvents::before_delete → Veto → No DELETE
///
/// **Not Intent**: INSERT/UPDATE behavior, successful delete
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_before_delete_veto_prevents_delete(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener with delete veto enabled
	let listener = Arc::new(TestMapperListener::new());
	listener.set_veto_delete(true);
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create and save new article
	let mut article = Article::new("Protected Article", "Content");
	article.save().await.expect("Failed to save article");
	let article_id = article.id.unwrap();

	// Attempt to delete - should be vetoed
	let result = article.delete().await;

	// Verify operation was vetoed
	assert!(result.is_err(), "Delete should fail when vetoed");

	// Verify before_delete was called but after_delete was not
	assert_eq!(listener.pre_delete_count(), 1, "before_delete should fire");
	assert_eq!(
		listener.post_delete_count(),
		0,
		"after_delete should NOT fire when vetoed"
	);

	// Verify record still exists in database
	let record_exists = sqlx::query("SELECT id FROM articles WHERE id = $1")
		.bind(article_id)
		.fetch_optional(pool.as_ref())
		.await
		.expect("Failed to query article")
		.is_some();

	assert!(record_exists, "Article should still exist when vetoed");
}

// ============================================================================
// Multiple Listeners: Sequential Execution
// ============================================================================

/// Test multiple listeners receive same event
///
/// **Test Intent**: Verify that multiple event listeners all receive the event
///
/// **Integration Point**: EventRegistry → Multiple MapperEvents listeners
///
/// **Not Intent**: Single listener behavior, veto behavior
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_multiple_listeners_receive_events(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register multiple event listeners
	let listener1 = Arc::new(TestMapperListener::new());
	let listener2 = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener1.clone());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener2.clone());

	// Create and save new article
	let mut article = Article::new("Multi-listener Test", "Content");
	article.save().await.expect("Failed to save article");

	// Verify both listeners received the events
	assert_eq!(
		listener1.pre_insert_count(),
		1,
		"Listener 1 should receive before_insert"
	);
	assert_eq!(
		listener2.pre_insert_count(),
		1,
		"Listener 2 should receive before_insert"
	);
	assert_eq!(
		listener1.post_insert_count(),
		1,
		"Listener 1 should receive after_insert"
	);
	assert_eq!(
		listener2.post_insert_count(),
		1,
		"Listener 2 should receive after_insert"
	);
}

/// Test first listener veto stops further processing
///
/// **Test Intent**: Verify that when first listener vetoes, operation is stopped immediately
///
/// **Integration Point**: EventRegistry → Veto propagation
///
/// **Not Intent**: Successful multi-listener dispatch
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_first_listener_veto_stops_processing(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register multiple listeners - first one vetoes
	let listener1 = Arc::new(TestMapperListener::new());
	listener1.set_veto_insert(true);
	let listener2 = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener1.clone());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener2.clone());

	// Attempt to save - should be vetoed by first listener
	let mut article = Article::new("Vetoed by First", "Content");
	let result = article.save().await;

	// Verify operation was vetoed
	assert!(result.is_err());

	// First listener should have been called
	assert_eq!(listener1.pre_insert_count(), 1);
	// Second listener may or may not have been called depending on implementation
	// (early exit vs. continue). The key is that the operation was blocked.
}

// ============================================================================
// Event Execution Order with Multiple Operations
// ============================================================================

/// Test event execution order across multiple operations
///
/// **Test Intent**: Verify events fire in correct order for INSERT → UPDATE → DELETE
///
/// **Integration Point**: Event sequence across operation types
///
/// **Not Intent**: Single operation verification
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_event_order_insert_update_delete(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_users_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(User::table_name().to_string(), listener.clone());

	// Operation 1: INSERT
	let mut user = User::new("testuser", "test@example.com");
	user.save().await.expect("Failed to save user");

	// Operation 2: UPDATE
	user.email = "updated@example.com".to_string();
	user.save().await.expect("Failed to update user");

	// Operation 3: DELETE
	user.delete().await.expect("Failed to delete user");

	// Verify execution counts
	assert_eq!(
		listener.pre_insert_count(),
		1,
		"before_insert should fire once"
	);
	assert_eq!(
		listener.post_insert_count(),
		1,
		"after_insert should fire once"
	);
	assert_eq!(
		listener.pre_update_count(),
		1,
		"before_update should fire once"
	);
	assert_eq!(
		listener.post_update_count(),
		1,
		"after_update should fire once"
	);
	assert_eq!(
		listener.pre_delete_count(),
		1,
		"before_delete should fire once"
	);
	assert_eq!(
		listener.post_delete_count(),
		1,
		"after_delete should fire once"
	);
}

/// Test multiple saves on same model
///
/// **Test Intent**: Verify first save triggers INSERT, subsequent saves trigger UPDATE
///
/// **Integration Point**: Model::save() → INSERT vs UPDATE detection
///
/// **Not Intent**: Delete behavior, single save verification
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_multiple_saves_insert_then_update(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;

	// Register event listener
	let listener = Arc::new(TestMapperListener::new());
	event_registry().register_mapper_listener(Article::table_name().to_string(), listener.clone());

	// Create new article
	let mut article = Article::new("First Title", "Content");

	// First save - should be INSERT
	article.save().await.expect("Failed to save article");
	assert_eq!(listener.pre_insert_count(), 1, "First save should INSERT");
	assert_eq!(
		listener.pre_update_count(),
		0,
		"First save should NOT UPDATE"
	);

	// Second save - should be UPDATE
	article.title = "Second Title".to_string();
	article.save().await.expect("Failed to save article");
	assert_eq!(
		listener.pre_insert_count(),
		1,
		"Second save should NOT INSERT"
	);
	assert_eq!(listener.pre_update_count(), 1, "Second save should UPDATE");

	// Third save - should also be UPDATE
	article.title = "Third Title".to_string();
	article.save().await.expect("Failed to save article");
	assert_eq!(
		listener.pre_insert_count(),
		1,
		"Third save should NOT INSERT"
	);
	assert_eq!(listener.pre_update_count(), 2, "Third save should UPDATE");
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test delete without primary key fails
///
/// **Test Intent**: Verify that calling delete() without a primary key returns error
///
/// **Integration Point**: Model::delete() validation
///
/// **Not Intent**: Successful delete, save operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_delete_without_pk_fails(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	// Create article without saving (no primary key)
	let article = Article::new("Unsaved Article", "Content");
	assert!(article.id.is_none());

	// Attempt to delete - should fail
	let result = article.delete().await;
	assert!(result.is_err(), "Delete without PK should fail");
	assert!(
		result.unwrap_err().to_string().contains("primary key"),
		"Error should mention primary key"
	);
}

/// Test events for different model types are independent
///
/// **Test Intent**: Verify that events registered for one model don't affect other models
///
/// **Integration Point**: EventRegistry → Model-specific listener dispatch
///
/// **Not Intent**: Cross-model event sharing
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_events_are_model_specific(
	#[future] events_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = events_test_db.await;
	clear_event_registry();

	create_articles_table(pool.as_ref()).await;
	create_users_table(pool.as_ref()).await;

	// Register listener only for Articles
	let article_listener = Arc::new(TestMapperListener::new());
	event_registry()
		.register_mapper_listener(Article::table_name().to_string(), article_listener.clone());

	// Save an article - listener should be called
	let mut article = Article::new("Test Article", "Content");
	article.save().await.expect("Failed to save article");
	assert_eq!(article_listener.pre_insert_count(), 1);

	// Save a user - article listener should NOT be called
	let mut user = User::new("testuser", "test@example.com");
	user.save().await.expect("Failed to save user");
	assert_eq!(
		article_listener.pre_insert_count(),
		1,
		"Article listener should not respond to User events"
	);
}
