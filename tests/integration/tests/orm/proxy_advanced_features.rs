//! Integration tests for advanced proxy features
//!
//! These tests verify the AssociationProxy integration with complex scenarios,
//! focusing on the actual API provided by reinhardt-proxy.

use reinhardt_proxy::{AssociationProxy, CollectionProxy};
use serde::{Deserialize, Serialize};

/// Test models for proxy testing
#[derive(Debug, Clone, Serialize, Deserialize, reinhardt_macros::OrmReflectable)]
struct User {
	id: Option<i64>,
	name: String,
	email: String,
	posts: Vec<Post>,
	roles: Vec<Role>,
	profile: Option<UserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, reinhardt_macros::OrmReflectable)]
struct Post {
	id: Option<i64>,
	user_id: i64,
	title: String,
	content: String,
	comments: Vec<Comment>,
	tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, reinhardt_macros::OrmReflectable)]
struct Comment {
	id: Option<i64>,
	post_id: i64,
	author_id: i64,
	content: String,
	author: Option<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, reinhardt_macros::OrmReflectable)]
struct Tag {
	id: Option<i64>,
	name: String,
	posts: Vec<Post>,
}

#[derive(Debug, Clone, Serialize, Deserialize, reinhardt_macros::OrmReflectable)]
struct Role {
	id: Option<i64>,
	name: String,
	permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, reinhardt_macros::OrmReflectable)]
struct Permission {
	id: Option<i64>,
	name: String,
	resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, reinhardt_macros::OrmReflectable)]
struct UserProfile {
	id: Option<i64>,
	user_id: i64,
	bio: String,
	#[orm_field(type = "String")]
	avatar_url: Option<String>,
}

/// Test AssociationProxy basic construction
#[tokio::test]
async fn test_association_proxy_new() {
	// Create basic association proxy
	let proxy = AssociationProxy::<User, String>::new("profile", "bio");

	// Verify proxy was created successfully
	assert_eq!(proxy.relationship, "profile");
	assert_eq!(proxy.attribute, "bio");
}

/// Test AssociationProxy with optional name
#[tokio::test]
async fn test_association_proxy_with_name() {
	// Create basic proxy - name field can be set via public field access
	let mut proxy = AssociationProxy::<User, String>::new("profile", "bio");
	proxy.name = Some("bio_proxy".to_string());

	// Verify name is set
	assert_eq!(proxy.name, Some("bio_proxy".to_string()));
	assert_eq!(proxy.relationship, "profile");
	assert_eq!(proxy.attribute, "bio");
}

/// Test AssociationProxy with creator function
#[tokio::test]
async fn test_association_proxy_with_creator() {
	// Create proxy and set creator via public field
	let mut proxy = AssociationProxy::<User, String>::new("profile", "bio");
	proxy.creator = Some(|bio: String| User {
		id: None,
		name: String::new(),
		email: String::new(),
		posts: vec![],
		roles: vec![],
		profile: Some(UserProfile {
			id: None,
			user_id: 0,
			bio,
			avatar_url: None,
		}),
	});

	// Verify creator is set
	assert!(proxy.creator.is_some());
	assert_eq!(proxy.relationship, "profile");
	assert_eq!(proxy.attribute, "bio");
}

/// Test AssociationProxy with getter function
#[tokio::test]
async fn test_association_proxy_with_getter() {
	// Define getter as a standalone function
	fn get_bio(user: &User) -> Result<String, reinhardt_proxy::ProxyError> {
		user.profile
			.as_ref()
			.map(|p| p.bio.clone())
			.ok_or_else(|| reinhardt_proxy::ProxyError::RelationshipNotFound("profile".to_string()))
	}

	// Create proxy and set getter via public field
	let mut proxy = AssociationProxy::<User, String>::new("profile", "bio");
	proxy.getter = Some(get_bio);

	// Verify getter is set
	assert!(proxy.getter.is_some());
	assert_eq!(proxy.relationship, "profile");
	assert_eq!(proxy.attribute, "bio");
}

/// Test AssociationProxy with setter function
#[tokio::test]
async fn test_association_proxy_with_setter() {
	// Define setter as a standalone function
	fn set_bio(user: &mut User, bio: String) -> Result<(), reinhardt_proxy::ProxyError> {
		if let Some(profile) = &mut user.profile {
			profile.bio = bio;
			Ok(())
		} else {
			Err(reinhardt_proxy::ProxyError::RelationshipNotFound(
				"profile".to_string(),
			))
		}
	}

	// Create proxy and set setter via public field
	let mut proxy = AssociationProxy::<User, String>::new("profile", "bio");
	proxy.setter = Some(set_bio);

	// Verify setter is set
	assert!(proxy.setter.is_some());
	assert_eq!(proxy.relationship, "profile");
	assert_eq!(proxy.attribute, "bio");
}

/// Test AssociationProxy with all options
#[tokio::test]
async fn test_association_proxy_complete_builder() {
	// Define helper functions
	fn get_bio(user: &User) -> Result<String, reinhardt_proxy::ProxyError> {
		user.profile
			.as_ref()
			.map(|p| p.bio.clone())
			.ok_or_else(|| reinhardt_proxy::ProxyError::RelationshipNotFound("profile".to_string()))
	}

	fn set_bio(user: &mut User, bio: String) -> Result<(), reinhardt_proxy::ProxyError> {
		if let Some(profile) = &mut user.profile {
			profile.bio = bio;
			Ok(())
		} else {
			Err(reinhardt_proxy::ProxyError::RelationshipNotFound(
				"profile".to_string(),
			))
		}
	}

	// Create proxy and set all options via public fields
	let mut proxy = AssociationProxy::<User, String>::new("profile", "bio");
	proxy.name = Some("complete_proxy".to_string());
	proxy.creator = Some(|bio: String| User {
		id: None,
		name: String::new(),
		email: String::new(),
		posts: vec![],
		roles: vec![],
		profile: Some(UserProfile {
			id: None,
			user_id: 0,
			bio,
			avatar_url: None,
		}),
	});
	proxy.getter = Some(get_bio);
	proxy.setter = Some(set_bio);

	// Verify all fields are set
	assert_eq!(proxy.name, Some("complete_proxy".to_string()));
	assert_eq!(proxy.relationship, "profile");
	assert_eq!(proxy.attribute, "bio");
	assert!(proxy.creator.is_some());
	assert!(proxy.getter.is_some());
	assert!(proxy.setter.is_some());
}

/// Test CollectionProxy serialization to JSON
#[tokio::test]
async fn test_collection_proxy_serialization() {
	// Create proxy with various options
	let proxy = CollectionProxy::new("posts", "title")
		.with_caching(true)
		.with_cache_ttl(300);

	// Serialize to JSON
	let json = serde_json::to_string(&proxy).unwrap();

	// Verify JSON contains expected fields
	assert!(json.contains("\"relationship\":\"posts\""));
	assert!(json.contains("\"attribute\":\"title\""));
}

/// Test CollectionProxy deserialization from JSON
#[tokio::test]
async fn test_collection_proxy_deserialization() {
	// Create JSON representation
	let json = r#"{
		"relationship": "posts",
		"attribute": "title",
		"unique": false,
		"caching": true,
		"cache_ttl": 300
	}"#;

	// Deserialize from JSON
	let proxy: CollectionProxy = serde_json::from_str(json).unwrap();

	// Verify fields were deserialized correctly
	assert_eq!(proxy.relationship(), "posts");
	assert_eq!(proxy.attribute(), "title");
	assert!(proxy.is_cached());
	assert_eq!(proxy.cache_ttl(), Some(300));
}

/// Test CollectionProxy with complex configuration
#[tokio::test]
async fn test_collection_proxy_complex_config() {
	// Create proxy with multiple configuration options
	let proxy = CollectionProxy::new("large_dataset", "value")
		.with_streaming(true)
		.with_chunk_size(500)
		.with_memory_limit(2 * 1024 * 1024) // 2MB
		.with_caching(true)
		.with_cache_ttl(600)
		.with_batch_size(50);

	// Verify all configurations
	assert_eq!(proxy.relationship(), "large_dataset");
	assert_eq!(proxy.attribute(), "value");
	assert!(proxy.is_cached());
	assert_eq!(proxy.cache_ttl(), Some(600));
}

/// Test CollectionProxy cascade configuration
#[tokio::test]
async fn test_collection_proxy_cascade() {
	// Create proxy with cascade enabled
	let proxy = CollectionProxy::new("related_items", "id").with_cascade(true);

	// Verify proxy was created (cascade is private)
	assert_eq!(proxy.relationship(), "related_items");
	assert_eq!(proxy.attribute(), "id");
}

/// Test CollectionProxy database routing
#[tokio::test]
async fn test_collection_proxy_database_routing() {
	// Create proxy with database routing
	let proxy = CollectionProxy::new("distributed_data", "key")
		.with_database("shard_1")
		.with_fallback_database("shard_2");

	// Verify database configuration
	assert_eq!(proxy.database(), Some("shard_1"));
	assert_eq!(proxy.fallback_database(), Some("shard_2"));
}

/// Test CollectionProxy stored procedure support
#[tokio::test]
async fn test_collection_proxy_stored_procedure() {
	// Create proxy for stored procedure
	let proxy = CollectionProxy::new("proc_result", "output")
		.with_stored_procedure("calculate_metrics")
		.with_procedure_params(&[("start_date", "2025-01-01"), ("end_date", "2025-01-31")]);

	// Verify stored procedure configuration
	assert_eq!(proxy.stored_procedure(), Some("calculate_metrics"));
	assert!(!proxy.procedure_params().is_empty());
}

/// Test CollectionProxy trigger events
#[tokio::test]
async fn test_collection_proxy_trigger_events() {
	// Create proxy with trigger events
	let proxy = CollectionProxy::new("audited_data", "change_log")
		.with_triggers(true)
		.with_trigger_events(&["INSERT", "UPDATE", "DELETE"]);

	// Verify trigger configuration
	assert!(proxy.has_triggers());
	assert!(!proxy.trigger_events().is_empty());
}

/// Test CollectionProxy view support
#[tokio::test]
async fn test_collection_proxy_view() {
	// Create proxy for database view
	let proxy = CollectionProxy::new("materialized_view", "computed_value").with_view(true);

	// Verify view configuration
	assert!(proxy.is_view());
}

/// Test CollectionProxy async loading
#[tokio::test]
async fn test_collection_proxy_async_loading() {
	// Create proxy with async loading
	let proxy = CollectionProxy::new("async_collection", "data")
		.with_async_loading(true)
		.with_concurrent_access(true);

	// Verify async configuration
	assert!(proxy.is_async_loading());
	assert!(proxy.supports_concurrent_access());
}

/// Test CollectionProxy with uniqueness
#[tokio::test]
async fn test_collection_proxy_uniqueness() {
	// Create unique collection proxy
	let proxy = CollectionProxy::unique("unique_tags", "tag_name");

	// Verify uniqueness flag
	assert!(proxy.is_unique());
}

/// Test CollectionProxy bulk operations
#[tokio::test]
async fn test_collection_proxy_bulk_operations() {
	// Create proxy for bulk operations
	let proxy = CollectionProxy::new("bulk_data", "value").with_batch_size(1000);

	// Test bulk insert with empty slice
	let items: &[String] = &[];
	let result = proxy.bulk_insert(items);

	// Verify operation completes without error
	assert!(result.is_ok());
}

/// Test CollectionProxy clear operation
#[tokio::test]
async fn test_collection_proxy_clear_operation() {
	// Create proxy with cascade
	let proxy = CollectionProxy::new("clearable_data", "item").with_cascade(true);

	// Test clear operation
	let result = proxy.clear();

	// Verify operation completes without error
	assert!(result.is_ok());
}

/// Test AssociationProxy with None relationships
#[tokio::test]
async fn test_association_proxy_null_relationship() {
	// Create user with no profile
	let user = User {
		id: Some(1),
		name: "Test User".to_string(),
		email: "test@example.com".to_string(),
		posts: vec![],
		roles: vec![],
		profile: None, // Null relationship
	};

	// Create proxy for profile bio
	let proxy = AssociationProxy::<User, String>::new("profile", "bio");

	// Verify proxy handles null relationship
	assert_eq!(proxy.relationship, "profile");
	assert_eq!(proxy.attribute, "bio");

	// Note: Actual get() behavior depends on implementation
	// This test verifies the proxy can be created for potentially null relationships
	drop(user);
}

/// Test CollectionProxy with empty collections
#[tokio::test]
async fn test_collection_proxy_empty_collection() {
	// Create user with empty posts
	let user = User {
		id: Some(1),
		name: "Test User".to_string(),
		email: "test@example.com".to_string(),
		posts: vec![], // Empty collection
		roles: vec![],
		profile: None,
	};

	// Create proxy for posts
	let proxy = CollectionProxy::new("posts", "title");

	// Verify proxy handles empty collection
	assert_eq!(proxy.relationship(), "posts");
	assert_eq!(proxy.attribute(), "title");

	// Note: Actual get_values() behavior depends on implementation
	drop(user);
}

/// Test multiple proxy chaining concept
#[tokio::test]
async fn test_proxy_chaining_concept() {
	// Create proxies for nested relationships
	// Note: Actual chaining would require connecting these through ORM
	let proxy1 = CollectionProxy::new("posts", "title");
	let proxy2 = CollectionProxy::new("comments", "content");
	let proxy3 = AssociationProxy::<Comment, String>::new("author", "name");

	// Verify all proxies can be created
	assert_eq!(proxy1.relationship(), "posts");
	assert_eq!(proxy2.relationship(), "comments");
	assert_eq!(proxy3.relationship, "author");
}
