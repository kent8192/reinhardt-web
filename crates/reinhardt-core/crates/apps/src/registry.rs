//! Global model registry
//!
//! This module provides a global registry for models, allowing models to be
//! discovered and registered at compile time using the `linkme` crate.
//!
//! # Examples
//!
//! ```rust
//! use reinhardt_apps::registry::{ModelMetadata, get_registered_models};
//!
//! // Register a model (typically done via derive macro)
//! #[linkme::distributed_slice(reinhardt_apps::registry::MODELS)]
//! static MY_MODEL: ModelMetadata = ModelMetadata {
//!     app_label: "myapp",
//!     model_name: "User",
//!     table_name: "users",
//! };
//!
//! // Access registered models
//! let models = get_registered_models();
//! // Note: In doc tests, the model may not be visible due to linkme limitations
//! ```

use linkme::distributed_slice;
use std::sync::RwLock;

/// Metadata for a registered model
///
/// This structure contains essential information about a model that has been
/// registered in the global model registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMetadata {
	/// The label of the application this model belongs to
	pub app_label: &'static str,

	/// The name of the model (e.g., "User", "Post")
	pub model_name: &'static str,

	/// The database table name for this model
	pub table_name: &'static str,
}

impl ModelMetadata {
	/// Create a new model metadata instance
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::registry::ModelMetadata;
	///
	/// let metadata = ModelMetadata::new("myapp", "User", "users");
	/// assert_eq!(metadata.app_label, "myapp");
	/// assert_eq!(metadata.model_name, "User");
	/// assert_eq!(metadata.table_name, "users");
	/// ```
	pub const fn new(
		app_label: &'static str,
		model_name: &'static str,
		table_name: &'static str,
	) -> Self {
		Self {
			app_label,
			model_name,
			table_name,
		}
	}

	/// Get the fully qualified model name (app_label.model_name)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::registry::ModelMetadata;
	///
	/// let metadata = ModelMetadata::new("myapp", "User", "users");
	/// assert_eq!(metadata.qualified_name(), "myapp.User");
	/// ```
	pub fn qualified_name(&self) -> String {
		format!("{}.{}", self.app_label, self.model_name)
	}
}

/// Global distributed slice for model registration
///
/// This is the global registry where all models are collected at link time.
/// Models can be registered by adding items to this slice using the `#[distributed_slice]`
/// attribute from the `linkme` crate.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::{MODELS, ModelMetadata};
///
/// #[linkme::distributed_slice(MODELS)]
/// static MY_MODEL: ModelMetadata = ModelMetadata {
///     app_label: "myapp",
///     model_name: "User",
///     table_name: "users",
/// };
/// ```
#[distributed_slice]
pub static MODELS: [ModelMetadata];

/// Cache for model lookups by app label
///
/// This cache is lazily populated the first time models are queried by app label.
static MODEL_CACHE: RwLock<
	Option<std::collections::HashMap<&'static str, Vec<&'static ModelMetadata>>>,
> = RwLock::new(None);

/// Get all registered models
///
/// This function returns a slice of all models that have been registered
/// in the global model registry.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::get_registered_models;
///
/// let models = get_registered_models();
/// println!("Found {} registered models", models.len());
/// ```
pub fn get_registered_models() -> &'static [ModelMetadata] {
	&MODELS
}

/// Get models for a specific application
///
/// This function returns all models that belong to the specified application label.
/// Results are cached for performance on subsequent calls.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::get_models_for_app;
///
/// let auth_models = get_models_for_app("auth");
/// for model in auth_models {
///     println!("Model: {}", model.model_name);
/// }
/// ```
pub fn get_models_for_app(app_label: &str) -> Vec<&'static ModelMetadata> {
	// Check if cache is initialized
	{
		let cache = MODEL_CACHE.read().unwrap();
		if let Some(ref cache_map) = *cache
			&& let Some(models) = cache_map.get(app_label)
		{
			return models.clone();
		}
	}

	// Initialize cache if needed
	{
		let mut cache = MODEL_CACHE.write().unwrap();
		if cache.is_none() {
			let mut cache_map = std::collections::HashMap::new();
			for model in MODELS.iter() {
				cache_map
					.entry(model.app_label)
					.or_insert_with(Vec::new)
					.push(model);
			}
			*cache = Some(cache_map);
		}
	}

	// Retrieve from cache
	let cache = MODEL_CACHE.read().unwrap();
	cache
		.as_ref()
		.unwrap()
		.get(app_label)
		.cloned()
		.unwrap_or_default()
}

/// Find a model by its qualified name (app_label.model_name)
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::find_model;
///
/// if let Some(model) = find_model("myapp.User") {
///     println!("Found model: {}", model.model_name);
/// } else {
///     println!("Model not found");
/// }
/// ```
pub fn find_model(qualified_name: &str) -> Option<&'static ModelMetadata> {
	let parts: Vec<&str> = qualified_name.split('.').collect();
	if parts.len() != 2 {
		return None;
	}

	let (app_label, model_name) = (parts[0], parts[1]);
	MODELS
		.iter()
		.find(|m| m.app_label == app_label && m.model_name == model_name)
}

/// Clear the model cache (primarily for testing)
///
/// This function clears the internal cache used for model lookups.
/// It should primarily be used in test scenarios.
pub fn clear_model_cache() {
	let mut cache = MODEL_CACHE.write().unwrap_or_else(|e| e.into_inner());
	*cache = None;
}

/// Metadata for a forward relationship
///
/// This structure contains information about a relationship field defined in a model.
/// It is populated at compile time via derive macros and stored in the global RELATIONSHIPS slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipMetadata {
	/// The model that owns this relationship (e.g., "blog.Post")
	pub from_model: &'static str,

	/// The target model of this relationship (e.g., "auth.User")
	pub to_model: &'static str,

	/// The type of relationship
	pub relationship_type: RelationshipType,

	/// The field name in the source model (e.g., "author", "tags")
	pub field_name: &'static str,

	/// The related name for reverse access (e.g., "posts", "authored_posts")
	/// If None, will be auto-generated as "{model_name}_set"
	pub related_name: Option<&'static str>,

	/// The database column name (for ForeignKey fields)
	/// None for ManyToMany relationships
	pub db_column: Option<&'static str>,

	/// The through table name (for ManyToMany relationships)
	/// None for ForeignKey and OneToOne relationships
	pub through_table: Option<&'static str>,
}

/// Type of relationship between models
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipType {
	/// Foreign key relationship (Many-to-One from the perspective of the field)
	ForeignKey,
	/// Many-to-Many relationship
	ManyToMany,
	/// One-to-One relationship
	OneToOne,
}

impl RelationshipMetadata {
	/// Create a new relationship metadata instance
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::registry::{RelationshipMetadata, RelationshipType};
	///
	/// let relationship = RelationshipMetadata::new(
	///     "blog.Post",
	///     "auth.User",
	///     RelationshipType::ForeignKey,
	///     "author",
	///     Some("posts"),
	///     Some("author_id"),
	///     None,
	/// );
	/// assert_eq!(relationship.field_name, "author");
	/// ```
	#[allow(clippy::too_many_arguments)]
	pub const fn new(
		from_model: &'static str,
		to_model: &'static str,
		relationship_type: RelationshipType,
		field_name: &'static str,
		related_name: Option<&'static str>,
		db_column: Option<&'static str>,
		through_table: Option<&'static str>,
	) -> Self {
		Self {
			from_model,
			to_model,
			relationship_type,
			field_name,
			related_name,
			db_column,
			through_table,
		}
	}

	/// Get the source model name without app label
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::registry::{RelationshipMetadata, RelationshipType};
	///
	/// let relationship = RelationshipMetadata::new(
	///     "blog.Post",
	///     "auth.User",
	///     RelationshipType::ForeignKey,
	///     "author",
	///     None,
	///     None,
	///     None,
	/// );
	/// assert_eq!(relationship.from_model_name(), "Post");
	/// ```
	pub fn from_model_name(&self) -> &str {
		self.from_model
			.split('.')
			.next_back()
			.unwrap_or(self.from_model)
	}

	/// Get the target model name without app label
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_apps::registry::{RelationshipMetadata, RelationshipType};
	///
	/// let relationship = RelationshipMetadata::new(
	///     "blog.Post",
	///     "auth.User",
	///     RelationshipType::ForeignKey,
	///     "author",
	///     None,
	///     None,
	///     None,
	/// );
	/// assert_eq!(relationship.to_model_name(), "User");
	/// ```
	pub fn to_model_name(&self) -> &str {
		self.to_model
			.split('.')
			.next_back()
			.unwrap_or(self.to_model)
	}
}

/// Global distributed slice for relationship registration
///
/// This is the global registry where all relationships are collected at compile time.
/// Relationships can be registered by adding items to this slice using the `#[distributed_slice]`
/// attribute from the `linkme` crate.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::{RELATIONSHIPS, RelationshipMetadata, RelationshipType};
///
/// #[linkme::distributed_slice(RELATIONSHIPS)]
/// static POST_AUTHOR: RelationshipMetadata = RelationshipMetadata {
///     from_model: "blog.Post",
///     to_model: "auth.User",
///     relationship_type: RelationshipType::ForeignKey,
///     field_name: "author",
///     related_name: Some("posts"),
///     db_column: Some("author_id"),
///     through_table: None,
/// };
/// ```
#[distributed_slice]
pub static RELATIONSHIPS: [RelationshipMetadata];

/// Cache for relationship lookups by model
static RELATIONSHIP_CACHE: RwLock<
	Option<std::collections::HashMap<&'static str, Vec<&'static RelationshipMetadata>>>,
> = RwLock::new(None);

/// Get all registered relationships
///
/// This function returns a slice of all relationships that have been registered
/// in the global relationship registry.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::get_registered_relationships;
///
/// let relationships = get_registered_relationships();
/// println!("Found {} registered relationships", relationships.len());
/// ```
pub fn get_registered_relationships() -> &'static [RelationshipMetadata] {
	&RELATIONSHIPS
}

pub fn get_relationships_for_model(model: &str) -> Vec<&'static RelationshipMetadata> {
	// Initialize cache if needed (with write lock)
	{
		let mut cache = RELATIONSHIP_CACHE.write().unwrap();
		if cache.is_none() {
			let mut cache_map = std::collections::HashMap::new();
			for rel in RELATIONSHIPS.iter() {
				cache_map
					.entry(rel.from_model)
					.or_insert_with(Vec::new)
					.push(rel);
			}
			*cache = Some(cache_map);
		}
	}

	// Retrieve from cache (with read lock)
	let cache = RELATIONSHIP_CACHE.read().unwrap();
	cache
		.as_ref()
		.expect("Cache should be initialized at this point")
		.get(model)
		.cloned()
		.unwrap_or_default()
}

/// Find relationships by target model
///
/// This function returns all relationships that point to the specified target model.
/// Useful for discovering reverse relationships.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::get_relationships_to_model;
///
/// let user_reverse_rels = get_relationships_to_model("auth.User");
/// for rel in user_reverse_rels {
///     println!("Reverse relationship from {}.{}", rel.from_model, rel.field_name);
/// }
/// ```
pub fn get_relationships_to_model(target_model: &str) -> Vec<&'static RelationshipMetadata> {
	RELATIONSHIPS
		.iter()
		.filter(|r| r.to_model == target_model)
		.collect()
}

/// Clear the relationship cache (primarily for testing)
///
/// This function clears the internal cache used for relationship lookups.
/// It should primarily be used in test scenarios.
pub fn clear_relationship_cache() {
	let mut cache = RELATIONSHIP_CACHE
		.write()
		.unwrap_or_else(|e| e.into_inner());
	*cache = None;
}

/// Metadata for a reverse relation
///
/// This structure represents a reverse relation that has been dynamically
/// registered for lazy loading or eager loading via select_related.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseRelationMetadata {
	/// The model this reverse relation belongs to (e.g., "User")
	pub on_model: &'static str,

	/// The accessor name (e.g., "posts" or "post_set")
	pub accessor_name: String,

	/// The related model (e.g., "Post")
	pub related_model: &'static str,

	/// The type of reverse relation
	pub relation_type: ReverseRelationType,

	/// The original field name in the related model
	pub through_field: &'static str,
}

/// Type of reverse relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverseRelationType {
	/// Reverse of OneToMany (collection)
	ReverseOneToMany,
	/// Reverse of ManyToMany (collection)
	ReverseManyToMany,
	/// Reverse of OneToOne (single object)
	ReverseOneToOne,
}

impl ReverseRelationMetadata {
	/// Create a new reverse relation metadata instance
	pub fn new(
		on_model: &'static str,
		accessor_name: String,
		related_model: &'static str,
		relation_type: ReverseRelationType,
		through_field: &'static str,
	) -> Self {
		Self {
			on_model,
			accessor_name,
			related_model,
			relation_type,
			through_field,
		}
	}
}

/// Global registry for reverse relations
///
/// This registry stores dynamically registered reverse relations.
/// Unlike the MODELS slice which is populated at compile time,
/// this registry is populated at runtime during model discovery.
static REVERSE_RELATIONS: RwLock<Vec<ReverseRelationMetadata>> = RwLock::new(Vec::new());

/// Register a reverse relation
///
/// This function adds a reverse relation to the global registry.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::{register_reverse_relation, ReverseRelationMetadata, ReverseRelationType};
///
/// let reverse_relation = ReverseRelationMetadata::new(
///     "User",
///     "posts".to_string(),
///     "Post",
///     ReverseRelationType::ReverseOneToMany,
///     "author",
/// );
/// register_reverse_relation(reverse_relation);
/// ```
pub fn register_reverse_relation(relation: ReverseRelationMetadata) {
	let mut relations = REVERSE_RELATIONS.write().unwrap();
	relations.push(relation);
}

/// Get all reverse relations for a specific model
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::get_reverse_relations_for_model;
///
/// let relations = get_reverse_relations_for_model("User");
/// for relation in relations {
///     println!("Reverse relation: {}", relation.accessor_name);
/// }
/// ```
pub fn get_reverse_relations_for_model(model_name: &str) -> Vec<ReverseRelationMetadata> {
	let relations = REVERSE_RELATIONS.read().unwrap();
	relations
		.iter()
		.filter(|r| r.on_model == model_name)
		.cloned()
		.collect()
}

/// Clear the reverse relations registry (primarily for testing)
///
/// This function clears all registered reverse relations.
/// It should primarily be used in test scenarios.
pub fn clear_reverse_relations() {
	let mut relations = REVERSE_RELATIONS.write().unwrap_or_else(|e| e.into_inner());
	relations.clear();
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_test::resource::{TeardownGuard, TestResource};
	use rstest::*;
	use std::collections::HashSet;

	// TeardownGuard for model cache cleanup
	struct ModelCacheGuard;

	impl TestResource for ModelCacheGuard {
		fn setup() -> Self {
			Self
		}

		fn teardown(&mut self) {
			clear_model_cache();
		}
	}

	#[fixture]
	fn model_cache() -> TeardownGuard<ModelCacheGuard> {
		clear_model_cache();
		TeardownGuard::new()
	}

	// TeardownGuard for relationship cache cleanup
	struct RelationshipCacheGuard;

	impl TestResource for RelationshipCacheGuard {
		fn setup() -> Self {
			Self
		}

		fn teardown(&mut self) {
			clear_relationship_cache();
		}
	}

	#[fixture]
	fn relationship_cache() -> TeardownGuard<RelationshipCacheGuard> {
		clear_relationship_cache();
		TeardownGuard::new()
	}

	// TeardownGuard for reverse relations cleanup
	struct ReverseRelationGuard;

	impl TestResource for ReverseRelationGuard {
		fn setup() -> Self {
			Self
		}

		fn teardown(&mut self) {
			clear_reverse_relations();
		}
	}

	#[fixture]
	fn reverse_relation() -> TeardownGuard<ReverseRelationGuard> {
		clear_reverse_relations();
		TeardownGuard::new()
	}

	// Test model registrations
	#[distributed_slice(MODELS)]
	static TEST_USER_MODEL: ModelMetadata = ModelMetadata {
		app_label: "auth",
		model_name: "User",
		table_name: "auth_users",
	};

	#[distributed_slice(MODELS)]
	static TEST_POST_MODEL: ModelMetadata = ModelMetadata {
		app_label: "blog",
		model_name: "Post",
		table_name: "blog_posts",
	};

	#[distributed_slice(MODELS)]
	static TEST_COMMENT_MODEL: ModelMetadata = ModelMetadata {
		app_label: "blog",
		model_name: "Comment",
		table_name: "blog_comments",
	};

	#[test]
	fn test_model_metadata_new() {
		let metadata = ModelMetadata::new("myapp", "MyModel", "my_table");
		assert_eq!(metadata.app_label, "myapp");
		assert_eq!(metadata.model_name, "MyModel");
		assert_eq!(metadata.table_name, "my_table");
	}

	#[test]
	fn test_qualified_name() {
		let metadata = ModelMetadata::new("auth", "User", "users");
		assert_eq!(metadata.qualified_name(), "auth.User");
	}

	#[test]
	fn test_find_model_invalid_format() {
		let model = find_model("InvalidFormat");
		assert!(model.is_none());

		let model = find_model("too.many.parts");
		assert!(model.is_none());
	}

	#[test]
	fn test_model_metadata_equality() {
		let meta1 = ModelMetadata::new("app", "Model", "table");
		let meta2 = ModelMetadata::new("app", "Model", "table");
		let meta3 = ModelMetadata::new("app", "Other", "table");

		assert_eq!(meta1, meta2);
		assert_ne!(meta1, meta3);
	}

	#[test]
	fn test_reverse_relation_metadata_new() {
		let relation = ReverseRelationMetadata::new(
			"User",
			"posts".to_string(),
			"Post",
			ReverseRelationType::ReverseOneToMany,
			"author",
		);

		assert_eq!(relation.on_model, "User");
		assert_eq!(relation.accessor_name, "posts");
		assert_eq!(relation.related_model, "Post");
		assert_eq!(
			relation.relation_type,
			ReverseRelationType::ReverseOneToMany
		);
		assert_eq!(relation.through_field, "author");
	}

	#[rstest]
	#[serial_test::serial(app_registry)]
	fn test_register_and_get_reverse_relations(
		_reverse_relation: TeardownGuard<ReverseRelationGuard>,
	) {
		let relation1 = ReverseRelationMetadata::new(
			"User",
			"posts".to_string(),
			"Post",
			ReverseRelationType::ReverseOneToMany,
			"author",
		);
		register_reverse_relation(relation1);

		let relation2 = ReverseRelationMetadata::new(
			"User",
			"comments".to_string(),
			"Comment",
			ReverseRelationType::ReverseOneToMany,
			"author",
		);
		register_reverse_relation(relation2);

		let relations = get_reverse_relations_for_model("User");
		assert_eq!(relations.len(), 2);

		let accessor_names: HashSet<String> =
			relations.iter().map(|r| r.accessor_name.clone()).collect();
		assert_eq!(
			accessor_names,
			HashSet::from(["posts".to_string(), "comments".to_string()])
		);
	}

	#[rstest]
	#[serial_test::serial(app_registry)]
	fn test_get_reverse_relations_for_nonexistent_model(
		_reverse_relation: TeardownGuard<ReverseRelationGuard>,
	) {
		let relations = get_reverse_relations_for_model("NonExistent");
		assert_eq!(relations.len(), 0);
	}

	#[test]
	#[serial_test::serial(app_registry)]
	fn test_clear_reverse_relations() {
		clear_reverse_relations();

		let relation = ReverseRelationMetadata::new(
			"User",
			"posts".to_string(),
			"Post",
			ReverseRelationType::ReverseOneToMany,
			"author",
		);
		register_reverse_relation(relation);

		assert_eq!(get_reverse_relations_for_model("User").len(), 1);

		clear_reverse_relations();
		assert_eq!(get_reverse_relations_for_model("User").len(), 0);
	}

	#[test]
	fn test_reverse_relation_types() {
		assert_eq!(
			ReverseRelationType::ReverseOneToMany,
			ReverseRelationType::ReverseOneToMany
		);
		assert_ne!(
			ReverseRelationType::ReverseOneToMany,
			ReverseRelationType::ReverseManyToMany
		);
		assert_ne!(
			ReverseRelationType::ReverseOneToMany,
			ReverseRelationType::ReverseOneToOne
		);
	}

	// Test relationship metadata
	#[distributed_slice(RELATIONSHIPS)]
	static TEST_POST_AUTHOR: RelationshipMetadata = RelationshipMetadata {
		from_model: "blog.Post",
		to_model: "auth.User",
		relationship_type: RelationshipType::ForeignKey,
		field_name: "author",
		related_name: Some("posts"),
		db_column: Some("author_id"),
		through_table: None,
	};

	#[distributed_slice(RELATIONSHIPS)]
	static TEST_POST_TAGS: RelationshipMetadata = RelationshipMetadata {
		from_model: "blog.Post",
		to_model: "blog.Tag",
		relationship_type: RelationshipType::ManyToMany,
		field_name: "tags",
		related_name: Some("posts"),
		db_column: None,
		through_table: Some("blog_post_tags"),
	};

	#[test]
	fn test_relationship_metadata_new() {
		let relationship = RelationshipMetadata::new(
			"blog.Post",
			"auth.User",
			RelationshipType::ForeignKey,
			"author",
			Some("posts"),
			Some("author_id"),
			None,
		);

		assert_eq!(relationship.from_model, "blog.Post");
		assert_eq!(relationship.to_model, "auth.User");
		assert_eq!(relationship.relationship_type, RelationshipType::ForeignKey);
		assert_eq!(relationship.field_name, "author");
		assert_eq!(relationship.related_name, Some("posts"));
		assert_eq!(relationship.db_column, Some("author_id"));
		assert_eq!(relationship.through_table, None);
	}

	#[test]
	fn test_relationship_metadata_model_names() {
		let relationship = RelationshipMetadata::new(
			"blog.Post",
			"auth.User",
			RelationshipType::ForeignKey,
			"author",
			None,
			None,
			None,
		);

		assert_eq!(relationship.from_model_name(), "Post");
		assert_eq!(relationship.to_model_name(), "User");
	}

	#[test]
	fn test_relationship_types() {
		assert_eq!(RelationshipType::ForeignKey, RelationshipType::ForeignKey);
		assert_ne!(RelationshipType::ForeignKey, RelationshipType::ManyToMany);
		assert_ne!(RelationshipType::ForeignKey, RelationshipType::OneToOne);
	}

	#[test]
	fn test_relationship_metadata_equality() {
		let rel1 = RelationshipMetadata::new(
			"app.Model",
			"app.Other",
			RelationshipType::ForeignKey,
			"field",
			None,
			None,
			None,
		);
		let rel2 = RelationshipMetadata::new(
			"app.Model",
			"app.Other",
			RelationshipType::ForeignKey,
			"field",
			None,
			None,
			None,
		);
		let rel3 = RelationshipMetadata::new(
			"app.Model",
			"app.Other",
			RelationshipType::ManyToMany,
			"field",
			None,
			None,
			None,
		);

		assert_eq!(rel1, rel2);
		assert_ne!(rel1, rel3);
	}
}
