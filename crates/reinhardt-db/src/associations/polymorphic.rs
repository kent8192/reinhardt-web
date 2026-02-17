//! Polymorphic associations
//!
//! This module provides support for polymorphic associations, allowing a model
//! to belong to multiple different model types through a single association.
//! This is similar to Rails' polymorphic associations and Django's GenericForeignKey.

use std::marker::PhantomData;

use super::foreign_key::CascadeAction;

/// Polymorphic association field
///
/// Represents a polymorphic relationship where the foreign key can point to
/// multiple different model types. This is achieved by storing both the ID
/// of the related object and a type discriminator.
///
/// # Type Parameters
///
/// * `K` - The type of the foreign key field (usually i64)
///
/// # Examples
///
/// ```
/// use reinhardt_db::associations::PolymorphicAssociation;
///
/// #[derive(Clone)]
/// struct Comment {
///     id: i64,
///     content: String,
///     commentable_id: i64,
///     commentable_type: String,
/// }
///
/// // A comment can belong to either a Post or a Video
/// let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("commentable")
///     .id_field("commentable_id")
///     .type_field("commentable_type");
/// ```
#[derive(Debug, Clone)]
pub struct PolymorphicAssociation<K> {
	/// The base name of the association (e.g., "commentable")
	pub association_name: String,
	/// The name of the ID field (e.g., "commentable_id")
	pub id_field: String,
	/// The name of the type discriminator field (e.g., "commentable_type")
	pub type_field: String,
	/// Action to take when referenced object is deleted
	pub on_delete: CascadeAction,
	/// Whether the foreign key can be null
	pub null: bool,
	/// Database index creation for the ID field
	pub db_index: bool,
	/// Phantom data for type parameter
	_phantom: PhantomData<K>,
}

impl<K> PolymorphicAssociation<K> {
	/// Create a new polymorphic association
	///
	/// # Arguments
	///
	/// * `association_name` - The base name of the association
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicAssociation;
	///
	/// let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("commentable");
	/// assert_eq!(rel.association_name(), "commentable");
	/// assert_eq!(rel.get_id_field(), "commentable_id");
	/// assert_eq!(rel.get_type_field(), "commentable_type");
	/// ```
	pub fn new(association_name: impl Into<String>) -> Self {
		let name = association_name.into();
		Self {
			id_field: format!("{}_id", name),
			type_field: format!("{}_type", name),
			association_name: name,
			on_delete: CascadeAction::default(),
			null: false,
			db_index: true,
			_phantom: PhantomData,
		}
	}

	/// Set the ID field name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicAssociation;
	///
	/// let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("taggable")
	///     .id_field("object_id");
	/// assert_eq!(rel.get_id_field(), "object_id");
	/// ```
	pub fn id_field(mut self, field_name: impl Into<String>) -> Self {
		self.id_field = field_name.into();
		self
	}

	/// Set the type discriminator field name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicAssociation;
	///
	/// let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("taggable")
	///     .type_field("content_type");
	/// assert_eq!(rel.get_type_field(), "content_type");
	/// ```
	pub fn type_field(mut self, field_name: impl Into<String>) -> Self {
		self.type_field = field_name.into();
		self
	}

	/// Set the on_delete cascade action
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::{PolymorphicAssociation, CascadeAction};
	///
	/// let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("commentable")
	///     .on_delete(CascadeAction::Cascade);
	/// assert_eq!(rel.get_on_delete(), CascadeAction::Cascade);
	/// ```
	pub fn on_delete(mut self, action: CascadeAction) -> Self {
		self.on_delete = action;
		self
	}

	/// Set whether the association can be null
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicAssociation;
	///
	/// let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("commentable")
	///     .null(true);
	/// assert!(rel.is_null());
	/// ```
	pub fn null(mut self, null: bool) -> Self {
		self.null = null;
		self
	}

	/// Set whether to create database index
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicAssociation;
	///
	/// let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("commentable")
	///     .db_index(false);
	/// assert!(!rel.has_db_index());
	/// ```
	pub fn db_index(mut self, db_index: bool) -> Self {
		self.db_index = db_index;
		self
	}

	/// Get the association name
	pub fn association_name(&self) -> &str {
		&self.association_name
	}

	/// Get the ID field name
	pub fn get_id_field(&self) -> &str {
		&self.id_field
	}

	/// Get the type field name
	pub fn get_type_field(&self) -> &str {
		&self.type_field
	}

	/// Get the on_delete action
	pub fn get_on_delete(&self) -> CascadeAction {
		self.on_delete
	}

	/// Check if null is allowed
	pub fn is_null(&self) -> bool {
		self.null
	}

	/// Check if database index should be created
	pub fn has_db_index(&self) -> bool {
		self.db_index
	}
}

impl<K> Default for PolymorphicAssociation<K> {
	fn default() -> Self {
		Self::new("polymorphic")
	}
}

/// Polymorphic many-to-many association
///
/// Represents a many-to-many relationship where the target can be multiple
/// different model types. This uses a junction table with a polymorphic foreign key.
///
/// # Type Parameters
///
/// * `K` - The type of the foreign key field (usually i64)
///
/// # Examples
///
/// ```
/// use reinhardt_db::associations::PolymorphicManyToMany;
///
/// #[derive(Clone)]
/// struct Tag {
///     id: i64,
///     name: String,
/// }
///
/// // Tags can be applied to Posts, Videos, or any other content type
/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
///     .through("taggings")
///     .source_field("tag_id")
///     .target_id_field("taggable_id")
///     .target_type_field("taggable_type");
/// ```
#[derive(Debug, Clone)]
pub struct PolymorphicManyToMany<K> {
	/// The base name of the association
	pub association_name: String,
	/// The name of the junction/through table
	pub through: Option<String>,
	/// The name of the source foreign key field in the junction table
	pub source_field: String,
	/// The name of the target ID field in the junction table
	pub target_id_field: String,
	/// The name of the target type discriminator field in the junction table
	pub target_type_field: String,
	/// Action to take when source object is deleted
	pub on_delete: CascadeAction,
	/// Whether to use lazy loading by default
	pub lazy: bool,
	/// Database constraint name prefix
	pub db_constraint_prefix: Option<String>,
	/// Phantom data for type parameter
	_phantom: PhantomData<K>,
}

impl<K> PolymorphicManyToMany<K> {
	/// Create a new polymorphic many-to-many association
	///
	/// # Arguments
	///
	/// * `association_name` - The base name of the association
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicManyToMany;
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable");
	/// assert_eq!(rel.association_name(), "taggable");
	/// ```
	pub fn new(association_name: impl Into<String>) -> Self {
		let name = association_name.into();
		Self {
			association_name: name.clone(),
			through: None,
			source_field: String::new(),
			target_id_field: format!("{}_id", name),
			target_type_field: format!("{}_type", name),
			on_delete: CascadeAction::Cascade,
			lazy: true,
			db_constraint_prefix: None,
			_phantom: PhantomData,
		}
	}

	/// Set the junction/through table name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicManyToMany;
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
	///     .through("taggings");
	/// assert_eq!(rel.get_through(), Some("taggings"));
	/// ```
	pub fn through(mut self, table_name: impl Into<String>) -> Self {
		self.through = Some(table_name.into());
		self
	}

	/// Set the source foreign key field name in the junction table
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicManyToMany;
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
	///     .source_field("tag_id");
	/// assert_eq!(rel.get_source_field(), "tag_id");
	/// ```
	pub fn source_field(mut self, field_name: impl Into<String>) -> Self {
		self.source_field = field_name.into();
		self
	}

	/// Set the target ID field name in the junction table
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicManyToMany;
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
	///     .target_id_field("object_id");
	/// assert_eq!(rel.get_target_id_field(), "object_id");
	/// ```
	pub fn target_id_field(mut self, field_name: impl Into<String>) -> Self {
		self.target_id_field = field_name.into();
		self
	}

	/// Set the target type discriminator field name in the junction table
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicManyToMany;
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
	///     .target_type_field("content_type");
	/// assert_eq!(rel.get_target_type_field(), "content_type");
	/// ```
	pub fn target_type_field(mut self, field_name: impl Into<String>) -> Self {
		self.target_type_field = field_name.into();
		self
	}

	/// Set the on_delete cascade action
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::{PolymorphicManyToMany, CascadeAction};
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
	///     .on_delete(CascadeAction::Restrict);
	/// assert_eq!(rel.get_on_delete(), CascadeAction::Restrict);
	/// ```
	pub fn on_delete(mut self, action: CascadeAction) -> Self {
		self.on_delete = action;
		self
	}

	/// Set whether to use lazy loading
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicManyToMany;
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
	///     .lazy(false);
	/// assert!(!rel.is_lazy());
	/// ```
	pub fn lazy(mut self, lazy: bool) -> Self {
		self.lazy = lazy;
		self
	}

	/// Set the database constraint name prefix
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::associations::PolymorphicManyToMany;
	///
	/// let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
	///     .db_constraint_prefix("poly_taggings");
	/// assert_eq!(rel.get_db_constraint_prefix(), Some("poly_taggings"));
	/// ```
	pub fn db_constraint_prefix(mut self, prefix: impl Into<String>) -> Self {
		self.db_constraint_prefix = Some(prefix.into());
		self
	}

	/// Get the association name
	pub fn association_name(&self) -> &str {
		&self.association_name
	}

	/// Get the through table name
	pub fn get_through(&self) -> Option<&str> {
		self.through.as_deref()
	}

	/// Get the source field name
	pub fn get_source_field(&self) -> &str {
		&self.source_field
	}

	/// Get the target ID field name
	pub fn get_target_id_field(&self) -> &str {
		&self.target_id_field
	}

	/// Get the target type field name
	pub fn get_target_type_field(&self) -> &str {
		&self.target_type_field
	}

	/// Get the on_delete action
	pub fn get_on_delete(&self) -> CascadeAction {
		self.on_delete
	}

	/// Check if lazy loading is enabled
	pub fn is_lazy(&self) -> bool {
		self.lazy
	}

	/// Get the database constraint prefix
	pub fn get_db_constraint_prefix(&self) -> Option<&str> {
		self.db_constraint_prefix.as_deref()
	}
}

impl<K> Default for PolymorphicManyToMany<K> {
	fn default() -> Self {
		Self::new("polymorphic")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_polymorphic_association_creation() {
		let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("commentable");
		assert_eq!(rel.association_name(), "commentable");
		assert_eq!(rel.get_id_field(), "commentable_id");
		assert_eq!(rel.get_type_field(), "commentable_type");
		assert_eq!(rel.get_on_delete(), CascadeAction::NoAction);
		assert!(!rel.is_null());
		assert!(rel.has_db_index());
	}

	#[rstest]
	fn test_polymorphic_association_builder() {
		let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("taggable")
			.id_field("object_id")
			.type_field("content_type")
			.on_delete(CascadeAction::Cascade)
			.null(true)
			.db_index(false);

		assert_eq!(rel.association_name(), "taggable");
		assert_eq!(rel.get_id_field(), "object_id");
		assert_eq!(rel.get_type_field(), "content_type");
		assert_eq!(rel.get_on_delete(), CascadeAction::Cascade);
		assert!(rel.is_null());
		assert!(!rel.has_db_index());
	}

	#[rstest]
	fn test_polymorphic_association_default_field_names() {
		let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("imageable");
		assert_eq!(rel.get_id_field(), "imageable_id");
		assert_eq!(rel.get_type_field(), "imageable_type");
	}

	#[rstest]
	fn test_polymorphic_many_to_many_creation() {
		let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable");
		assert_eq!(rel.association_name(), "taggable");
		assert_eq!(rel.get_through(), None);
		assert_eq!(rel.get_source_field(), "");
		assert_eq!(rel.get_target_id_field(), "taggable_id");
		assert_eq!(rel.get_target_type_field(), "taggable_type");
		assert_eq!(rel.get_on_delete(), CascadeAction::Cascade);
		assert!(rel.is_lazy());
	}

	#[rstest]
	fn test_polymorphic_many_to_many_builder() {
		let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("taggable")
			.through("taggings")
			.source_field("tag_id")
			.target_id_field("object_id")
			.target_type_field("content_type")
			.on_delete(CascadeAction::Restrict)
			.lazy(false)
			.db_constraint_prefix("poly_tag");

		assert_eq!(rel.association_name(), "taggable");
		assert_eq!(rel.get_through(), Some("taggings"));
		assert_eq!(rel.get_source_field(), "tag_id");
		assert_eq!(rel.get_target_id_field(), "object_id");
		assert_eq!(rel.get_target_type_field(), "content_type");
		assert_eq!(rel.get_on_delete(), CascadeAction::Restrict);
		assert!(!rel.is_lazy());
		assert_eq!(rel.get_db_constraint_prefix(), Some("poly_tag"));
	}

	#[rstest]
	fn test_polymorphic_many_to_many_default_field_names() {
		let rel: PolymorphicManyToMany<i64> = PolymorphicManyToMany::new("likeable");
		assert_eq!(rel.get_target_id_field(), "likeable_id");
		assert_eq!(rel.get_target_type_field(), "likeable_type");
	}

	#[rstest]
	fn test_cascade_actions_polymorphic() {
		let actions = vec![
			CascadeAction::NoAction,
			CascadeAction::Restrict,
			CascadeAction::SetNull,
			CascadeAction::SetDefault,
			CascadeAction::Cascade,
		];

		for action in actions {
			let rel: PolymorphicAssociation<i64> =
				PolymorphicAssociation::new("commentable").on_delete(action);
			assert_eq!(rel.get_on_delete(), action);
		}
	}

	#[rstest]
	fn test_null_configuration_polymorphic() {
		let rel1: PolymorphicAssociation<i64> =
			PolymorphicAssociation::new("commentable").null(true);
		assert!(rel1.is_null());

		let rel2: PolymorphicAssociation<i64> =
			PolymorphicAssociation::new("commentable").null(false);
		assert!(!rel2.is_null());
	}

	#[rstest]
	fn test_db_index_configuration_polymorphic() {
		let rel1: PolymorphicAssociation<i64> =
			PolymorphicAssociation::new("commentable").db_index(true);
		assert!(rel1.has_db_index());

		let rel2: PolymorphicAssociation<i64> =
			PolymorphicAssociation::new("commentable").db_index(false);
		assert!(!rel2.has_db_index());
	}
}
