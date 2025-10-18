//! Model serializer integration tests - based on django-rest-framework test_model_serializer.py
//! Tests the integration between reinhardt-orm and reinhardt-serializers

use reinhardt_orm::Model;
use reinhardt_serializers::{ModelSerializer, Serializer};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Test Models
    // ============================================================================

    /// Simple test model for basic CRUD operations
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct User {
        id: Option<i64>,
        username: String,
        email: String,
        age: i32,
    }

    impl Model for User {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "users"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    /// Test model with foreign key
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Article {
        id: Option<i64>,
        title: String,
        content: String,
        author_id: i64,
    }

    impl Model for Article {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "articles"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    /// Test model for validation
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Product {
        id: Option<i64>,
        name: String,
        price: f64,
        quantity: i32,
    }

    impl Model for Product {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "products"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // ============================================================================
    // Basic Model Serialization Tests
    // ============================================================================

    #[test]
    fn test_model_serializer_create() {
        // Test creating model instances from serialized data
        let serializer = ModelSerializer::<User>::new();

        let user = User {
            id: Some(1),
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            age: 30,
        };

        // Validate the model instance
        assert!(serializer.validate(&user).is_ok());

        // Serialize to JSON
        let serialized = serializer.serialize(&user).unwrap();
        assert!(serialized.contains("alice"));
        assert!(serialized.contains("alice@example.com"));
        assert!(serialized.contains("30"));

        // Deserialize back
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(user, deserialized);
    }

    #[test]
    fn test_model_serializer_update() {
        // Test updating existing model instances
        let serializer = ModelSerializer::<User>::new();

        // Create updated version
        let updated = User {
            id: Some(1),
            username: "bob_updated".to_string(),
            email: "bob.new@example.com".to_string(),
            age: 26,
        };

        // Serialize updated data
        let serialized = serializer.serialize(&updated).unwrap();

        // Verify updates are reflected
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(deserialized.username, "bob_updated");
        assert_eq!(deserialized.email, "bob.new@example.com");
        assert_eq!(deserialized.age, 26);

        // Primary key should remain the same
        assert_eq!(deserialized.id, Some(1));
    }

    #[test]
    fn test_model_serializer_foreign_key() {
        // Test serializing models with foreign key relationships
        let serializer = ModelSerializer::<Article>::new();

        let article = Article {
            id: Some(1),
            title: "Test Article".to_string(),
            content: "This is test content".to_string(),
            author_id: 42,
        };

        // Validate
        assert!(serializer.validate(&article).is_ok());

        // Serialize
        let serialized = serializer.serialize(&article).unwrap();
        assert!(serialized.contains("Test Article"));
        assert!(serialized.contains("42")); // Foreign key should be included

        // Deserialize
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(article.author_id, deserialized.author_id);
    }

    #[test]
    fn test_model_serializer_many_to_many() {
        // Test serializing models with many-to-many relationships
        // Note: This is a simplified test as full M2M support requires additional infrastructure

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Post {
            id: Option<i64>,
            title: String,
            tag_ids: Vec<i64>, // Simplified M2M as array of IDs
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

        let serializer = ModelSerializer::<Post>::new();

        let post = Post {
            id: Some(1),
            title: "Tagged Post".to_string(),
            tag_ids: vec![1, 2, 3],
        };

        // Serialize
        let serialized = serializer.serialize(&post).unwrap();
        assert!(serialized.contains("Tagged Post"));

        // Deserialize
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(post.tag_ids, deserialized.tag_ids);
        assert_eq!(post.tag_ids.len(), 3);
    }

    #[test]
    fn test_model_serializer_validation() {
        // Test model-level validation integration
        let serializer = ModelSerializer::<Product>::new();

        let valid_product = Product {
            id: None,
            name: "Widget".to_string(),
            price: 19.99,
            quantity: 100,
        };

        // Valid product should pass validation
        assert!(serializer.validate(&valid_product).is_ok());

        // Test serialization of valid product
        let serialized = serializer.serialize(&valid_product).unwrap();
        assert!(serialized.contains("Widget"));
        assert!(serialized.contains("19.99"));

        // Round-trip validation
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert!(serializer.validate(&deserialized).is_ok());
    }

    // ============================================================================
    // Meta Configuration Tests
    // ============================================================================

    #[test]
    fn test_model_serializer_meta_fields() {
        // Test Meta class field configuration
        // Note: This tests that all fields are included by default
        let serializer = ModelSerializer::<User>::new();

        let user = User {
            id: Some(1),
            username: "charlie".to_string(),
            email: "charlie@example.com".to_string(),
            age: 35,
        };

        let serialized = serializer.serialize(&user).unwrap();

        // All fields should be present in serialization
        assert!(serialized.contains("\"id\":1"));
        assert!(serialized.contains("charlie"));
        assert!(serialized.contains("charlie@example.com"));
        assert!(serialized.contains("35"));
    }

    #[test]
    fn test_model_serializer_read_only_fields() {
        // Test read-only field handling
        // ID field acts as read-only in create scenarios

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct ReadOnlyModel {
            id: Option<i64>, // Read-only: set by database
            name: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            created_at: Option<String>, // Read-only timestamp
        }

        impl Model for ReadOnlyModel {
            type PrimaryKey = i64;

            fn table_name() -> &'static str {
                "readonly_models"
            }

            fn primary_key(&self) -> Option<&Self::PrimaryKey> {
                self.id.as_ref()
            }

            fn set_primary_key(&mut self, value: Self::PrimaryKey) {
                self.id = Some(value);
            }
        }

        let serializer = ModelSerializer::<ReadOnlyModel>::new();

        let model = ReadOnlyModel {
            id: Some(1),
            name: "test".to_string(),
            created_at: Some("2024-01-01".to_string()),
        };

        // Read-only fields should be included in serialization
        let serialized = serializer.serialize(&model).unwrap();
        assert!(serialized.contains("\"id\":1"));
        assert!(serialized.contains("2024-01-01"));

        // Can deserialize with read-only fields
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(model.id, deserialized.id);
        assert_eq!(model.created_at, deserialized.created_at);
    }

    #[test]
    fn test_model_serializer_write_only_fields() {
        // Test write-only field handling

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct PasswordModel {
            id: Option<i64>,
            username: String,
            #[serde(skip_serializing)] // Write-only: never exposed
            password: String,
        }

        impl Model for PasswordModel {
            type PrimaryKey = i64;

            fn table_name() -> &'static str {
                "password_models"
            }

            fn primary_key(&self) -> Option<&Self::PrimaryKey> {
                self.id.as_ref()
            }

            fn set_primary_key(&mut self, value: Self::PrimaryKey) {
                self.id = Some(value);
            }
        }

        let serializer = ModelSerializer::<PasswordModel>::new();

        let model = PasswordModel {
            id: Some(1),
            username: "secure_user".to_string(),
            password: "secret123".to_string(),
        };

        // Password should NOT be in serialized output
        let serialized = serializer.serialize(&model).unwrap();
        assert!(serialized.contains("secure_user"));
        assert!(!serialized.contains("secret123"));
        assert!(!serialized.contains("password"));
    }

    #[test]
    fn test_model_serializer_custom_fields() {
        // Test custom field definitions with serde attributes

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct CustomFieldModel {
            id: Option<i64>,
            #[serde(rename = "full_name")]
            name: String,
            #[serde(default)]
            status: String,
        }

        impl Model for CustomFieldModel {
            type PrimaryKey = i64;

            fn table_name() -> &'static str {
                "custom_models"
            }

            fn primary_key(&self) -> Option<&Self::PrimaryKey> {
                self.id.as_ref()
            }

            fn set_primary_key(&mut self, value: Self::PrimaryKey) {
                self.id = Some(value);
            }
        }

        let serializer = ModelSerializer::<CustomFieldModel>::new();

        let model = CustomFieldModel {
            id: Some(1),
            name: "John Doe".to_string(),
            status: "active".to_string(),
        };

        // Field should be renamed in serialization
        let serialized = serializer.serialize(&model).unwrap();
        assert!(serialized.contains("full_name"));
        assert!(!serialized.contains("\"name\""));
        assert!(serialized.contains("John Doe"));

        // Deserialize with custom field name
        let json = r#"{"id":2,"full_name":"Jane Doe"}"#.to_string();
        let deserialized = serializer.deserialize(&json).unwrap();
        assert_eq!(deserialized.name, "Jane Doe");
        assert_eq!(deserialized.status, ""); // Default value
    }

    // ============================================================================
    // Nested and Related Object Tests
    // ============================================================================

    #[test]
    fn test_model_serializer_nested_create() {
        // Test creating nested model instances

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Author {
            id: Option<i64>,
            name: String,
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

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Book {
            id: Option<i64>,
            title: String,
            author: Author, // Nested author object
        }

        impl Model for Book {
            type PrimaryKey = i64;

            fn table_name() -> &'static str {
                "books"
            }

            fn primary_key(&self) -> Option<&Self::PrimaryKey> {
                self.id.as_ref()
            }

            fn set_primary_key(&mut self, value: Self::PrimaryKey) {
                self.id = Some(value);
            }
        }

        let serializer = ModelSerializer::<Book>::new();

        let book = Book {
            id: Some(1),
            title: "Rust Programming".to_string(),
            author: Author {
                id: Some(10),
                name: "Alice Cooper".to_string(),
            },
        };

        // Validate nested structure
        assert!(serializer.validate(&book).is_ok());

        // Serialize with nested object
        let serialized = serializer.serialize(&book).unwrap();
        assert!(serialized.contains("Rust Programming"));
        assert!(serialized.contains("Alice Cooper"));

        // Deserialize with nested object
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(book.author.name, deserialized.author.name);
        assert_eq!(book.author.id, deserialized.author.id);
    }

    // ============================================================================
    // Constraint and Validation Tests
    // ============================================================================

    #[test]
    fn test_model_serializer_unique_together() {
        // Test unique_together constraint validation
        // Note: Actual constraint enforcement would be at database level

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Membership {
            id: Option<i64>,
            user_id: i64,
            group_id: i64,
            role: String,
        }

        impl Model for Membership {
            type PrimaryKey = i64;

            fn table_name() -> &'static str {
                "memberships"
            }

            fn primary_key(&self) -> Option<&Self::PrimaryKey> {
                self.id.as_ref()
            }

            fn set_primary_key(&mut self, value: Self::PrimaryKey) {
                self.id = Some(value);
            }
        }

        let serializer = ModelSerializer::<Membership>::new();

        let membership = Membership {
            id: Some(1),
            user_id: 10,
            group_id: 20,
            role: "admin".to_string(),
        };

        // Validate the combination
        assert!(serializer.validate(&membership).is_ok());

        // Serialize
        let serialized = serializer.serialize(&membership).unwrap();
        assert!(serialized.contains("\"user_id\":10"));
        assert!(serialized.contains("\"group_id\":20"));

        // Round-trip
        let deserialized = serializer.deserialize(&serialized).unwrap();
        assert_eq!(membership.user_id, deserialized.user_id);
        assert_eq!(membership.group_id, deserialized.group_id);
    }

    #[test]
    fn test_model_serializer_queryset_optimization() {
        // Test queryset optimization (select_related, prefetch_related)
        // This test verifies that serialization doesn't break with eager-loaded data

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Comment {
            id: Option<i64>,
            text: String,
            post_id: i64,
            author_id: i64,
        }

        impl Model for Comment {
            type PrimaryKey = i64;

            fn table_name() -> &'static str {
                "comments"
            }

            fn primary_key(&self) -> Option<&Self::PrimaryKey> {
                self.id.as_ref()
            }

            fn set_primary_key(&mut self, value: Self::PrimaryKey) {
                self.id = Some(value);
            }
        }

        let serializer = ModelSerializer::<Comment>::new();

        // Simulate data that would be loaded with select_related
        let comments = vec![
            Comment {
                id: Some(1),
                text: "Great post!".to_string(),
                post_id: 100,
                author_id: 1,
            },
            Comment {
                id: Some(2),
                text: "Thanks for sharing".to_string(),
                post_id: 100,
                author_id: 2,
            },
            Comment {
                id: Some(3),
                text: "Interesting perspective".to_string(),
                post_id: 101,
                author_id: 1,
            },
        ];

        // Verify all comments can be serialized
        for comment in &comments {
            let serialized = serializer.serialize(comment).unwrap();
            assert!(serialized.len() > 0);

            let deserialized = serializer.deserialize(&serialized).unwrap();
            assert_eq!(comment, &deserialized);
        }

        // Verify foreign key references are maintained
        assert_eq!(comments[0].post_id, comments[1].post_id);
        assert_eq!(comments[0].author_id, comments[2].author_id);
    }
}
