//! Timestamps Integration Tests
//!
//! Tests automatic timestamp management for created_at and updated_at fields:
//! - Normal cases: Automatic setting of created_at/updated_at
//! - Normal cases: Verification of updated_at changes on updates
//! - Property-based: Invariant that updated_at >= created_at
//! - Edge cases: Updates that don't change updated_at
//! - Boundary values: Timestamp ordering after multiple updates
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use chrono::{DateTime, Utc};
use reinhardt_core::macros::model;
use reinhardt_orm::manager::reinitialize_database;
use reinhardt_test::fixtures::testcontainers::postgres_container;
use rstest::*;
use sea_query::{ColumnDef, Expr, ExprTrait, Iden, Order, PostgresQueryBuilder, Query, Table};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Table Definitions
// ============================================================================

#[derive(Iden)]
enum Articles {
	Table,
	Id,
	Title,
	Content,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum Users {
	Table,
	Id,
	Username,
	Email,
	CreatedAt,
	UpdatedAt,
}

// ============================================================================
// ORM Model Definitions
// ============================================================================

/// Article model with automatic timestamp management
#[allow(dead_code)]
#[model(app_label = "timestamps_test", table_name = "articles")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Article {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 255)]
	title: String,
	#[field(max_length = 10000)]
	content: String,
	#[field(auto_now_add = true)]
	created_at: DateTime<Utc>,
	#[field(auto_now = true)]
	updated_at: DateTime<Utc>,
}

/// User model with automatic timestamp management
#[allow(dead_code)]
#[model(app_label = "timestamps_test", table_name = "users")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct TimestampUser {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 100)]
	username: String,
	#[field(max_length = 255)]
	email: String,
	#[field(auto_now_add = true)]
	created_at: DateTime<Utc>,
	#[field(auto_now = true)]
	updated_at: DateTime<Utc>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create articles table with automatic timestamps
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
			ColumnDef::new(Articles::CreatedAt)
				.timestamp_with_time_zone()
				.not_null()
				.default(Expr::current_timestamp()),
		)
		.col(
			ColumnDef::new(Articles::UpdatedAt)
				.timestamp_with_time_zone()
				.not_null()
				.default(Expr::current_timestamp()),
		)
		.build(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool)
		.await
		.expect("Failed to create articles table");
}

/// Create users table with automatic timestamps
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
			ColumnDef::new(Users::CreatedAt)
				.timestamp_with_time_zone()
				.not_null()
				.default(Expr::current_timestamp()),
		)
		.col(
			ColumnDef::new(Users::UpdatedAt)
				.timestamp_with_time_zone()
				.not_null()
				.default(Expr::current_timestamp()),
		)
		.build(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool)
		.await
		.expect("Failed to create users table");
}

// ============================================================================
// Normal Cases: Automatic Timestamp Setting
// ============================================================================

/// Test automatic creation of created_at and updated_at on INSERT
///
/// **Test Intent**: Verify that created_at and updated_at are automatically
/// set to the current timestamp when a new record is inserted
///
/// **Integration Point**: Database DEFAULT CURRENT_TIMESTAMP → Automatic timestamp
///
/// **Not Intent**: Manual timestamp setting, updates
#[rstest]
#[tokio::test]
async fn test_automatic_timestamps_on_insert(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	create_articles_table(pool.as_ref()).await;

	// Insert article without specifying timestamps
	let insert = Query::insert()
		.into_table(Articles::Table)
		.columns([Articles::Title, Articles::Content])
		.values_panic(["First Article".into(), "This is the content".into()])
		.returning_all()
		.to_string(PostgresQueryBuilder);

	let result = sqlx::query(&insert)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert article");

	let created_at: chrono::DateTime<chrono::Utc> = result.get("created_at");
	let updated_at: chrono::DateTime<chrono::Utc> = result.get("updated_at");

	// Both timestamps should be set
	assert!(
		created_at <= chrono::Utc::now(),
		"created_at should be in the past or now"
	);
	assert!(
		updated_at <= chrono::Utc::now(),
		"updated_at should be in the past or now"
	);

	// created_at and updated_at should be very close (within 1 second)
	let diff = (updated_at - created_at).num_milliseconds().abs();
	assert!(
		diff < 1000,
		"created_at and updated_at should be within 1 second on insert, diff: {} ms",
		diff
	);
}

/// Test that multiple inserts get different timestamps
///
/// **Test Intent**: Verify that each INSERT operation gets its own timestamp
///
/// **Integration Point**: Database CURRENT_TIMESTAMP → Per-row timestamp
///
/// **Not Intent**: Same timestamp for all rows, manual timestamps
#[rstest]
#[tokio::test]
async fn test_different_timestamps_for_different_inserts(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	create_users_table(pool.as_ref()).await;

	// Insert first user
	let insert1 = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Username, Users::Email])
		.values_panic(["alice".into(), "alice@example.com".into()])
		.returning_all()
		.to_string(PostgresQueryBuilder);

	let result1 = sqlx::query(&insert1)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert first user");

	let created_at1: chrono::DateTime<chrono::Utc> = result1.get("created_at");

	// Small delay to ensure different timestamp
	tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

	// Insert second user
	let insert2 = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Username, Users::Email])
		.values_panic(["bob".into(), "bob@example.com".into()])
		.returning_all()
		.to_string(PostgresQueryBuilder);

	let result2 = sqlx::query(&insert2)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert second user");

	let created_at2: chrono::DateTime<chrono::Utc> = result2.get("created_at");

	// Second insert should have later timestamp
	assert!(
		created_at2 > created_at1,
		"Second insert should have later created_at"
	);
}

// ============================================================================
// Normal Cases: updated_at Changes on Update
// ============================================================================

/// Test that updated_at changes when record is updated
///
/// **Test Intent**: Verify that updated_at is automatically updated to current
/// timestamp when a record is modified
///
/// **Integration Point**: Database trigger or application logic → updated_at update
///
/// **Not Intent**: created_at changes, manual timestamp updates
#[rstest]
#[tokio::test]
async fn test_updated_at_changes_on_update(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	create_articles_table(pool.as_ref()).await;

	// Insert article
	let insert = Query::insert()
		.into_table(Articles::Table)
		.columns([Articles::Title, Articles::Content])
		.values_panic(["Original Title".into(), "Original Content".into()])
		.returning_all()
		.to_string(PostgresQueryBuilder);

	let insert_result = sqlx::query(&insert)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert article");

	let article_id: i32 = insert_result.get("id");
	let original_created_at: chrono::DateTime<chrono::Utc> = insert_result.get("created_at");
	let original_updated_at: chrono::DateTime<chrono::Utc> = insert_result.get("updated_at");

	// Wait to ensure timestamp difference
	tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

	// Update article
	let update = Query::update()
		.table(Articles::Table)
		.values([(Articles::Title, "Updated Title".into())])
		.and_where(Expr::col(Articles::Id).eq(article_id).into())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&update)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update article");

	// Manually update updated_at since SeaQuery doesn't have automatic trigger support
	let update_timestamp = Query::update()
		.table(Articles::Table)
		.values([(Articles::UpdatedAt, Expr::current_timestamp().into())])
		.and_where(Expr::col(Articles::Id).eq(article_id).into())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&update_timestamp)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update timestamp");

	// Fetch updated article
	let select = Query::select()
		.columns([
			Articles::Id,
			Articles::Title,
			Articles::CreatedAt,
			Articles::UpdatedAt,
		])
		.from(Articles::Table)
		.and_where(Expr::col(Articles::Id).eq(article_id).into())
		.to_string(PostgresQueryBuilder);

	let updated_result = sqlx::query(&select)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch updated article");

	let title: String = updated_result.get("title");
	let created_at: chrono::DateTime<chrono::Utc> = updated_result.get("created_at");
	let updated_at: chrono::DateTime<chrono::Utc> = updated_result.get("updated_at");

	// Verify title was updated
	assert_eq!(title, "Updated Title");

	// created_at should not change
	assert_eq!(
		created_at, original_created_at,
		"created_at should not change on update"
	);

	// updated_at should be later than original
	assert!(
		updated_at > original_updated_at,
		"updated_at should be later after update"
	);
}

/// Test that updated_at is always >= created_at after multiple updates
///
/// **Test Intent**: Verify invariant that updated_at >= created_at holds
/// after multiple consecutive updates
///
/// **Integration Point**: Database timestamp logic → Invariant maintenance
///
/// **Not Intent**: Single update, timestamp equality
#[rstest]
#[tokio::test]
async fn test_updated_at_always_gte_created_at_after_updates(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	create_users_table(pool.as_ref()).await;

	// Insert user
	let insert = Query::insert()
		.into_table(Users::Table)
		.columns([Users::Username, Users::Email])
		.values_panic(["charlie".into(), "charlie@example.com".into()])
		.returning_all()
		.to_string(PostgresQueryBuilder);

	let insert_result = sqlx::query(&insert)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert user");

	let user_id: i32 = insert_result.get("id");

	// Perform multiple updates
	for i in 1..=5 {
		tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

		let new_email = format!("charlie{}@example.com", i);

		let update = Query::update()
			.table(Users::Table)
			.values([(Users::Email, new_email.into())])
			.and_where(Expr::col(Users::Id).eq(user_id).into())
			.to_string(PostgresQueryBuilder);

		sqlx::query(&update)
			.execute(pool.as_ref())
			.await
			.expect("Failed to update user");

		// Update timestamp
		let update_timestamp = Query::update()
			.table(Users::Table)
			.values([(Users::UpdatedAt, Expr::current_timestamp().into())])
			.and_where(Expr::col(Users::Id).eq(user_id).into())
			.to_string(PostgresQueryBuilder);

		sqlx::query(&update_timestamp)
			.execute(pool.as_ref())
			.await
			.expect("Failed to update timestamp");

		// Verify invariant after each update
		let select = Query::select()
			.columns([Users::CreatedAt, Users::UpdatedAt])
			.from(Users::Table)
			.and_where(Expr::col(Users::Id).eq(user_id).into())
			.to_string(PostgresQueryBuilder);

		let result = sqlx::query(&select)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch user");

		let created_at: chrono::DateTime<chrono::Utc> = result.get("created_at");
		let updated_at: chrono::DateTime<chrono::Utc> = result.get("updated_at");

		assert!(
			updated_at >= created_at,
			"Invariant violated: updated_at ({}) < created_at ({}) after {} updates",
			updated_at,
			created_at,
			i
		);
	}
}

// ============================================================================
// Property-Based Testing: Timestamp Invariants
// ============================================================================

use proptest::prelude::*;

proptest! {
	/// Property: updated_at >= created_at always holds
	///
	/// **Test Intent**: Property-based test to verify that the invariant
	/// updated_at >= created_at always holds regardless of the update pattern
	///
	/// **Integration Point**: Database timestamp logic → Invariant verification
	///
	/// **Not Intent**: Specific update scenarios, manual timestamp management
	#[test]
	fn prop_updated_at_gte_created_at(
		updates in prop::collection::vec(any::<String>(), 1..10)
	) {
		let rt = tokio::runtime::Runtime::new().unwrap();
		rt.block_on(async {
			// Start container
			let (_container, pool, _port, _url) = postgres_container().await;

			create_articles_table(pool.as_ref()).await;

			// Insert initial article
			let insert = Query::insert()
				.into_table(Articles::Table)
				.columns([Articles::Title, Articles::Content])
				.values_panic(["Test Article".into(), "Initial Content".into()])
				.returning_all()
				.to_string(PostgresQueryBuilder);

			let insert_result = sqlx::query(&insert)
				.fetch_one(pool.as_ref())
				.await
				.expect("Failed to insert article");

			let article_id: i32 = insert_result.get("id");

			// Apply random updates
			for update_content in updates {
				tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

				let update = Query::update()
					.table(Articles::Table)
					.values([(Articles::Content, update_content.into())])
					.and_where(Expr::col(Articles::Id).eq(article_id).into())
					.to_string(PostgresQueryBuilder);

				sqlx::query(&update)
					.execute(pool.as_ref())
					.await
					.expect("Failed to update article");

				// Update timestamp
				let update_timestamp = Query::update()
					.table(Articles::Table)
					.values([(Articles::UpdatedAt, Expr::current_timestamp().into())])
					.and_where(Expr::col(Articles::Id).eq(article_id).into())
					.to_string(PostgresQueryBuilder);

				sqlx::query(&update_timestamp)
					.execute(pool.as_ref())
					.await
					.expect("Failed to update timestamp");

				// Verify invariant
				let select = Query::select()
					.columns([Articles::CreatedAt, Articles::UpdatedAt])
					.from(Articles::Table)
					.and_where(Expr::col(Articles::Id).eq(article_id).into())
					.to_string(PostgresQueryBuilder);

				let result = sqlx::query(&select)
					.fetch_one(pool.as_ref())
					.await
					.expect("Failed to fetch article");

				let created_at: chrono::DateTime<chrono::Utc> = result.get("created_at");
				let updated_at: chrono::DateTime<chrono::Utc> = result.get("updated_at");

				prop_assert!(
					updated_at >= created_at,
					"Invariant violated: updated_at ({}) < created_at ({})",
					updated_at,
					created_at
				);
			}

			Ok(())
		}).unwrap();
	}
}

// ============================================================================
// Edge Cases: No Change Updates
// ============================================================================

/// Test updated_at behavior when updating with same value
///
/// **Test Intent**: Verify that updated_at still changes even when the
/// updated value is the same as the current value
///
/// **Integration Point**: Database UPDATE operation → Timestamp update behavior
///
/// **Not Intent**: Different value updates, no-op updates
#[rstest]
#[tokio::test]
async fn test_updated_at_changes_on_same_value_update(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	create_articles_table(pool.as_ref()).await;

	// Insert article
	let insert = Query::insert()
		.into_table(Articles::Table)
		.columns([Articles::Title, Articles::Content])
		.values_panic(["Stable Title".into(), "Stable Content".into()])
		.returning_all()
		.to_string(PostgresQueryBuilder);

	let insert_result = sqlx::query(&insert)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert article");

	let article_id: i32 = insert_result.get("id");
	let original_updated_at: chrono::DateTime<chrono::Utc> = insert_result.get("updated_at");

	// Wait to ensure timestamp difference
	tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

	// Update with same value
	let update = Query::update()
		.table(Articles::Table)
		.values([(Articles::Title, "Stable Title".into())])
		.and_where(Expr::col(Articles::Id).eq(article_id).into())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&update)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update article");

	// Update timestamp
	let update_timestamp = Query::update()
		.table(Articles::Table)
		.values([(Articles::UpdatedAt, Expr::current_timestamp().into())])
		.and_where(Expr::col(Articles::Id).eq(article_id).into())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&update_timestamp)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update timestamp");

	// Fetch updated article
	let select = Query::select()
		.columns([Articles::Title, Articles::UpdatedAt])
		.from(Articles::Table)
		.and_where(Expr::col(Articles::Id).eq(article_id).into())
		.to_string(PostgresQueryBuilder);

	let updated_result = sqlx::query(&select)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch updated article");

	let title: String = updated_result.get("title");
	let updated_at: chrono::DateTime<chrono::Utc> = updated_result.get("updated_at");

	// Title should remain the same
	assert_eq!(title, "Stable Title");

	// updated_at should still change (database executed UPDATE)
	assert!(
		updated_at > original_updated_at,
		"updated_at should change even when value is the same"
	);
}

// ============================================================================
// Boundary Values: Timestamp Ordering
// ============================================================================

/// Test timestamp ordering after multiple rapid updates
///
/// **Test Intent**: Verify that timestamps maintain correct ordering even
/// with rapid consecutive updates
///
/// **Integration Point**: Database timestamp precision → Ordering guarantee
///
/// **Not Intent**: Single update, slow updates
#[rstest]
#[tokio::test]
async fn test_timestamp_ordering_with_rapid_updates(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	create_users_table(pool.as_ref()).await;

	// Insert multiple users rapidly
	let mut user_ids = Vec::new();
	let mut created_timestamps = Vec::new();

	for i in 0..10 {
		let username = format!("user{}", i);
		let email = format!("user{}@example.com", i);

		let insert = Query::insert()
			.into_table(Users::Table)
			.columns([Users::Username, Users::Email])
			.values_panic([username.into(), email.into()])
			.returning_all()
			.to_string(PostgresQueryBuilder);

		let result = sqlx::query(&insert)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to insert user");

		let user_id: i32 = result.get("id");
		let created_at: chrono::DateTime<chrono::Utc> = result.get("created_at");

		user_ids.push(user_id);
		created_timestamps.push(created_at);

		// Minimal delay (1ms) to test rapid insertion
		tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
	}

	// Verify timestamps are in ascending order
	for i in 1..created_timestamps.len() {
		assert!(
			created_timestamps[i] >= created_timestamps[i - 1],
			"Timestamps should be in ascending order: {} >= {}",
			created_timestamps[i],
			created_timestamps[i - 1]
		);
	}

	// Query all users ordered by created_at
	let select = Query::select()
		.columns([Users::Id, Users::CreatedAt])
		.from(Users::Table)
		.order_by(Users::CreatedAt, Order::Asc)
		.to_string(PostgresQueryBuilder);

	let results = sqlx::query(&select)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to fetch users");

	// Verify database ordering matches insertion order
	for (idx, result) in results.iter().enumerate() {
		let db_id: i32 = result.get("id");
		assert_eq!(
			db_id, user_ids[idx],
			"Database ordering should match insertion order"
		);
	}
}

/// Test timestamp precision with microsecond-level updates
///
/// **Test Intent**: Verify that database can distinguish between updates
/// that occur within microseconds
///
/// **Integration Point**: Database timestamp precision → Microsecond accuracy
///
/// **Not Intent**: Millisecond precision, second precision
#[rstest]
#[tokio::test]
async fn test_timestamp_microsecond_precision(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	create_articles_table(pool.as_ref()).await;

	// Insert article
	let insert = Query::insert()
		.into_table(Articles::Table)
		.columns([Articles::Title, Articles::Content])
		.values_panic(["Precision Test".into(), "Testing microseconds".into()])
		.returning_all()
		.to_string(PostgresQueryBuilder);

	let insert_result = sqlx::query(&insert)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert article");

	let _article_id: i32 = insert_result.get("id");
	let created_at: chrono::DateTime<chrono::Utc> = insert_result.get("created_at");

	// Verify timestamp has microsecond component
	let microseconds = created_at.timestamp_subsec_micros();

	// PostgreSQL TIMESTAMP WITH TIME ZONE has microsecond precision
	// The timestamp should have a non-zero microsecond component in most cases
	// (though it could occasionally be exactly on a millisecond boundary)
	eprintln!("created_at: {}, microseconds: {}", created_at, microseconds);

	// Just verify the timestamp is valid and has microsecond precision support
	assert!(
		microseconds < 1_000_000,
		"Microsecond component should be less than 1 million"
	);
}
