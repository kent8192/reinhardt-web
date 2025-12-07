//! Marker types for relationship field annotations.
//!
//! These types are used as field types to indicate relationship definitions
//! when using the `#[rel]` attribute macro.
//!
//! # Example
//!
//! ```ignore
//! use reinhardt::prelude::*;
//! use reinhardt::db::associations::ManyToManyField;
//!
//! #[model(app_label = "users")]
//! pub struct User {
//!     #[field(primary_key = true)]
//!     pub id: Uuid,
//!
//!     // User -> User relationship (self-referential)
//!     #[rel(many_to_many, related_name = "followers")]
//!     pub following: ManyToManyField<User, User>,
//! }
//!
//! #[model(app_label = "users")]
//! pub struct Group {
//!     #[field(primary_key = true)]
//!     pub id: Uuid,
//!
//!     // Group -> User relationship
//!     #[rel(many_to_many, related_name = "groups")]
//!     pub members: ManyToManyField<Group, User>,
//! }
//! ```

use std::marker::PhantomData;

/// Marker type for ManyToMany relationship fields.
///
/// This type is used as the field type for `#[rel(many_to_many, ...)]` attributes.
/// It indicates that the field represents a many-to-many relationship.
///
/// The type parameters:
/// - `Source`: The source model type (the model containing this field)
/// - `Target`: The target model type (the related model)
/// - `S`: Relationship metadata type (defaults to `()`)
///
/// The `S` parameter allows storing relationship-specific metadata at the type level,
/// such as through-table configuration, ordering, or custom relationship behavior.
///
/// The intermediate table is automatically generated based on the source and target types:
/// - Table name: `{app_label}_{source_table}_{field_name}`
/// - Source FK: `{source_table}_id`
/// - Target FK: `{target_table}_id`
///
/// # Example
///
/// ```ignore
/// use reinhardt::prelude::*;
/// use reinhardt::db::associations::ManyToManyField;
///
/// #[model(app_label = "auth", table_name = "users")]
/// pub struct User {
///     #[field(primary_key = true)]
///     pub id: Uuid,
///
///     // User -> Group relationship
///     #[rel(many_to_many, related_name = "members")]
///     pub groups: ManyToManyField<User, Group>,
///
///     // Self-referential relationship (following)
///     #[rel(many_to_many, related_name = "followers")]
///     pub following: ManyToManyField<User, User>,
/// }
///
/// // With custom metadata type:
/// struct OrderedRelation;
///
/// #[model(app_label = "project", table_name = "tasks")]
/// pub struct Task {
///     #[rel(many_to_many)]
///     pub tags: ManyToManyField<Task, Tag, OrderedRelation>,
/// }
///
/// // Usage with ManyToManyAccessor:
/// use reinhardt_orm::ManyToManyAccessor;
///
/// let user = User::find_by_id(&db, user_id).await?;
/// let accessor = ManyToManyAccessor::new(&user, "groups", ());
///
/// // Add relationship
/// accessor.add(&group).await?;
///
/// // Get all related
/// let groups = accessor.all().await?;
///
/// // Remove relationship
/// accessor.remove(&group).await?;
///
/// // Clear all
/// accessor.clear().await?;
/// ```
/// Marker type for ManyToMany relationship.
///
/// `Default` is implemented manually to avoid requiring `Source` and `Target` to implement `Default`.
/// This is because the derive macro would add `Source: Default, Target: Default` bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(bound = "")]
pub struct ManyToManyField<Source, Target, S = ()>(PhantomData<(Source, Target, S)>);

impl<Source, Target, S> Default for ManyToManyField<Source, Target, S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Source, Target, S> ManyToManyField<Source, Target, S> {
	/// Creates a new ManyToManyField marker.
	///
	/// This constructor hides the internal `PhantomData` implementation,
	/// providing a clean API for users without exposing implementation details.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_associations::ManyToManyField;
	///
	/// struct User;
	/// struct Tag;
	///
	/// // User -> Tag relationship with default metadata type ()
	/// let field: ManyToManyField<User, Tag> = ManyToManyField::new();
	///
	/// // Self-referential relationship
	/// let following: ManyToManyField<User, User> = ManyToManyField::new();
	///
	/// // Custom metadata type
	/// struct CustomMetadata;
	/// let field_with_metadata: ManyToManyField<User, Tag, CustomMetadata> = ManyToManyField::new();
	/// ```
	#[inline]
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

/// Marker type for ForeignKey relationship fields.
///
/// This type is used as the field type for `#[rel(foreign_key, ...)]` attributes.
/// It represents a many-to-one relationship where multiple instances of the
/// containing model can reference a single instance of `T`.
///
/// The type parameter:
/// - `T`: The target model type (the related model)
///
/// When using this type, the macro automatically:
/// - Generates a `{field_name}_id` column for the foreign key value
/// - Infers the target model from `T` (no need for `to = Model` attribute)
/// - Generates lazy load accessor method `{field_name}(&self, conn)` -> `Result<Option<T>>`
///
/// # Example
///
/// ```ignore
/// use reinhardt::db::associations::ForeignKeyField;
/// use reinhardt::model;
///
/// #[model(app_label = "blog", table_name = "posts")]
/// pub struct Post {
///     #[field(primary_key = true)]
///     pub id: i64,
///
///     // Generates `author_id` column automatically
///     #[rel(foreign_key, related_name = "posts")]
///     pub author: ForeignKeyField<User>,
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(bound = "")]
pub struct ForeignKeyField<T>(PhantomData<T>);

impl<T> Default for ForeignKeyField<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T> ForeignKeyField<T> {
	/// Creates a new ForeignKeyField marker.
	///
	/// This constructor hides the internal `PhantomData` implementation.
	#[inline]
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

/// Marker type for OneToOne relationship fields.
///
/// This type is used as the field type for `#[rel(one_to_one, ...)]` attributes.
/// It represents a one-to-one relationship where each instance of the
/// containing model references exactly one instance of `T`.
///
/// The type parameter:
/// - `T`: The target model type (the related model)
///
/// When using this type, the macro automatically:
/// - Generates a `{field_name}_id` column for the foreign key value
/// - Infers the target model from `T` (no need for `to = Model` attribute)
/// - Generates lazy load accessor method `{field_name}(&self, conn)` -> `Result<Option<T>>`
/// - Adds UNIQUE constraint to the generated column
///
/// # Example
///
/// ```ignore
/// use reinhardt::db::associations::OneToOneField;
/// use reinhardt::model;
///
/// #[model(app_label = "auth", table_name = "profiles")]
/// pub struct Profile {
///     #[field(primary_key = true)]
///     pub id: i64,
///
///     // Generates `user_id` column with UNIQUE constraint
///     #[rel(one_to_one, related_name = "profile")]
///     pub user: OneToOneField<User>,
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(bound = "")]
pub struct OneToOneField<T>(PhantomData<T>);

impl<T> Default for OneToOneField<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T> OneToOneField<T> {
	/// Creates a new OneToOneField marker.
	///
	/// This constructor hides the internal `PhantomData` implementation.
	#[inline]
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

/// Marker type for OneToMany relationship fields (reverse of ForeignKey).
///
/// This type is used as the field type for `#[rel(one_to_many, ...)]` attributes.
/// It represents the reverse side of a ForeignKey relationship.
///
/// The type parameters:
/// - `T`: The related model type
/// - `S`: Relationship metadata type (defaults to `()`)
///
/// # Example
///
/// ```ignore
/// // On User model
/// #[rel(one_to_many, foreign_key = "author_id")]
/// pub posts: OneToManyField<Post>,
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct OneToManyField<T, S = ()>(PhantomData<(T, S)>);

impl<T, S> OneToManyField<T, S> {
	/// Creates a new OneToManyField marker.
	///
	/// This constructor hides the internal `PhantomData` implementation.
	#[inline]
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

/// Marker type for PolymorphicManyToMany relationship fields.
///
/// This type is used as the field type for `#[rel(polymorphic_many_to_many, ...)]` attributes.
/// It indicates a polymorphic many-to-many relationship.
///
/// The type parameters:
/// - `K`: The key type (usually `i64` or `Uuid`)
/// - `S`: Relationship metadata type (defaults to `()`)
///
/// # Example
///
/// ```ignore
/// #[rel(polymorphic_many_to_many, name = "taggable")]
/// pub tags: PolymorphicManyToManyField<Uuid>,
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PolymorphicManyToManyField<K, S = ()>(PhantomData<(K, S)>);

impl<K, S> PolymorphicManyToManyField<K, S> {
	/// Creates a new PolymorphicManyToManyField marker.
	///
	/// This constructor hides the internal `PhantomData` implementation.
	#[inline]
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_many_to_many_field_creation() {
		struct User;
		struct Group;
		// User -> Group relationship
		let _field: ManyToManyField<User, Group> = ManyToManyField::new();
	}

	#[test]
	fn test_many_to_many_field_self_referential() {
		struct User;
		// User -> User (self-referential, e.g., following)
		let _field: ManyToManyField<User, User> = ManyToManyField::new();
	}

	#[test]
	fn test_one_to_many_field_creation() {
		struct Post;
		let _field: OneToManyField<Post> = OneToManyField::new();
	}

	#[test]
	fn test_polymorphic_many_to_many_field_creation() {
		let _field: PolymorphicManyToManyField<i64> = PolymorphicManyToManyField::new();
	}

	#[test]
	fn test_default_impl() {
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
		struct Article;
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
		struct Tag;
		// Article -> Tag relationship
		let field1: ManyToManyField<Article, Tag> = ManyToManyField::default();
		let field2: ManyToManyField<Article, Tag> = ManyToManyField::new();
		assert_eq!(field1, field2);
	}

	#[test]
	fn test_many_to_many_field_with_metadata() {
		struct User;
		struct Tag;
		struct OrderedRelation;

		// Default metadata type ()
		let _field1: ManyToManyField<User, Tag> = ManyToManyField::new();

		// Custom metadata type
		let _field2: ManyToManyField<User, Tag, OrderedRelation> = ManyToManyField::new();
	}

	#[test]
	fn test_foreign_key_field_creation() {
		struct User;
		let _field: ForeignKeyField<User> = ForeignKeyField::new();
	}

	#[test]
	fn test_foreign_key_field_default() {
		#[derive(Debug, PartialEq)]
		struct User;
		let field1: ForeignKeyField<User> = ForeignKeyField::default();
		let field2: ForeignKeyField<User> = ForeignKeyField::new();
		assert_eq!(field1, field2);
	}

	#[test]
	fn test_one_to_one_field_creation() {
		struct User;
		let _field: OneToOneField<User> = OneToOneField::new();
	}

	#[test]
	fn test_one_to_one_field_default() {
		#[derive(Debug, PartialEq)]
		struct User;
		let field1: OneToOneField<User> = OneToOneField::default();
		let field2: OneToOneField<User> = OneToOneField::new();
		assert_eq!(field1, field2);
	}
}
