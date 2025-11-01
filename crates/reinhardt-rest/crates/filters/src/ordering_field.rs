//! Type-safe ordering field definition
//!
//! Provides compile-time safe field ordering using reinhardt-orm's Field system.
//! Similar to Field/Lookup pattern in field_lookup module.

use reinhardt_orm::Model;
use std::marker::PhantomData;

/// Ordering direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
	/// Ascending order (ASC)
	Asc,
	/// Descending order (DESC)
	Desc,
}

/// Type-safe ordering field
///
/// Represents a field with its ordering direction. Can be created from any Field<M, T>
/// using the `.asc()` or `.desc()` methods on Field.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_filters::OrderingField;
/// use reinhardt_orm::Field;
///
// Create ascending ordering - call .asc() on Field
/// let field = Field::<Post, String>::new(vec!["title"]);
/// let asc_order = field.asc();
///
// Create descending ordering - call .desc() on Field
/// let field = Field::<Post, String>::new(vec!["created_at"]);
/// let desc_order = field.desc();
/// ```
pub struct OrderingField<M: Model> {
	pub(crate) field_path: Vec<&'static str>,
	pub(crate) direction: OrderDirection,
	pub(crate) _phantom: PhantomData<M>,
}

impl<M: Model> OrderingField<M> {
	/// Create a new ordering field from path and direction
	pub(crate) fn new(field_path: Vec<&'static str>, direction: OrderDirection) -> Self {
		Self {
			field_path,
			direction,
			_phantom: PhantomData,
		}
	}
	/// Get the field path
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_filters::OrderingField;
	/// use reinhardt_orm::Field;
	///
	/// let field = Field::<Post, String>::new(vec!["title"]);
	/// let order = field.asc();
	/// assert_eq!(order.field_path(), &["title"]);
	/// ```
	pub fn field_path(&self) -> &[&'static str] {
		&self.field_path
	}
	/// Get the ordering direction
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_filters::{OrderingField, OrderDirection};
	/// use reinhardt_orm::Field;
	///
	/// let field = Field::<Post, String>::new(vec!["created_at"]);
	/// let order = field.desc();
	/// assert_eq!(order.direction(), OrderDirection::Desc);
	/// ```
	pub fn direction(&self) -> OrderDirection {
		self.direction
	}

	/// Convert to SQL ORDER BY clause fragment
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let order = OrderingField::asc(Post::title());
	/// assert_eq!(order.to_sql(), "title ASC");
	///
	/// let order = OrderingField::desc(Post::created_at());
	/// assert_eq!(order.to_sql(), "created_at DESC");
	/// ```
	pub fn to_sql(&self) -> String {
		let field_name = self.field_path.join(".");
		let direction_str = match self.direction {
			OrderDirection::Asc => "ASC",
			OrderDirection::Desc => "DESC",
		};
		format!("{} {}", field_name, direction_str)
	}
}

impl<M: Model> Clone for OrderingField<M> {
	fn clone(&self) -> Self {
		Self {
			field_path: self.field_path.clone(),
			direction: self.direction,
			_phantom: PhantomData,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field_extensions::FieldOrderingExt;
	use reinhardt_orm::{Field, Model};

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
	fn test_asc_ordering() {
		let field = Field::<TestPost, String>::new(vec!["title"]);
		let order = field.asc();

		assert_eq!(order.field_path(), &["title"]);
		assert_eq!(order.direction(), OrderDirection::Asc);
		assert_eq!(order.to_sql(), "title ASC");
	}

	#[test]
	fn test_desc_ordering() {
		let field = Field::<TestPost, String>::new(vec!["created_at"]);
		let order = field.desc();

		assert_eq!(order.field_path(), &["created_at"]);
		assert_eq!(order.direction(), OrderDirection::Desc);
		assert_eq!(order.to_sql(), "created_at DESC");
	}
}
