//! Integration tests requiring reinhardt-orm
//!
//! These tests verify the CollectionProxy integration with ORM models,
//! focusing on the actual API provided by reinhardt-proxy.

use reinhardt_orm::types::DatabaseDialect;
use reinhardt_orm::{
	Constraint, ForeignKeyConstraint, LoadingStrategy, Model, OnDelete, Relationship,
	RelationshipType,
};
use reinhardt_proxy::CollectionProxy;
use serde::{Deserialize, Serialize};

use rstest::*;

/// Test models for ORM integration testing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	id: Option<i64>,
	name: String,
	email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
	id: Option<i64>,
	user_id: i64,
	title: String,
	content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Role {
	id: Option<i64>,
	name: String,
}

// Implement Model trait for test models
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

impl Model for Post {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"posts"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

impl Model for Role {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"roles"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

/// Test creating CollectionProxy with basic configuration
#[rstest]
#[tokio::test]
async fn test_create_collection_proxy() {
	// Create basic proxy for keywords collection
	let proxy = CollectionProxy::new("keywords", "name");

	// Verify proxy was created successfully (no panics)
	assert_eq!(proxy.relationship(), "keywords");
	assert_eq!(proxy.attribute(), "name");
}

/// Test CollectionProxy with uniqueness constraint
#[rstest]
#[tokio::test]
async fn test_collection_proxy_unique() {
	// Create proxy with uniqueness enforced
	let proxy = CollectionProxy::unique("tags", "name");

	// Verify unique flag is set
	assert!(proxy.is_unique());
}

/// Test CollectionProxy with caching enabled
#[rstest]
#[tokio::test]
async fn test_collection_proxy_caching() {
	// Create proxy with caching
	let proxy = CollectionProxy::new("posts", "title")
		.with_caching(true)
		.with_cache_ttl(300); // 5 minutes

	// Verify caching configuration
	assert!(proxy.is_cached());
	assert_eq!(proxy.cache_ttl(), Some(300));
}

/// Test CollectionProxy with streaming enabled
#[rstest]
#[tokio::test]
async fn test_collection_proxy_streaming() {
	// Create proxy with streaming for large collections
	let proxy = CollectionProxy::new("large_collection", "data")
		.with_streaming(true)
		.with_chunk_size(100)
		.with_memory_limit(1024 * 1024); // 1MB limit

	// Verify proxy was created successfully with streaming options
	assert_eq!(proxy.relationship(), "large_collection");
	assert_eq!(proxy.attribute(), "data");
}

/// Test CollectionProxy builder pattern with cascade
#[rstest]
#[tokio::test]
async fn test_collection_proxy_cascade() {
	// Create proxy with cascade delete
	let proxy = CollectionProxy::new("posts", "title").with_cascade(true);

	// Note: cascade is a private field, but we can verify the proxy was built without errors
	assert_eq!(proxy.relationship(), "posts");
}

/// Test CollectionProxy builder pattern with batch size
#[rstest]
#[tokio::test]
async fn test_collection_proxy_batch_operations() {
	// Create proxy with batch size for bulk operations
	let proxy = CollectionProxy::new("keywords", "name").with_batch_size(100);

	// Note: batch_size is private, but we can verify the proxy was built without errors
	assert_eq!(proxy.attribute(), "name");
}

/// Test generating SQL JOIN queries for relationships
#[rstest]
#[tokio::test]
async fn test_relationship_join_sql_generation() {
	// Create relationship with Lazy strategy for SELECT-based loading
	let relationship = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
		.with_foreign_key("user_id")
		.with_lazy(LoadingStrategy::Lazy);

	// Generate SQL for Lazy loading (should produce SELECT query)
	let sql = relationship.load_sql("users.id", DatabaseDialect::PostgreSQL);

	// Verify SQL structure - Lazy loading generates SELECT, not JOIN
	assert!(sql.contains("SELECT"), "SQL should contain SELECT");
	assert!(sql.contains("posts"), "SQL should reference posts table");
	assert!(
		sql.contains("user_id"),
		"SQL should reference user_id foreign key"
	);
}

/// Test relationship loading strategies
#[rstest]
#[tokio::test]
async fn test_relationship_loading_strategies() {
	// Test different loading strategies
	let strategies = vec![
		LoadingStrategy::Lazy,
		LoadingStrategy::Joined,
		LoadingStrategy::Selectin,
		LoadingStrategy::Subquery,
	];

	for strategy in strategies {
		let relationship = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
			.with_foreign_key("user_id")
			.with_lazy(strategy);

		assert_eq!(relationship.lazy(), strategy);

		// Test SQL generation for each strategy
		let sql = relationship.load_sql("users.id", DatabaseDialect::PostgreSQL);
		match strategy {
			LoadingStrategy::Joined => {
				// Joined strategy returns empty string from load_sql()
				// (JOIN is handled at query builder level, not in load_sql)
				assert!(
					sql.is_empty(),
					"Joined loading returns empty from load_sql()"
				);
			}
			LoadingStrategy::Lazy | LoadingStrategy::Selectin | LoadingStrategy::Subquery => {
				// All these strategies generate basic SELECT statements from load_sql()
				// Subquery's IN clause is generated by build_subquery() method, not load_sql()
				assert!(
					sql.contains("SELECT")
						&& (sql.contains("FROM posts") || sql.contains("FROM \"posts\"")),
					"Lazy/Selectin/Subquery loading should generate SELECT FROM posts, got: {}",
					sql
				);
			}
			_ => {}
		}
	}
}

/// Test relationship with backref
#[rstest]
#[tokio::test]
async fn test_relationship_backref() {
	// Create bidirectional relationship
	let relationship = Relationship::<User, Post>::new("posts", RelationshipType::OneToMany)
		.with_foreign_key("user_id")
		.with_back_populates("author");

	// Verify relationship configuration
	assert_eq!(relationship.name(), "posts");
	assert_eq!(
		relationship.relationship_type(),
		RelationshipType::OneToMany
	);
}

/// Test polymorphic relationships
#[rstest]
#[tokio::test]
async fn test_polymorphic_relationship() {
	// Create self-referential relationship for tree structure
	let relationship = Relationship::<Role, Role>::new("subroles", RelationshipType::OneToMany)
		.with_foreign_key("parent_id")
		.with_remote_side(vec!["id".to_string()]);

	// Verify relationship configuration
	assert_eq!(relationship.name(), "subroles");
	assert_eq!(
		relationship.relationship_type(),
		RelationshipType::OneToMany
	);
}

/// Test database constraints
#[rstest]
#[tokio::test]
async fn test_foreign_key_constraint() {
	// Create foreign key constraint with cascade delete
	let constraint = ForeignKeyConstraint::new("posts_user_id_fk", "user_id", "users", "id")
		.on_delete(OnDelete::Cascade);

	// Verify constraint configuration using Constraint trait
	assert_eq!(constraint.name(), "posts_user_id_fk");
	assert_eq!(constraint.references_table, "users");
}

/// Test CollectionProxy with database-specific features
#[rstest]
#[tokio::test]
async fn test_collection_proxy_database_features() {
	// Create proxy with database-specific features
	let proxy = CollectionProxy::new("cross_db_data", "value")
		.with_database("read_replica")
		.with_fallback_database("primary");

	// Verify database configuration
	assert_eq!(proxy.database(), Some("read_replica"));
	assert_eq!(proxy.fallback_database(), Some("primary"));
}

/// Test CollectionProxy with procedure parameters
#[rstest]
#[tokio::test]
async fn test_collection_proxy_stored_procedures() {
	// Create proxy with stored procedure support
	let proxy = CollectionProxy::new("procedure_data", "result")
		.with_stored_procedure("get_user_data")
		.with_procedure_params(&[("user_id", "12345")]);

	// Verify stored procedure configuration
	assert_eq!(proxy.stored_procedure(), Some("get_user_data"));
	// procedure_params() returns &[(String, String)]
	assert!(!proxy.procedure_params().is_empty());
}

/// Test CollectionProxy with trigger events
#[rstest]
#[tokio::test]
async fn test_collection_proxy_triggers() {
	// Create proxy with trigger support
	let proxy = CollectionProxy::new("triggered_data", "value")
		.with_triggers(true)
		.with_trigger_events(&["INSERT", "UPDATE"]);

	// Verify trigger configuration
	assert!(proxy.has_triggers());
	// trigger_events() returns &[String]
	assert!(!proxy.trigger_events().is_empty());
}

/// Test CollectionProxy with view support
#[rstest]
#[tokio::test]
async fn test_collection_proxy_database_views() {
	// Create proxy for database view
	let proxy = CollectionProxy::new("user_posts_view", "title").with_view(true);

	// Verify view configuration
	assert!(proxy.is_view());
}

// TODO: Implement bulk_insert and clear operations tests after API stabilization
// These operations require database source and are better tested with actual models

/// Test CollectionProxy async loading support
#[rstest]
#[tokio::test]
async fn test_collection_proxy_async_loading() {
	// Create proxy with async loading
	let proxy = CollectionProxy::new("async_data", "value")
		.with_async_loading(true)
		.with_concurrent_access(true);

	// Verify async configuration
	assert!(proxy.is_async_loading());
	assert!(proxy.supports_concurrent_access());
}

/// Test many-to-many relationship configuration
#[rstest]
#[tokio::test]
async fn test_many_to_many_relationship() {
	// Create many-to-many relationship
	let relationship = Relationship::<User, Role>::new("roles", RelationshipType::ManyToMany)
		.with_join_condition("users.id = user_roles.user_id AND roles.id = user_roles.role_id");

	// Verify relationship configuration
	assert_eq!(relationship.name(), "roles");
	assert_eq!(
		relationship.relationship_type(),
		RelationshipType::ManyToMany
	);
}

/// Test one-to-one relationship configuration
#[rstest]
#[tokio::test]
async fn test_one_to_one_relationship() {
	// Create one-to-one relationship
	let relationship = Relationship::<User, User>::new("manager", RelationshipType::OneToOne)
		.with_foreign_key("manager_id")
		.with_back_populates("subordinates");

	// Verify relationship configuration
	assert_eq!(relationship.name(), "manager");
	assert_eq!(relationship.relationship_type(), RelationshipType::OneToOne);
}
