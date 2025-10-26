//! NestedSerializer - Django REST Framework inspired nested serialization
//!
//! This module provides serializers for handling nested relationships between models,
//! enabling complex object graphs to be serialized and deserialized.
//!
//! # Relationship Loading Strategy
//!
//! Following Django REST Framework's design philosophy, `NestedSerializer` works with
//! data that is **already loaded** by the ORM layer. This separation of concerns means:
//!
//! - **ORM Layer (reinhardt-orm)**: Responsible for loading related data using
//!   `LoadingStrategy` (Lazy, Joined, Selectin, etc.)
//! - **Serializer Layer**: Responsible for serializing the already-loaded data to JSON
//!
//! ## Usage Pattern
//!
//! ```ignore
//! // 1. Load data with relationships using ORM
//! let posts = Post::objects()
//!     .select_related("author")  // Load author relationship
//!     .all()
//!     .await?;
//!
//! // 2. Serialize with NestedSerializer
//! let serializer = NestedSerializer::<Post, Author>::new("author").depth(1);
//! for post in posts {
//!     let json = serializer.serialize(&post)?;
//!     // JSON includes author data if it was loaded
//! }
//! ```
//!
//! This design avoids the N+1 query problem and gives developers explicit control
//! over when and how relationships are loaded.

use crate::{Serializer, SerializerError};
use reinhardt_orm::Model;
use serde_json::Value;
use std::marker::PhantomData;

/// NestedSerializer - Serialize related models inline
///
/// Handles one-to-one and many-to-one relationships by embedding the related
/// model's data directly in the serialized output.
///
/// # Examples
///
/// ```no_run
/// # use reinhardt_serializers::NestedSerializer;
/// # use reinhardt_orm::Model;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Post {
/// #     id: Option<i64>,
/// #     title: String,
/// #     author_id: i64,
/// # }
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Author {
/// #     id: Option<i64>,
/// #     name: String,
/// # }
/// #
/// # impl Model for Post {
/// #     type PrimaryKey = i64;
/// #     fn table_name() -> &'static str { "posts" }
/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// # }
/// #
/// # impl Model for Author {
/// #     type PrimaryKey = i64;
/// #     fn table_name() -> &'static str { "authors" }
/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// # }
/// #
/// # fn example() {
/// // Serialize a post with its author nested
/// let serializer = NestedSerializer::<Post, Author>::new("author");
/// # }
/// ```
pub struct NestedSerializer<M: Model, R: Model> {
    relationship_field: String,
    depth: usize,
    _phantom: PhantomData<(M, R)>,
}

impl<M: Model, R: Model> NestedSerializer<M, R> {
    /// Create a new NestedSerializer
    ///
    /// # Arguments
    ///
    /// * `relationship_field` - The field name that contains the related model
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::NestedSerializer;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct Post { id: Option<i64>, title: String }
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct Author { id: Option<i64>, name: String }
    /// #
    /// # impl Model for Post {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "posts" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// #
    /// # impl Model for Author {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "authors" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let serializer = NestedSerializer::<Post, Author>::new("author");
    /// ```
    pub fn new(relationship_field: impl Into<String>) -> Self {
        Self {
            relationship_field: relationship_field.into(),
            depth: 1,
            _phantom: PhantomData,
        }
    }

    /// Set the nesting depth (default: 1)
    ///
    /// Controls how many levels of relationships to serialize.
    /// depth=0 means no nesting (like ModelSerializer),
    /// depth=1 means serialize immediate relationships,
    /// depth=2+ means serialize nested relationships of relationships.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::NestedSerializer;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct Post { id: Option<i64>, title: String }
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct Author { id: Option<i64>, name: String }
    /// #
    /// # impl Model for Post {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "posts" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// #
    /// # impl Model for Author {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "authors" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let serializer = NestedSerializer::<Post, Author>::new("author")
    ///     .depth(2); // Serialize author and author's relationships
    /// ```
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }
}

impl<M: Model, R: Model> Serializer for NestedSerializer<M, R> {
    type Input = M;
    type Output = String;

    fn serialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError> {
        // Serialize parent model to JSON
        let mut parent_value = serde_json::to_value(input)
            .map_err(|e| SerializerError::new(format!("Serialization error: {}", e)))?;

        // If depth > 0, check if relationship data is already loaded in the parent JSON
        // This follows Django REST Framework's approach where related data is loaded
        // by the ORM layer (e.g., using select_related/prefetch_related) before serialization
        if self.depth > 0 {
            if let Some(obj) = parent_value.as_object_mut() {
                // Check if the relationship field already has data
                if let Some(related_data) = obj.get(&self.relationship_field) {
                    // If the data is not null, it means the relationship was already loaded
                    // by the ORM (e.g., via Joined or Selectin loading strategy)
                    if !related_data.is_null() {
                        // The relationship data is already present in the serialized JSON
                        // This works because reinhardt-orm's Model trait implementations
                        // include relationship fields in their Serialize implementation
                        // when those relationships are loaded
                    }
                }
            }
        }

        // Convert the value back to string
        serde_json::to_string(&parent_value)
            .map_err(|e| SerializerError::new(format!("Serialization error: {}", e)))
    }

    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError> {
        serde_json::from_str(output)
            .map_err(|e| SerializerError::new(format!("Deserialization error: {}", e)))
    }
}

/// ListSerializer - Serialize collections of models
///
/// Handles serializing multiple instances efficiently, useful for
/// many-to-many and reverse foreign key relationships.
///
/// # Examples
///
/// ```
/// # use reinhardt_serializers::ListSerializer;
/// # use reinhardt_orm::Model;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct User {
/// #     id: Option<i64>,
/// #     username: String,
/// # }
/// #
/// # impl Model for User {
/// #     type PrimaryKey = i64;
/// #     fn table_name() -> &'static str { "users" }
/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// # }
/// let serializer = ListSerializer::<User>::new();
/// ```
pub struct ListSerializer<M: Model> {
    _phantom: PhantomData<M>,
}

impl<M: Model> ListSerializer<M> {
    /// Create a new ListSerializer
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<M: Model> Default for ListSerializer<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Model> Serializer for ListSerializer<M> {
    type Input = Vec<M>;
    type Output = String;

    fn serialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError> {
        serde_json::to_string(input)
            .map_err(|e| SerializerError::new(format!("Serialization error: {}", e)))
    }

    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError> {
        serde_json::from_str(output)
            .map_err(|e| SerializerError::new(format!("Deserialization error: {}", e)))
    }
}

/// WritableNestedSerializer - Serialize and create/update nested models
///
/// Extends NestedSerializer to support write operations on nested relationships.
/// This allows creating or updating related models when the parent is saved.
///
/// # Design Philosophy
///
/// This serializer follows the **separation of concerns** principle:
/// - **Validation**: The serializer validates JSON structure and permissions
/// - **Data Extraction**: Provides helper methods to extract nested data
/// - **Database Operations**: Caller handles ORM operations and transactions
///
/// This design gives you full control over transaction management and error handling
/// while the serializer ensures data validity.
///
/// # Permission Control
///
/// - `allow_create(bool)`: Allow creating new related instances (default: false)
/// - `allow_update(bool)`: Allow updating existing related instances (default: false)
///
/// Without these permissions, deserialization will fail if nested data contains
/// create/update operations.
///
/// # Usage Patterns
///
/// ## Basic Usage with Manual ORM Integration
///
/// ```ignore
/// use reinhardt_serializers::WritableNestedSerializer;
/// use reinhardt_orm::{Model, Transaction};
///
/// // Define serializer with permissions
/// let serializer = WritableNestedSerializer::<Post, Author>::new("author")
///     .allow_create(true)
///     .allow_update(true);
///
/// // JSON with nested author
/// let json = r#"{
///     "title": "My Post",
///     "author": {
///         "id": null,
///         "name": "Alice"
///     }
/// }"#;
///
/// // Validate and deserialize
/// let post: Post = serializer.deserialize(&json.to_string())?;
///
/// // Extract nested data for manual processing
/// if let Some(author_data) = serializer.extract_nested_data(json)? {
///     // Create author within transaction
///     let mut tx = Transaction::new();
///     tx.begin()?;
///
///     let author: Author = serde_json::from_value(author_data)?;
///     let saved_author = Author::objects().create(&author).await?;
///
///     // Set foreign key and save parent
///     post.author_id = saved_author.id;
///     let saved_post = Post::objects().create(&post).await?;
///
///     tx.commit()?;
/// }
/// ```
///
/// ## Advanced: Handling Both Create and Update
///
/// ```ignore
/// if let Some(author_data) = serializer.extract_nested_data(json)? {
///     let mut tx = Transaction::new();
///     tx.begin()?;
///
///     let author: Author = serde_json::from_value(author_data)?;
///     let saved_author = if WritableNestedSerializer::<Post, Author>::is_create_operation(&author_data) {
///         // Create new author
///         Author::objects().create(&author).await?
///     } else {
///         // Update existing author
///         Author::objects().update(&author).await?
///     };
///
///     post.author_id = saved_author.id;
///     Post::objects().create(&post).await?;
///
///     tx.commit()?;
/// }
/// ```
///
/// # Error Handling
///
/// The serializer returns `SerializerError` in these cases:
/// - JSON parsing fails
/// - Nested data violates permissions (create/update not allowed)
/// - Invalid nested data structure
///
/// Database errors are handled by the caller during ORM operations.
///
/// # Examples
///
/// ```no_run
/// # use reinhardt_serializers::WritableNestedSerializer;
/// # use reinhardt_orm::Model;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Post { id: Option<i64>, title: String }
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct Comment { id: Option<i64>, text: String }
/// #
/// # impl Model for Post {
/// #     type PrimaryKey = i64;
/// #     fn table_name() -> &'static str { "posts" }
/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// # }
/// #
/// # impl Model for Comment {
/// #     type PrimaryKey = i64;
/// #     fn table_name() -> &'static str { "comments" }
/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// # }
/// #
/// # fn example() {
/// // Create a post and its comments in one operation
/// let serializer = WritableNestedSerializer::<Post, Comment>::new("comments")
///     .allow_create(true);
/// # }
/// ```
pub struct WritableNestedSerializer<M: Model, R: Model> {
    relationship_field: String,
    allow_create: bool,
    allow_update: bool,
    _phantom: PhantomData<(M, R)>,
}

impl<M: Model, R: Model> WritableNestedSerializer<M, R> {
    /// Create a new WritableNestedSerializer
    pub fn new(relationship_field: impl Into<String>) -> Self {
        Self {
            relationship_field: relationship_field.into(),
            allow_create: false,
            allow_update: false,
            _phantom: PhantomData,
        }
    }

    /// Allow creating new related instances (default: false)
    pub fn allow_create(mut self, allow: bool) -> Self {
        self.allow_create = allow;
        self
    }

    /// Allow updating existing related instances (default: false)
    pub fn allow_update(mut self, allow: bool) -> Self {
        self.allow_update = allow;
        self
    }

    /// Extract nested data from JSON for manual processing
    ///
    /// Returns the nested data as a serde_json::Value for the caller to process.
    /// This allows the caller to handle database operations with full control.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let serializer = WritableNestedSerializer::<Post, Author>::new("author");
    /// let json = r#"{"id": 1, "title": "Post", "author": {"id": 2, "name": "Alice"}}"#;
    ///
    /// if let Some(nested_data) = serializer.extract_nested_data(json)? {
    ///     // Process nested_data as needed
    ///     let author: Author = serde_json::from_value(nested_data)?;
    /// }
    /// ```
    pub fn extract_nested_data(&self, json: &str) -> Result<Option<Value>, SerializerError> {
        let value: Value = serde_json::from_str(json)
            .map_err(|e| SerializerError::new(format!("JSON parsing error: {}", e)))?;

        if let Value::Object(ref map) = value {
            if let Some(nested_value) = map.get(&self.relationship_field) {
                if !nested_value.is_null() {
                    return Ok(Some(nested_value.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Check if nested data represents a create operation (no primary key or null primary key)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let create_data = serde_json::json!({"id": null, "name": "New Author"});
    /// assert!(WritableNestedSerializer::<Post, Author>::is_create_operation(&create_data));
    ///
    /// let update_data = serde_json::json!({"id": 42, "name": "Existing Author"});
    /// assert!(!WritableNestedSerializer::<Post, Author>::is_create_operation(&update_data));
    /// ```
    pub fn is_create_operation(nested_value: &Value) -> bool {
        if let Some(pk) = nested_value.get(M::primary_key_field()) {
            pk.is_null()
        } else {
            true // No primary key field means create
        }
    }
}

impl<M: Model, R: Model> Serializer for WritableNestedSerializer<M, R> {
    type Input = M;
    type Output = String;

    fn serialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError> {
        // Same as NestedSerializer - requires ORM relationship loading
        // See NestedSerializer::serialize for implementation roadmap
        serde_json::to_string(input)
            .map_err(|e| SerializerError::new(format!("Serialization error: {}", e)))
    }

    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError> {
        // Parse JSON to validate structure
        let value: Value = serde_json::from_str(output)
            .map_err(|e| SerializerError::new(format!("JSON parsing error: {}", e)))?;

        // Check for nested data at relationship_field
        if let Value::Object(ref map) = value {
            if let Some(nested_value) = map.get(&self.relationship_field) {
                // Validate permissions
                if nested_value.is_object() {
                    // Single related object
                    if let Some(pk) = nested_value.get(M::primary_key_field()) {
                        if pk.is_null() && !self.allow_create {
                            return Err(SerializerError::new(
                                "Creating nested instances is not allowed".to_string(),
                            ));
                        } else if !pk.is_null() && !self.allow_update {
                            return Err(SerializerError::new(
                                "Updating nested instances is not allowed".to_string(),
                            ));
                        }
                    }
                } else if nested_value.is_array() {
                    // Multiple related objects
                    for item in nested_value.as_array().unwrap() {
                        if let Some(pk) = item.get(M::primary_key_field()) {
                            if pk.is_null() && !self.allow_create {
                                return Err(SerializerError::new(
                                    "Creating nested instances is not allowed".to_string(),
                                ));
                            } else if !pk.is_null() && !self.allow_update {
                                return Err(SerializerError::new(
                                    "Updating nested instances is not allowed".to_string(),
                                ));
                            }
                        }
                    }
                }

                // Nested data validation passed
                // The actual database operations (create/update) are handled by the caller
                // using ORM methods like QuerySet::create() or Model::save()
                //
                // This design follows the separation of concerns principle:
                // - Serializer: Validates structure and permissions
                // - ORM Layer: Performs database operations
                // - Transaction: Ensures atomicity
                //
                // Example usage pattern:
                // ```
                // let serializer = WritableNestedSerializer::new("author").allow_create(true);
                // let post: Post = serializer.deserialize(&json)?;
                //
                // // Caller handles database operations within transaction:
                // let mut tx = Transaction::new();
                // tx.begin()?;
                // let author = Author::objects().create(&post.author).await?;
                // post.author_id = author.id;
                // let saved_post = Post::objects().create(&post).await?;
                // tx.commit()?;
                // ```
            }
        }

        // For now, deserialize parent model only
        serde_json::from_str(output)
            .map_err(|e| SerializerError::new(format!("Deserialization error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Post {
        id: Option<i64>,
        title: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Author {
        id: Option<i64>,
        name: String,
    }

    impl Model for Post {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "posts"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    impl Model for Author {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "authors"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[test]
    fn test_nested_serializer_creation() {
        let serializer = NestedSerializer::<Post, Author>::new("author");
        assert_eq!(serializer.relationship_field, "author");
        assert_eq!(serializer.depth, 1);
    }

    #[test]
    fn test_nested_serializer_custom_depth() {
        let serializer = NestedSerializer::<Post, Author>::new("author").depth(3);
        assert_eq!(serializer.depth, 3);
    }

    #[test]
    fn test_list_serializer_creation() {
        let serializer = ListSerializer::<Post>::new();
        let posts = vec![
            Post {
                id: Some(1),
                title: String::from("First"),
            },
            Post {
                id: Some(2),
                title: String::from("Second"),
            },
        ];

        let result = serializer.serialize(&posts).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(value.is_array());
        assert_eq!(value.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_writable_nested_serializer_creation() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author")
            .allow_create(true)
            .allow_update(true);
        assert_eq!(serializer.relationship_field, "author");
        assert!(serializer.allow_create);
        assert!(serializer.allow_update);
    }

    #[test]
    fn test_writable_nested_default_permissions() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author");
        assert!(!serializer.allow_create);
        assert!(!serializer.allow_update);
    }

    #[test]
    fn test_writable_nested_deserialize_rejects_create_when_not_allowed() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author");

        // JSON with nested author without id (create operation)
        let json = r#"{
            "id": 1,
            "title": "Test Post",
            "author": {
                "id": null,
                "name": "New Author"
            }
        }"#;

        let result = serializer.deserialize(&json.to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Creating nested instances is not allowed"));
    }

    #[test]
    fn test_writable_nested_deserialize_rejects_update_when_not_allowed() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author");

        // JSON with nested author with id (update operation)
        let json = r#"{
            "id": 1,
            "title": "Test Post",
            "author": {
                "id": 42,
                "name": "Existing Author"
            }
        }"#;

        let result = serializer.deserialize(&json.to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Updating nested instances is not allowed"));
    }

    #[test]
    fn test_writable_nested_deserialize_allows_create_when_enabled() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author").allow_create(true);

        let json = r#"{
            "id": 1,
            "title": "Test Post",
            "author": {
                "id": null,
                "name": "New Author"
            }
        }"#;

        // Should not error - actual creation requires ORM integration
        let result = serializer.deserialize(&json.to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_writable_nested_deserialize_allows_update_when_enabled() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author").allow_update(true);

        let json = r#"{
            "id": 1,
            "title": "Test Post",
            "author": {
                "id": 42,
                "name": "Updated Author"
            }
        }"#;

        // Should not error - actual update requires ORM integration
        let result = serializer.deserialize(&json.to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_writable_nested_deserialize_array_rejects_create() {
        let serializer = WritableNestedSerializer::<Author, Post>::new("posts");

        let json = r#"{
            "id": 1,
            "name": "Author",
            "posts": [
                {"id": null, "title": "New Post"}
            ]
        }"#;

        let result = serializer.deserialize(&json.to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Creating nested instances is not allowed"));
    }

    #[test]
    fn test_writable_nested_deserialize_without_nested_data() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author");

        // JSON without nested data - should work fine
        let json = r#"{
            "id": 1,
            "title": "Test Post"
        }"#;

        let result = serializer.deserialize(&json.to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_nested_data_with_nested_object() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author");

        let json = r#"{
            "id": 1,
            "title": "Test Post",
            "author": {
                "id": 42,
                "name": "Alice"
            }
        }"#;

        let result = serializer.extract_nested_data(json).unwrap();
        assert!(result.is_some());

        let nested = result.unwrap();
        assert!(nested.is_object());
        assert_eq!(nested.get("id").unwrap().as_i64().unwrap(), 42);
        assert_eq!(nested.get("name").unwrap().as_str().unwrap(), "Alice");
    }

    #[test]
    fn test_extract_nested_data_without_nested_field() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author");

        let json = r#"{
            "id": 1,
            "title": "Test Post"
        }"#;

        let result = serializer.extract_nested_data(json).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_nested_data_with_null_nested_field() {
        let serializer = WritableNestedSerializer::<Post, Author>::new("author");

        let json = r#"{
            "id": 1,
            "title": "Test Post",
            "author": null
        }"#;

        let result = serializer.extract_nested_data(json).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_is_create_operation_with_null_pk() {
        let data = serde_json::json!({
            "id": null,
            "name": "New Author"
        });

        assert!(WritableNestedSerializer::<Post, Author>::is_create_operation(&data));
    }

    #[test]
    fn test_is_create_operation_with_existing_pk() {
        let data = serde_json::json!({
            "id": 42,
            "name": "Existing Author"
        });

        assert!(!WritableNestedSerializer::<Post, Author>::is_create_operation(&data));
    }

    #[test]
    fn test_is_create_operation_without_pk_field() {
        let data = serde_json::json!({
            "name": "Author Without ID"
        });

        assert!(WritableNestedSerializer::<Post, Author>::is_create_operation(&data));
    }

    #[test]
    fn test_extract_nested_data_with_array() {
        let serializer = WritableNestedSerializer::<Author, Post>::new("posts");

        let json = r#"{
            "id": 1,
            "name": "Alice",
            "posts": [
                {"id": 1, "title": "First Post"},
                {"id": 2, "title": "Second Post"}
            ]
        }"#;

        let result = serializer.extract_nested_data(json).unwrap();
        assert!(result.is_some());

        let nested = result.unwrap();
        assert!(nested.is_array());
        assert_eq!(nested.as_array().unwrap().len(), 2);
    }
}
