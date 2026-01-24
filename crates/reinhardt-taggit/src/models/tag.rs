//! Tag model definition
//!
//! Core tag entity with normalized name and URL-friendly slug.

use chrono::{DateTime, Utc};
use reinhardt_db::prelude::*;
use serde::{Deserialize, Serialize};

/// Core tag entity
///
/// Represents a tag with a normalized name and auto-generated slug.
/// Tags are globally unique by name (case-insensitive if configured).
///
/// # Database Schema
///
/// ```sql
/// CREATE TABLE tags (
///     id BIGSERIAL PRIMARY KEY,
///     name VARCHAR(255) NOT NULL,
///     slug VARCHAR(255) NOT NULL,
///     created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
///     UNIQUE(name),
///     UNIQUE(slug)
/// );
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_taggit::Tag;
///
/// let tag = Tag {
///     id: None,
///     name: "rust-programming".to_string(),
///     slug: "rust-programming".to_string(),
///     created_at: Utc::now(),
/// };
/// ```
#[model(app_label = "taggit", table_name = "tags")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
	/// Primary key
	#[field(primary_key = true)]
	pub id: Option<i64>,

	/// Normalized tag name (unique, case-insensitive if configured)
	#[field(max_length = 255, unique = true)]
	pub name: String,

	/// URL-friendly slug (unique, auto-generated from name)
	#[field(max_length = 255, unique = true)]
	pub slug: String,

	/// Creation timestamp
	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
}

impl Tag {
	/// Create a new Tag instance
	///
	/// Note: This does not save to the database. Use `` `save()` `` or `` `objects().create()` ``.
	///
	/// # Arguments
	///
	/// * `name` - Tag name (will be normalized)
	/// * `slug` - URL-friendly slug
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let tag = Tag::new("Rust Programming", "rust-programming");
	/// ```
	pub fn new(name: impl Into<String>, slug: impl Into<String>) -> Self {
		Self {
			id: None,
			name: name.into(),
			slug: slug.into(),
			created_at: Utc::now(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_tag_creation() {
		let tag = Tag::new("rust", "rust");
		assert_eq!(tag.name, "rust");
		assert_eq!(tag.slug, "rust");
		assert!(tag.id.is_none());
	}
}
