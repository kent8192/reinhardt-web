//! Extensions for reinhardt-orm's Field type
//!
//! Adds ordering methods (.asc(), .desc()) to Field<M, T> similar to how
//! field_lookup adds comparison methods.

use crate::ordering_field::{OrderDirection, OrderingField};
use reinhardt_orm::{Field, Model};

/// Extension trait to add ordering methods to Field
pub trait FieldOrderingExt<M: Model, T> {
	/// Create an ascending ordering from this field
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let order = Field::<Post, String>::new(vec!["title"]).asc();
	// Generates: ORDER BY title ASC
	/// ```
	fn asc(self) -> OrderingField<M>;

	/// Create a descending ordering from this field
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let order = Field::<Post, String>::new(vec!["created_at"]).desc();
	// Generates: ORDER BY created_at DESC
	/// ```
	fn desc(self) -> OrderingField<M>;
}

impl<M: Model, T> FieldOrderingExt<M, T> for Field<M, T> {
	fn asc(self) -> OrderingField<M> {
		OrderingField::new(self.path().to_vec(), OrderDirection::Asc)
	}

	fn desc(self) -> OrderingField<M> {
		OrderingField::new(self.path().to_vec(), OrderDirection::Desc)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_orm::Model;

	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestPost {
		id: i64,
		title: String,
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

	#[test]
	fn test_field_asc_extension() {
		let field = Field::<TestPost, String>::new(vec!["title"]);
		let order = field.asc();

		assert_eq!(order.field_path(), &["title"]);
		assert_eq!(order.direction(), OrderDirection::Asc);
	}

	#[test]
	fn test_field_desc_extension() {
		let field = Field::<TestPost, String>::new(vec!["created_at"]);
		let order = field.desc();

		assert_eq!(order.field_path(), &["created_at"]);
		assert_eq!(order.direction(), OrderDirection::Desc);
	}

	#[test]
	fn test_nested_field_ordering() {
		let field = Field::<TestPost, String>::new(vec!["author", "username"]);
		let order = field.asc();

		assert_eq!(order.field_path(), &["author", "username"]);
		assert_eq!(order.to_sql(), "author.username ASC");
	}
}
