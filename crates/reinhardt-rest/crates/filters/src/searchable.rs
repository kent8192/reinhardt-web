//! Searchable model trait
//!
//! Defines which fields are searchable and the default ordering for a model.

use crate::ordering_field::OrderingField;
use reinhardt_db::orm::{Field, Model};

/// Trait for models that support search and ordering
///
/// Implement this trait to define which fields can be searched
/// and what the default ordering should be.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_filters::SearchableModel;
/// use reinhardt_db::orm::{Model, Field};
///
/// #[model(app_label = "posts", table_name = "posts")]
/// struct Post {
///     id: i64,
///     title: String,
///     content: String,
///     created_at: DateTime,
/// }
///
/// impl SearchableModel for Post {
///     fn searchable_fields() -> Vec<Field<Self, String>> {
///         vec![
///             Field::new(vec!["title"]),
///             Field::new(vec!["content"]),
///         ]
///     }
///
///     fn default_ordering() -> Vec<OrderingField<Self>> {
///         use reinhardt_filters::FieldOrderingExt;
///         vec![Field::new(vec!["created_at"]).desc()]
///     }
/// }
/// ```
pub trait SearchableModel: Model {
	/// Get the list of searchable string fields
	///
	/// These fields will be used for text search operations.
	fn searchable_fields() -> Vec<Field<Self, String>> {
		Vec::new()
	}

	/// Get the default ordering
	///
	/// Returns an empty vector by default (no ordering).
	/// Override to specify default sort order.
	fn default_ordering() -> Vec<OrderingField<Self>> {
		Vec::new()
	}

	/// Get searchable field names as strings (for compatibility)
	///
	/// This is a helper method that extracts field names from searchable_fields().
	fn searchable_field_names() -> Vec<String> {
		Self::searchable_fields()
			.into_iter()
			.map(|field| field.path().join("."))
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field_extensions::FieldOrderingExt;

	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestPost {
		id: i64,
		title: String,
		content: String,
		created_at: String,
	}

	impl Model for TestPost {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"test_posts"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			Some(&self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}
	}

	impl SearchableModel for TestPost {
		fn searchable_fields() -> Vec<Field<Self, String>> {
			vec![Field::new(vec!["title"]), Field::new(vec!["content"])]
		}

		fn default_ordering() -> Vec<OrderingField<Self>> {
			vec![Field::<Self, String>::new(vec!["created_at"]).desc()]
		}
	}

	#[test]
	fn test_searchable_fields() {
		let fields = TestPost::searchable_fields();
		assert_eq!(fields.len(), 2);
		assert_eq!(fields[0].path(), &["title"]);
		assert_eq!(fields[1].path(), &["content"]);
	}

	#[test]
	fn test_searchable_field_names() {
		let names = TestPost::searchable_field_names();
		assert_eq!(names, vec!["title", "content"]);
	}

	#[test]
	fn test_default_ordering() {
		let ordering = TestPost::default_ordering();
		assert_eq!(ordering.len(), 1);
		assert_eq!(ordering[0].field_path(), &["created_at"]);
	}
}
