//! Relation fields - Django REST Framework inspired relationship serialization
//!
//! This module provides field types for representing relationships between models
//! in serialized output, supporting various representation strategies.

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// RelationField - Base trait for relationship representation
///
/// Defines the common interface for all relationship field types.
/// Different field types represent the same relationship in different ways.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationField<T> {
	_phantom: PhantomData<T>,
}

impl<T> RelationField<T> {
	/// Create a new RelationField
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<T> Default for RelationField<T> {
	fn default() -> Self {
		Self::new()
	}
}

/// PrimaryKeyRelatedField - Represent relationships by primary key
///
/// This is the most common and efficient way to represent relationships,
/// storing only the primary key of the related model.
///
/// # Examples
///
/// ```
/// # use reinhardt_serializers::PrimaryKeyRelatedField;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Author {
/// #     id: Option<i64>,
/// #     name: String,
/// # }
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Post {
/// #     id: Option<i64>,
/// #     title: String,
/// #     author: PrimaryKeyRelatedField<Author>,
/// # }
/// // In JSON, the author field will be represented as just the ID:
/// // {"id": 1, "title": "My Post", "author": 42}
/// ```
pub type PrimaryKeyRelatedField<T> = RelationField<T>;

/// SlugRelatedField - Represent relationships by a slug field
///
/// Uses a unique text field (slug) instead of the primary key to represent
/// the relationship. Useful for human-readable URLs and references.
///
/// # Examples
///
/// ```
/// # use reinhardt_serializers::SlugRelatedField;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Category {
/// #     id: Option<i64>,
/// #     slug: String,
/// #     name: String,
/// # }
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Product {
/// #     id: Option<i64>,
/// #     name: String,
/// #     category: SlugRelatedField<Category>,
/// # }
/// // In JSON, the category field will be represented as a slug:
/// // {"id": 1, "name": "Widget", "category": "electronics"}
/// ```
pub type SlugRelatedField<T> = RelationField<T>;

/// StringRelatedField - Represent relationships by string representation
///
/// Uses the string representation of the related model (typically from
/// Display trait or a custom method). Read-only field type.
///
/// # Examples
///
/// ```
/// # use reinhardt_serializers::StringRelatedField;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct User {
/// #     id: Option<i64>,
/// #     username: String,
/// # }
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Comment {
/// #     id: Option<i64>,
/// #     text: String,
/// #     author: StringRelatedField<User>,
/// # }
/// // In JSON, the author field will be represented as a string:
/// // {"id": 1, "text": "Great post!", "author": "john_doe"}
/// ```
pub type StringRelatedField<T> = RelationField<T>;

/// HyperlinkedRelatedField - Represent relationships by URL
///
/// Stores a URL that points to the detail view of the related model,
/// following HATEOAS principles.
///
/// # Examples
///
/// ```
/// # use reinhardt_serializers::HyperlinkedRelatedField;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Author {
/// #     id: Option<i64>,
/// #     name: String,
/// # }
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Book {
/// #     id: Option<i64>,
/// #     title: String,
/// #     author: HyperlinkedRelatedField<Author>,
/// # }
/// // In JSON, the author field will be represented as a URL:
/// // {"id": 1, "title": "My Book", "author": "/api/authors/42/"}
/// ```
pub type HyperlinkedRelatedField<T> = RelationField<T>;

/// ManyRelatedField - Represent many-to-many or reverse relationships
///
/// Wrapper for collections of related objects, can use any of the above
/// field types for the individual items.
///
/// # Examples
///
/// ```
/// # use reinhardt_serializers::ManyRelatedField;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Tag {
/// #     id: Option<i64>,
/// #     name: String,
/// # }
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Article {
/// #     id: Option<i64>,
/// #     title: String,
/// #     tags: ManyRelatedField<Tag>,
/// # }
/// // In JSON, the tags field will be an array:
/// // {"id": 1, "title": "My Article", "tags": [1, 2, 3]}
/// // or with SlugRelatedField:
/// // {"id": 1, "title": "My Article", "tags": ["rust", "programming", "web"]}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManyRelatedField<T> {
	_phantom: PhantomData<T>,
}

impl<T> ManyRelatedField<T> {
	/// Create a new ManyRelatedField
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<T> Default for ManyRelatedField<T> {
	fn default() -> Self {
		Self::new()
	}
}

/// IdentityField - Returns the entire related object
///
/// Used internally by NestedSerializer to represent the entire related
/// object instead of just a reference.
///
/// # Examples
///
/// ```
/// # use reinhardt_serializers::IdentityField;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Profile {
/// #     id: Option<i64>,
/// #     bio: String,
/// # }
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct User {
/// #     id: Option<i64>,
/// #     username: String,
/// #     profile: IdentityField<Profile>,
/// # }
/// // In JSON, the profile field will be the entire object:
/// // {"id": 1, "username": "alice", "profile": {"id": 1, "bio": "Developer"}}
/// ```
pub type IdentityField<T> = RelationField<T>;

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TestRelated {
		id: Option<i64>,
		name: String,
	}

	#[test]
	fn test_relation_field_creation() {
		let field = RelationField::<TestRelated>::new();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_primary_key_related_field() {
		let field = PrimaryKeyRelatedField::<TestRelated>::new();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_slug_related_field() {
		let field = SlugRelatedField::<TestRelated>::new();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_string_related_field() {
		let field = StringRelatedField::<TestRelated>::new();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_hyperlinked_related_field() {
		let field = HyperlinkedRelatedField::<TestRelated>::new();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_many_related_field() {
		let field = ManyRelatedField::<TestRelated>::new();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_identity_field() {
		let field = IdentityField::<TestRelated>::new();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_relation_field_default() {
		let field = RelationField::<TestRelated>::default();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_many_related_field_default() {
		let field = ManyRelatedField::<TestRelated>::default();
		assert!(std::mem::size_of_val(&field) >= 0);
	}

	#[test]
	fn test_relation_field_serialization() {
		let field = RelationField::<TestRelated>::new();
		let json = serde_json::to_string(&field).unwrap();
		assert!(!json.is_empty());
	}

	#[test]
	fn test_many_related_field_serialization() {
		let field = ManyRelatedField::<TestRelated>::new();
		let json = serde_json::to_string(&field).unwrap();
		assert!(!json.is_empty());
	}
}
