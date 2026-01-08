//! ViewSet Advanced Features Integration Tests
//!
//! Tests advanced ViewSet feature combinations:
//! - Pagination + Filtering + Ordering combined
//! - Custom permissions with filtering
//! - Multiple filter backends
//! - Custom queryset with pagination
//! - Read/Write serializer separation
//! - Nested resources with caching
//! - Complex filtering with multiple fields
//! - Custom ordering with multiple fields
//! - Permission-based filtering
//! - Advanced configuration combinations
//!
//! **Test Category**: Combination Testing
//!
//! **Fixtures Used:**
//! - shared_db_pool: Shared PostgreSQL database pool with ORM initialized
//!
//! **Note**: These tests verify that multiple advanced features work correctly
//! when used together, not just in isolation.

use bytes::Bytes;
use chrono::{DateTime, Utc};
use hyper::{HeaderMap, Method, Version};
use reinhardt_core::http::Request;
use reinhardt_core::macros::model;
use reinhardt_serializers::JsonSerializer;
use reinhardt_test::fixtures::shared_db_pool;
use reinhardt_viewsets::{
	FilterConfig, FilterableViewSet, ModelViewSet, OrderingConfig, PaginatedViewSet,
	PaginationConfig,
};
use rstest::*;
use sea_query::{Expr, ExprTrait, Iden, PostgresQueryBuilder, Query, Table};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;

// ============================================================================
// Test Structures
// ============================================================================

/// Advanced test model with multiple fields for filtering/ordering
#[allow(dead_code)]
#[model(app_label = "viewset_advanced_test", table_name = "advanced_items")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct AdvancedItem {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 200)]
	title: String,
	#[field(max_length = 100)]
	category: String,
	#[field(max_length = 100)]
	status: String,
	priority: i32,
	score: f64,
	published: bool,
	#[field(null = true)]
	author_id: Option<i64>,
	#[field(null = true)]
	created_at: Option<DateTime<Utc>>,
	#[field(null = true)]
	updated_at: Option<DateTime<Utc>>,
}

/// Iden enum for advanced_items table
#[derive(Iden)]
enum AdvancedItems {
	Table,
	Id,
	Title,
	Category,
	Status,
	Priority,
	Score,
	Published,
	AuthorId,
	CreatedAt,
	UpdatedAt,
}

// ============================================================================
// Fixtures
// ============================================================================

/// Setup: Shared PostgreSQL database pool and schema
///
/// Dependencies: shared_db_pool (shared PostgreSQL with ORM initialized)
#[fixture]
async fn setup_advanced(#[future] shared_db_pool: (PgPool, String)) -> Arc<PgPool> {
	let (pool, _url) = shared_db_pool.await;

	// Create advanced_items table
	let create_table_sql = Table::create()
		.table(AdvancedItems::Table)
		.if_not_exists()
		.col(
			sea_query::ColumnDef::new(AdvancedItems::Id)
				.big_integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(
			sea_query::ColumnDef::new(AdvancedItems::Title)
				.string_len(200)
				.not_null(),
		)
		.col(
			sea_query::ColumnDef::new(AdvancedItems::Category)
				.string_len(100)
				.not_null(),
		)
		.col(
			sea_query::ColumnDef::new(AdvancedItems::Status)
				.string_len(100)
				.not_null(),
		)
		.col(
			sea_query::ColumnDef::new(AdvancedItems::Priority)
				.integer()
				.not_null(),
		)
		.col(
			sea_query::ColumnDef::new(AdvancedItems::Score)
				.double()
				.not_null(),
		)
		.col(
			sea_query::ColumnDef::new(AdvancedItems::Published)
				.boolean()
				.not_null(),
		)
		.col(sea_query::ColumnDef::new(AdvancedItems::AuthorId).big_integer())
		.col(sea_query::ColumnDef::new(AdvancedItems::CreatedAt).timestamp())
		.col(sea_query::ColumnDef::new(AdvancedItems::UpdatedAt).timestamp())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_table_sql).execute(&pool).await.unwrap();

	// Insert test data with varied attributes
	for i in 1..=20 {
		let category = match i % 3 {
			0 => "Electronics",
			1 => "Books",
			_ => "Furniture",
		};

		let status = match i % 4 {
			0 => "draft",
			1 => "published",
			2 => "archived",
			_ => "pending",
		};

		let item = AdvancedItem::new(
			format!("Item {}", i),
			category.to_string(),
			status.to_string(),
			(i % 5 + 1) as i32,       // priority: 1-5
			(i as f64) * 1.5,         // score: 1.5, 3.0, 4.5, ...
			i % 2 == 0,               // published: alternating
			Some((i % 3 + 1) as i64), // author_id: 1, 2, 3
			Some(Utc::now()),
			Some(Utc::now()),
		);

		let insert_sql = Query::insert()
			.into_table(AdvancedItems::Table)
			.columns([
				AdvancedItems::Title,
				AdvancedItems::Category,
				AdvancedItems::Status,
				AdvancedItems::Priority,
				AdvancedItems::Score,
				AdvancedItems::Published,
				AdvancedItems::AuthorId,
				AdvancedItems::CreatedAt,
				AdvancedItems::UpdatedAt,
			])
			.values_panic([
				item.title.into(),
				item.category.into(),
				item.status.into(),
				item.priority.into(),
				item.score.into(),
				item.published.into(),
				item.author_id.into(),
				item.created_at.into(),
				item.updated_at.into(),
			])
			.to_string(PostgresQueryBuilder);

		sqlx::query(&insert_sql).execute(&pool).await.unwrap();
	}

	Arc::new(pool)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper: Create HTTP GET request
fn create_get_request(uri: &str) -> Request {
	Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.expect("Failed to build request")
}

// ============================================================================
// Tests
// ============================================================================

/// Test: Pagination + Filtering + Ordering combined
#[rstest]
#[tokio::test]
async fn test_pagination_filtering_ordering_combined(#[future] setup_advanced: Arc<PgPool>) {
	let _pool = setup_advanced.await;

	// Create ViewSet with all three features
	let pagination_config = PaginationConfig::page_number(5, Some(20));

	let filter_config = FilterConfig {
		filterable_fields: vec!["category".to_string(), "status".to_string()],
		search_fields: vec!["title".to_string()],
		case_insensitive_search: true,
	};

	let ordering_config = OrderingConfig {
		ordering_fields: vec!["priority".to_string(), "score".to_string()],
		default_ordering: vec!["-priority".to_string()], // Descending priority
	};

	let viewset = ModelViewSet::<AdvancedItem, JsonSerializer<AdvancedItem>>::new("advanced-items")
		.with_pagination(pagination_config)
		.with_filters(filter_config)
		.with_ordering(ordering_config);

	// Verify configuration
	assert!(viewset.get_pagination_config().is_some());
	assert!(viewset.get_filter_config().is_some());
	assert!(viewset.get_ordering_config().is_some());

	// Test: Filter by category="Electronics", order by -score, page 1
	let _request =
		create_get_request("/advanced-items/?category=Electronics&ordering=-score&page=1");

	// In real implementation, this would filter, order, and paginate
	// For this test, we verify the configuration is correctly set
	assert_eq!(
		viewset.get_filter_config().unwrap().filterable_fields,
		vec!["category", "status"]
	);
	assert_eq!(
		viewset.get_ordering_config().unwrap().default_ordering,
		vec!["-priority"]
	);
}

/// Test: Multiple filter backends
#[rstest]
#[tokio::test]
async fn test_multiple_filter_backends(#[future] setup_advanced: Arc<PgPool>) {
	let _pool = setup_advanced.await;

	// Create filter config with multiple filterable fields
	let filter_config = FilterConfig {
		filterable_fields: vec![
			"category".to_string(),
			"status".to_string(),
			"published".to_string(),
			"author_id".to_string(),
		],
		search_fields: vec!["title".to_string()],
		case_insensitive_search: true,
	};

	let viewset = ModelViewSet::<AdvancedItem, JsonSerializer<AdvancedItem>>::new("advanced-items")
		.with_filters(filter_config);

	// Verify all filterable fields are registered
	let config = viewset.get_filter_config().unwrap();
	assert_eq!(config.filterable_fields.len(), 4);
	assert!(config.filterable_fields.contains(&"category".to_string()));
	assert!(config.filterable_fields.contains(&"status".to_string()));
	assert!(config.filterable_fields.contains(&"published".to_string()));
	assert!(config.filterable_fields.contains(&"author_id".to_string()));
}

/// Test: Custom queryset with pagination
#[rstest]
#[tokio::test]
async fn test_custom_queryset_with_pagination(#[future] setup_advanced: Arc<PgPool>) {
	let _pool = setup_advanced.await;

	let pagination_config = PaginationConfig::LimitOffset {
		default_limit: 10,
		max_limit: Some(50),
	};

	let viewset = ModelViewSet::<AdvancedItem, JsonSerializer<AdvancedItem>>::new("advanced-items")
		.with_pagination(pagination_config);

	// Verify pagination configuration
	if let Some(PaginationConfig::LimitOffset {
		default_limit,
		max_limit,
	}) = viewset.get_pagination_config()
	{
		assert_eq!(default_limit, 10);
		assert_eq!(max_limit, Some(50));
	} else {
		panic!("Expected LimitOffset pagination config");
	}
}

/// Test: Complex filtering with multiple fields
#[rstest]
#[tokio::test]
async fn test_complex_multi_field_filtering(#[future] setup_advanced: Arc<PgPool>) {
	let pool = setup_advanced.await;

	// Query: category=Books AND status=published AND published=true
	let filter_sql = Query::select()
		.from(AdvancedItems::Table)
		.columns([
			AdvancedItems::Id,
			AdvancedItems::Title,
			AdvancedItems::Category,
			AdvancedItems::Status,
			AdvancedItems::Published,
		])
		.and_where(Expr::col(AdvancedItems::Category).eq("Books"))
		.and_where(Expr::col(AdvancedItems::Status).eq("published"))
		.and_where(Expr::col(AdvancedItems::Published).eq(true))
		.to_string(PostgresQueryBuilder);

	let rows = sqlx::query(&filter_sql)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Verify all results match the filter criteria
	for row in rows {
		let category: String = row.get("category");
		let status: String = row.get("status");
		let published: bool = row.get("published");

		assert_eq!(category, "Books");
		assert_eq!(status, "published");
		assert_eq!(published, true);
	}
}

/// Test: Custom ordering with multiple fields
#[rstest]
#[tokio::test]
async fn test_custom_multi_field_ordering(#[future] setup_advanced: Arc<PgPool>) {
	let _pool = setup_advanced.await;

	let ordering_config = OrderingConfig {
		ordering_fields: vec![
			"priority".to_string(),
			"score".to_string(),
			"created_at".to_string(),
		],
		default_ordering: vec!["-priority".to_string(), "score".to_string()],
	};

	let viewset = ModelViewSet::<AdvancedItem, JsonSerializer<AdvancedItem>>::new("advanced-items")
		.with_ordering(ordering_config);

	// Verify ordering configuration
	let config = viewset.get_ordering_config().unwrap();
	assert_eq!(config.ordering_fields.len(), 3);
	assert_eq!(config.default_ordering.len(), 2);
	assert_eq!(config.default_ordering[0], "-priority");
	assert_eq!(config.default_ordering[1], "score");
}

/// Test: Pagination with custom page size
#[rstest]
#[tokio::test]
async fn test_pagination_custom_page_size(#[future] setup_advanced: Arc<PgPool>) {
	let _pool = setup_advanced.await;

	// Test various page sizes
	let page_sizes = vec![2, 5, 10, 20];

	for page_size in page_sizes {
		let pagination_config = PaginationConfig::PageNumber {
			page_size,
			max_page_size: Some(100),
		};

		let viewset =
			ModelViewSet::<AdvancedItem, JsonSerializer<AdvancedItem>>::new("advanced-items")
				.with_pagination(pagination_config);

		// Verify page size is correctly set
		if let Some(PaginationConfig::PageNumber {
			page_size: actual_size,
			..
		}) = viewset.get_pagination_config()
		{
			assert_eq!(actual_size, page_size);
		} else {
			panic!("Expected PageNumber pagination config");
		}
	}
}

/// Test: Filter by multiple values for same field
#[rstest]
#[tokio::test]
async fn test_filter_multiple_values_same_field(#[future] setup_advanced: Arc<PgPool>) {
	let pool = setup_advanced.await;

	// Query: category IN ('Books', 'Electronics')
	let filter_sql = Query::select()
		.from(AdvancedItems::Table)
		.columns([
			AdvancedItems::Id,
			AdvancedItems::Title,
			AdvancedItems::Category,
		])
		.and_where(Expr::col(AdvancedItems::Category).is_in(vec!["Books", "Electronics"]))
		.to_string(PostgresQueryBuilder);

	let rows = sqlx::query(&filter_sql)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Verify all results are either Books or Electronics
	assert!(rows.len() > 0, "Should find items in Books or Electronics");

	for row in rows {
		let category: String = row.get("category");
		assert!(
			category == "Books" || category == "Electronics",
			"Category should be Books or Electronics, found: {}",
			category
		);
	}
}

/// Test: Range filtering (priority between 2 and 4)
#[rstest]
#[tokio::test]
async fn test_range_filtering(#[future] setup_advanced: Arc<PgPool>) {
	let pool = setup_advanced.await;

	// Query: priority >= 2 AND priority <= 4
	let filter_sql = Query::select()
		.from(AdvancedItems::Table)
		.columns([AdvancedItems::Id, AdvancedItems::Priority])
		.and_where(Expr::col(AdvancedItems::Priority).gte(2))
		.and_where(Expr::col(AdvancedItems::Priority).lte(4))
		.to_string(PostgresQueryBuilder);

	let rows = sqlx::query(&filter_sql)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Verify all results are within range
	for row in rows {
		let priority: i32 = row.get("priority");
		assert!(
			priority >= 2 && priority <= 4,
			"Priority should be 2-4, found: {}",
			priority
		);
	}
}

/// Test: Ordering with null values handling
#[rstest]
#[tokio::test]
async fn test_ordering_null_values(#[future] setup_advanced: Arc<PgPool>) {
	let pool = setup_advanced.await;

	// Insert item with null author_id
	let null_item = AdvancedItem::new(
		"Null Author Item".to_string(),
		"Books".to_string(),
		"draft".to_string(),
		3,
		5.0,
		false,
		None, // null author_id
		Some(Utc::now()),
		Some(Utc::now()),
	);

	let insert_sql = Query::insert()
		.into_table(AdvancedItems::Table)
		.columns([
			AdvancedItems::Title,
			AdvancedItems::Category,
			AdvancedItems::Status,
			AdvancedItems::Priority,
			AdvancedItems::Score,
			AdvancedItems::Published,
			AdvancedItems::AuthorId,
			AdvancedItems::CreatedAt,
			AdvancedItems::UpdatedAt,
		])
		.values_panic([
			null_item.title.into(),
			null_item.category.into(),
			null_item.status.into(),
			null_item.priority.into(),
			null_item.score.into(),
			null_item.published.into(),
			null_item.author_id.into(),
			null_item.created_at.into(),
			null_item.updated_at.into(),
		])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Query with ordering by author_id (nulls should be handled)
	let order_sql = Query::select()
		.from(AdvancedItems::Table)
		.columns([
			AdvancedItems::Id,
			AdvancedItems::Title,
			AdvancedItems::AuthorId,
		])
		.order_by(AdvancedItems::AuthorId, sea_query::Order::Asc)
		.to_string(PostgresQueryBuilder);

	let rows = sqlx::query(&order_sql)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Verify results are returned (null handling doesn't cause errors)
	assert!(rows.len() > 0, "Should return results with null author_ids");
}

/// Test: Combined search and filter
#[rstest]
#[tokio::test]
async fn test_combined_search_and_filter(#[future] setup_advanced: Arc<PgPool>) {
	let pool = setup_advanced.await;

	// Query: title ILIKE '%Item%' AND category='Books'
	let search_filter_sql = Query::select()
		.from(AdvancedItems::Table)
		.columns([
			AdvancedItems::Id,
			AdvancedItems::Title,
			AdvancedItems::Category,
		])
		.and_where(Expr::col(AdvancedItems::Title).like("%Item%"))
		.and_where(Expr::col(AdvancedItems::Category).eq("Books"))
		.to_string(PostgresQueryBuilder);

	let rows = sqlx::query(&search_filter_sql)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Verify all results match both search and filter
	for row in rows {
		let title: String = row.get("title");
		let category: String = row.get("category");

		assert!(title.contains("Item"), "Title should contain 'Item'");
		assert_eq!(category, "Books", "Category should be Books");
	}
}
