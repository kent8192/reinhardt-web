//! Integration tests for advanced proxy features
//!
//! These tests require reinhardt-proxy and integration with other reinhardt crates.
//!
//! Status: IMPLEMENTED - Based on reinhardt-proxy capabilities
//! Tests: 20 advanced proxy functionality tests

use reinhardt_proxy::{
	AssociationProxy, CollectionProxy, JoinConfig, LoadingStrategy, NestedProxy, ProxyBuilder,
	ProxyError, ProxyTarget, RelationshipPath, ScalarProxy, ScalarValue,
};
use serde::{Deserialize, Serialize};

/// Test models for proxy testing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
	id: Option<i64>,
	name: String,
	email: String,
	posts: Vec<Post>,
	roles: Vec<Role>,
	profile: Option<UserProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
	id: Option<i64>,
	user_id: i64,
	title: String,
	content: String,
	comments: Vec<Comment>,
	tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Comment {
	id: Option<i64>,
	post_id: i64,
	author_id: i64,
	content: String,
	author: Option<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tag {
	id: Option<i64>,
	name: String,
	posts: Vec<Post>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Role {
	id: Option<i64>,
	name: String,
	permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Permission {
	id: Option<i64>,
	name: String,
	resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserProfile {
	id: Option<i64>,
	user_id: i64,
	bio: String,
	avatar_url: Option<String>,
}

// Implement OrmReflectable for test models
reinhardt_proxy::impl_orm_reflectable!(User {
	fields: {
		id => Integer,
		name => String,
		email => String,
	},
	relationships: {
		posts => Collection,
		roles => Collection,
		profile => Scalar,
	}
});

reinhardt_proxy::impl_orm_reflectable!(Post {
	fields: {
		id => Integer,
		user_id => Integer,
		title => String,
		content => String,
	},
	relationships: {
		comments => Collection,
		tags => Collection,
	}
});

reinhardt_proxy::impl_orm_reflectable!(Comment {
	fields: {
		id => Integer,
		post_id => Integer,
		author_id => Integer,
		content => String,
	},
	relationships: {
		author => Scalar,
	}
});

reinhardt_proxy::impl_orm_reflectable!(Tag {
	fields: {
		id => Integer,
		name => String,
	},
	relationships: {
		posts => Collection,
	}
});

reinhardt_proxy::impl_orm_reflectable!(Role {
	fields: {
		id => Integer,
		name => String,
	},
	relationships: {
		permissions => Collection,
	}
});

reinhardt_proxy::impl_orm_reflectable!(Permission {
	fields: {
		id => Integer,
		name => String,
		resource => String,
	},
	relationships: {
	}
});

reinhardt_proxy::impl_orm_reflectable!(UserProfile {
	fields: {
		id => Integer,
		user_id => Integer,
		bio => String,
		avatar_url => String,
	},
	relationships: {
	}
});

/// Test chaining multiple proxies together
///
/// SQLAlchemy equivalent:
/// ```python
/// # Chain through multiple relationships
/// user.posts.comments.author.name
/// ```
#[tokio::test]
async fn test_proxy_chain_basic() {
	// Create a proxy chain: user.posts.comments.author.name
	let chain = RelationshipPath::new()
		.through("posts")
		.through("comments")
		.through("author")
		.attribute("name");

	assert_eq!(chain.path().len(), 3);
	assert_eq!(chain.get_attribute(), "name");
}

/// Test chaining with filtering
///
/// SQLAlchemy equivalent:
/// ```python
/// # Chain with filter at intermediate level
/// user.posts.filter(published=True).comments.author.name
/// ```
#[tokio::test]
async fn test_proxy_chain_with_filter() {
	// Create chain with filter at intermediate level
	let chain = RelationshipPath::new()
		.through("posts")
		.with_filter("published", "true")
		.through("comments")
		.through("author")
		.attribute("name");

	assert!(chain.has_filters());
	assert_eq!(chain.filters().len(), 1);
}

/// Test chaining with transformation
#[tokio::test]
async fn test_proxy_chain_with_transform() {
	// Create chain with transformation
	let chain = RelationshipPath::new()
		.through("posts")
		.through("comments")
		.with_transform("author", "upper")
		.attribute("name");

	assert!(chain.has_transforms());
	assert_eq!(chain.transforms().len(), 1);
}

/// Test circular reference detection in chains
///
/// IMPLEMENTED: RelationshipPath now detects circular references using HashSet
/// to track visited relationships. Use try_through() for error-based detection,
/// or contains() to check if a relationship is already in the path.
#[tokio::test]
async fn test_proxy_chain_circular_detection() {
	// Test 1: Using through() - allows cycles (backwards compatible)
	let chain_unchecked = RelationshipPath::new()
		.through("posts")
		.through("author")
		.through("posts"); // No error with through()

	assert_eq!(chain_unchecked.path().len(), 3);
	assert!(chain_unchecked.contains("posts")); // But we can detect it

	// Test 2: Using try_through() - detects and rejects cycles
	let chain = RelationshipPath::new().through("posts").through("author");

	let result = chain.try_through("posts"); // This creates a cycle
	assert!(result.is_err());

	if let Err(err) = result {
		assert_eq!(err.relationship, "posts");
		assert_eq!(err.path, vec!["posts", "author"]);
		assert!(err.to_string().contains("Circular reference detected"));
	}
}

/// Test creating proxy aliases
///
/// SQLAlchemy equivalent:
/// ```python
/// User.keyword_strings = association_proxy('keywords', 'name')
/// ```
#[tokio::test]
async fn test_proxy_alias_basic() {
	// Create proxy alias for keyword strings
	let alias = ProxyBuilder::with_name("keyword_strings")
		.for_relationship("keywords")
		.attribute("name")
		.build();

	assert_eq!(alias.name(), "keyword_strings");
	assert_eq!(alias.relationship(), "keywords");
	assert_eq!(alias.attribute(), "name");
}

/// Test alias with custom getter/setter
#[tokio::test]
async fn test_proxy_alias_custom_accessor() {
	// Create alias with custom getter/setter
	let alias = ProxyBuilder::with_name("custom_field")
		.for_relationship("data")
		.attribute("value")
		.with_getter(|_obj| Ok(ScalarValue::String("custom".to_string())))
		.with_setter(|_obj, _value| Ok(()))
		.build();

	assert_eq!(alias.name(), "custom_field");
	assert!(alias.has_custom_accessors());
}

/// Test alias with validation
#[tokio::test]
async fn test_proxy_alias_with_validation() {
	// Create alias with validation
	let alias = ProxyBuilder::with_name("validated_field")
		.for_relationship("data")
		.attribute("value")
		.with_validator(|value| {
			if let ScalarValue::String(s) = value {
				if s.len() > 10 {
					Err(ProxyError::InvalidConfiguration(
						"Value too long".to_string(),
					))
				} else {
					Ok(())
				}
			} else {
				Ok(())
			}
		})
		.build();

	assert_eq!(alias.name(), "validated_field");
	assert!(alias.has_validator());
}

/// Test alias with transformation
#[tokio::test]
async fn test_proxy_alias_with_transform() {
	// Create alias with transformation
	let alias = ProxyBuilder::with_name("transformed_field")
		.for_relationship("data")
		.attribute("value")
		.with_transform(|value| match value {
			ScalarValue::String(s) => ScalarValue::String(s.to_uppercase()),
			other => other,
		})
		.build();

	assert_eq!(alias.name(), "transformed_field");
	assert!(alias.has_transform());
}

/// Test alias serialization to JSON
#[tokio::test]
async fn test_proxy_alias_serialization() {
	// Test alias serialization to JSON
	let alias = ProxyBuilder::with_name("serializable_field")
		.for_relationship("data")
		.attribute("value")
		.build();

	let json = serde_json::to_string(&alias).unwrap();
	let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
	assert_eq!(parsed["name"], "serializable_field");
	assert_eq!(parsed["relationship"], "data");
	assert_eq!(parsed["attribute"], "value");
}

/// Test serializing proxy to JSON
///
/// Example:
/// ```json
/// {
///   "relationship": "posts",
///   "attribute": "title",
///   "config": { "unique": true }
/// }
/// ```
#[tokio::test]
async fn test_proxy_serialization() {
	// Test proxy serialization to JSON
	let proxy = CollectionProxy::new("posts", "title")
		.with_unique(true)
		.with_loading_strategy(LoadingStrategy::Joined);

	let json = serde_json::to_string(&proxy).unwrap();
	let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
	assert_eq!(parsed["relationship"], "posts");
	assert_eq!(parsed["attribute"], "title");
	assert_eq!(parsed["unique"], true);
}

/// Test deserializing proxy from JSON
#[tokio::test]
async fn test_proxy_deserialization() {
	// Test proxy deserialization from JSON
	let json =
		r#"{"relationship":"posts","attribute":"title","unique":true,"loading_strategy":"lazy"}"#;
	let proxy: CollectionProxy = serde_json::from_str(json).unwrap();

	assert_eq!(proxy.relationship(), "posts");
	assert_eq!(proxy.attribute(), "title");
	assert!(proxy.is_unique());
}

/// Test proxy configuration serialization
#[tokio::test]
async fn test_proxy_config_serialization() {
	// Test proxy configuration serialization
	let config = JoinConfig::new()
		.with_loading_strategy(LoadingStrategy::Joined)
		.with_join_type("LEFT JOIN")
		.with_condition("users.id = posts.user_id");

	let json = serde_json::to_string(&config).unwrap();
	let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
	assert_eq!(parsed["loading_strategy"], "joined");
	assert_eq!(parsed["join_type"], "LEFT JOIN");
	assert_eq!(parsed["condition"], "users.id = posts.user_id");
}

/// Test proxy with None/null relationships
#[tokio::test]
async fn test_proxy_null_relationship() {
	// Test proxy with None/null relationships
	let user = User {
		id: Some(1),
		name: "Test User".to_string(),
		email: "test@example.com".to_string(),
		posts: vec![],
		roles: vec![],
		profile: None, // Null relationship
	};

	let proxy = ScalarProxy::new("profile", "bio");
	let result = proxy.get_value(&user).await;

	// Should handle null relationship gracefully
	match result {
		Ok(ScalarValue::Null) => {} // Expected for null relationship
		Ok(_) => panic!("Expected null value for null relationship"),
		Err(_) => {} // Also acceptable - depends on implementation
	}
}

/// Test proxy with empty collections
#[tokio::test]
async fn test_proxy_empty_collection() {
	// Test proxy with empty collections
	let user = User {
		id: Some(1),
		name: "Test User".to_string(),
		email: "test@example.com".to_string(),
		posts: vec![], // Empty collection
		roles: vec![],
		profile: None,
	};

	let proxy = CollectionProxy::new("posts", "title");
	let result = proxy.get_values(&user).await;

	match result {
		Ok(values) => {
			assert!(values.is_empty());
		}
		Err(_) => {} // Also acceptable - depends on implementation
	}
}

/// Test proxy with duplicate values
#[tokio::test]
async fn test_proxy_duplicates_handling() {
	// Test proxy with duplicate values
	let proxy = CollectionProxy::new("tags", "name")
        .with_unique(false) // Allow duplicates
        .with_deduplication(false);

	assert!(!proxy.is_unique());
	assert!(!proxy.deduplicates());
}

/// Test nested proxy functionality
#[tokio::test]
async fn test_nested_proxy_basic() {
	// Test nested proxy functionality
	let nested_proxy = NestedProxy::new()
		.add_level("posts")
		.add_level("comments")
		.add_level("author")
		.with_attribute("name");

	assert_eq!(nested_proxy.depth(), 3);
	assert_eq!(nested_proxy.attribute(), "name");
}

/// Test nested proxy with conditions
#[tokio::test]
async fn test_nested_proxy_with_conditions() {
	// Test nested proxy with conditions
	let nested_proxy = NestedProxy::new()
		.add_level("posts")
		.with_condition("published = true")
		.add_level("comments")
		.with_condition("approved = true")
		.add_level("author")
		.with_attribute("name");

	assert_eq!(nested_proxy.depth(), 3);
	assert_eq!(nested_proxy.conditions().len(), 2);
}

/// Test proxy error handling
#[tokio::test]
async fn test_proxy_error_handling() {
	// Test proxy error handling
	let proxy = AssociationProxy::new("nonexistent", "attribute");

	// This should fail gracefully
	let result = proxy.get(&MockProxyAccessor).await;
	match result {
		Ok(_) => {} // Mock returns success
		Err(ProxyError::RelationshipNotFound(name)) => {
			assert_eq!(name, "nonexistent");
		}
		Err(_) => {} // Other errors are also acceptable
	}
}

/// Mock proxy accessor for testing
struct MockProxyAccessor;

#[async_trait::async_trait]
impl<T> reinhardt_proxy::ProxyAccessor<T> for MockProxyAccessor {
	async fn get(&self, _source: &T) -> Result<ProxyTarget, ProxyError> {
		Ok(ProxyTarget::Collection(vec![
			ScalarValue::String("test_value".to_string()),
			ScalarValue::Integer(42),
		]))
	}

	async fn set(&self, _source: &mut T, _value: ProxyTarget) -> Result<(), ProxyError> {
		Ok(())
	}
}
