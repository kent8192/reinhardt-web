// Model serializer relationship tests - based on django-rest-framework
use reinhardt_orm::Model;
use reinhardt_serializers::{
    DefaultModelSerializer, Deserializer as ReinhardtDeserializer, ModelSerializer,
    ModelSerializerBuilder, RelationshipStrategy, Serializer,
};
use serde::{Deserialize, Serialize};

// Test models with relationships

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Author {
    id: Option<i64>,
    name: String,
    email: String,
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
    author_id: Option<i64>, // ForeignKey as ID
    isbn: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BookWithAuthor {
    id: Option<i64>,
    title: String,
    author: Option<Author>, // Nested relationship
    isbn: String,
}

impl Model for BookWithAuthor {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Profile {
    id: Option<i64>,
    user_id: i64, // OneToOne relationship
    bio: String,
    website: Option<String>,
}

impl Model for Profile {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "profiles"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

// Test: ForeignKey as primary key (PrimaryKey strategy)
#[test]
fn test_foreign_key_primary_key_strategy() {
    let serializer = ModelSerializerBuilder::<Book>::new()
        .relationship_strategy(RelationshipStrategy::PrimaryKey)
        .build();

    let book = Book {
        id: Some(1),
        title: "The Rust Book".to_string(),
        author_id: Some(42),
        isbn: "978-1-59327-828-1".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    // Should contain author_id as foreign key
    assert!(json_str.contains("\"author_id\":42"));

    let deserialized: Book = ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(book, deserialized);
}

// Test: ForeignKey with null value
#[test]
fn test_foreign_key_null() {
    let serializer = DefaultModelSerializer::<Book>::new();
    let book = Book {
        id: Some(1),
        title: "Orphan Book".to_string(),
        author_id: None, // No author
        isbn: "123-456".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let deserialized: Book = ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(deserialized.author_id, None);
}

// Test: Nested relationship (Nested strategy)
#[test]
fn test_nested_relationship_strategy() {
    let serializer = ModelSerializerBuilder::<BookWithAuthor>::new()
        .relationship_strategy(RelationshipStrategy::Nested)
        .depth(1)
        .build();

    let author = Author {
        id: Some(1),
        name: "Steve Klabnik".to_string(),
        email: "steve@example.com".to_string(),
    };

    let book = BookWithAuthor {
        id: Some(1),
        title: "The Rust Book".to_string(),
        author: Some(author.clone()),
        isbn: "978-1-59327-828-1".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    // Should contain nested author object
    assert!(json_str.contains("\"author\""));
    assert!(json_str.contains("\"Steve Klabnik\""));

    let deserialized: BookWithAuthor =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(book, deserialized);
}

// Test: Nested relationship with null value
#[test]
fn test_nested_relationship_null() {
    let serializer = DefaultModelSerializer::<BookWithAuthor>::new();
    let book = BookWithAuthor {
        id: Some(1),
        title: "Anonymous Book".to_string(),
        author: None,
        isbn: "000-000".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let deserialized: BookWithAuthor =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(deserialized.author, None);
}

// Test: OneToOne relationship
#[test]
fn test_one_to_one_relationship() {
    let serializer = DefaultModelSerializer::<Profile>::new();
    let profile = Profile {
        id: Some(1),
        user_id: 10,
        bio: "Rust developer".to_string(),
        website: Some("https://example.com".to_string()),
    };

    let serialized = Serializer::serialize(&serializer, &profile).unwrap();
    let deserialized: Profile =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(profile, deserialized);
    assert_eq!(deserialized.user_id, 10);
}

// Test: Depth level 0 (no nesting)
#[test]
fn test_depth_zero_no_nesting() {
    let serializer = ModelSerializerBuilder::<BookWithAuthor>::new()
        .depth(0)
        .build();

    let author = Author {
        id: Some(1),
        name: "Test Author".to_string(),
        email: "test@example.com".to_string(),
    };

    let book = BookWithAuthor {
        id: Some(1),
        title: "Test Book".to_string(),
        author: Some(author),
        isbn: "123".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let deserialized: BookWithAuthor =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    // With depth 0, nested objects should still serialize (serde handles this)
    assert_eq!(book, deserialized);
}

// Test: Depth level 1 (one level of nesting)
#[test]
fn test_depth_one_nesting() {
    let serializer = ModelSerializerBuilder::<BookWithAuthor>::new()
        .depth(1)
        .build();

    let author = Author {
        id: Some(1),
        name: "Author One".to_string(),
        email: "one@example.com".to_string(),
    };

    let book = BookWithAuthor {
        id: Some(1),
        title: "Book One".to_string(),
        author: Some(author),
        isbn: "111".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    // Should contain full nested author
    assert!(json_str.contains("\"Author One\""));

    let deserialized: BookWithAuthor =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(book, deserialized);
}

// Test: Multiple foreign keys in same model
#[test]
fn test_multiple_foreign_keys() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Review {
        id: Option<i64>,
        book_id: i64,
        reviewer_id: i64,
        rating: i32,
        comment: String,
    }

    impl Model for Review {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "reviews"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<Review>::new();
    let review = Review {
        id: Some(1),
        book_id: 5,
        reviewer_id: 10,
        rating: 5,
        comment: "Excellent!".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &review).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    assert!(json_str.contains("\"book_id\":5"));
    assert!(json_str.contains("\"reviewer_id\":10"));

    let deserialized: Review =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(review, deserialized);
}

// Test: Related name (reverse relationship) - conceptual test
#[test]
fn test_related_name_concept() {
    // In DRF, you can access reverse relationships like author.books
    // This test validates that we can serialize models with relationship metadata
    let author = Author {
        id: Some(1),
        name: "Prolific Author".to_string(),
        email: "prolific@example.com".to_string(),
    };

    let serializer = DefaultModelSerializer::<Author>::new();
    let serialized = Serializer::serialize(&serializer, &author).unwrap();
    let deserialized: Author =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(author, deserialized);
    // Note: Actual reverse relationship traversal requires ORM query support
}

// Test: Self-referencing relationship
#[test]
fn test_self_referencing_relationship() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Category {
        id: Option<i64>,
        name: String,
        parent_id: Option<i64>, // Self-referencing foreign key
    }

    impl Model for Category {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "categories"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<Category>::new();
    let parent = Category {
        id: Some(1),
        name: "Parent Category".to_string(),
        parent_id: None,
    };

    let child = Category {
        id: Some(2),
        name: "Child Category".to_string(),
        parent_id: Some(1),
    };

    let parent_serialized = Serializer::serialize(&serializer, &parent).unwrap();
    let child_serialized = Serializer::serialize(&serializer, &child).unwrap();

    let parent_deserialized: Category =
        ReinhardtDeserializer::deserialize(&serializer, &parent_serialized).unwrap();
    let child_deserialized: Category =
        ReinhardtDeserializer::deserialize(&serializer, &child_serialized).unwrap();

    assert_eq!(parent_deserialized.parent_id, None);
    assert_eq!(child_deserialized.parent_id, Some(1));
}

// Test: Relationship with complex nested structure
#[test]
fn test_complex_nested_structure() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Publisher {
        id: Option<i64>,
        name: String,
        country: String,
    }

    impl Model for Publisher {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "publishers"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct AuthorWithPublisher {
        id: Option<i64>,
        name: String,
        publisher: Option<Publisher>,
    }

    impl Model for AuthorWithPublisher {
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
    struct BookComplex {
        id: Option<i64>,
        title: String,
        author: Option<AuthorWithPublisher>,
    }

    impl Model for BookComplex {
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

    let publisher = Publisher {
        id: Some(1),
        name: "O'Reilly".to_string(),
        country: "USA".to_string(),
    };

    let author = AuthorWithPublisher {
        id: Some(1),
        name: "Steve Klabnik".to_string(),
        publisher: Some(publisher),
    };

    let book = BookComplex {
        id: Some(1),
        title: "The Rust Book".to_string(),
        author: Some(author),
    };

    let serializer = ModelSerializerBuilder::<BookComplex>::new()
        .depth(2)
        .build();

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    assert!(json_str.contains("\"O'Reilly\""));
    assert!(json_str.contains("\"Steve Klabnik\""));

    let deserialized: BookComplex =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(book, deserialized);
}

// Test: Relationship serialization with builder
#[test]
fn test_relationship_builder_configuration() {
    let pk_serializer = ModelSerializerBuilder::<Book>::new()
        .relationship_strategy(RelationshipStrategy::PrimaryKey)
        .build();

    let nested_serializer = ModelSerializerBuilder::<BookWithAuthor>::new()
        .relationship_strategy(RelationshipStrategy::Nested)
        .depth(1)
        .build();

    let book_pk = Book {
        id: Some(1),
        title: "Test".to_string(),
        author_id: Some(1),
        isbn: "123".to_string(),
    };

    let author = Author {
        id: Some(1),
        name: "Author".to_string(),
        email: "author@example.com".to_string(),
    };

    let book_nested = BookWithAuthor {
        id: Some(1),
        title: "Test".to_string(),
        author: Some(author),
        isbn: "123".to_string(),
    };

    let pk_serialized = Serializer::serialize(&pk_serializer, &book_pk).unwrap();
    let nested_serialized = Serializer::serialize(&nested_serializer, &book_nested).unwrap();

    // Both should serialize successfully
    assert!(!pk_serialized.is_empty());
    assert!(!nested_serialized.is_empty());
}

// Test: Relationship field validation
#[test]
fn test_relationship_field_validation() {
    let serializer = DefaultModelSerializer::<Book>::new();
    let book = Book {
        id: Some(1),
        title: "Valid Book".to_string(),
        author_id: Some(999), // Valid ID
        isbn: "978-1234567890".to_string(),
    };

    let result = serializer.create(book.clone());
    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.author_id, Some(999));
}

// Test: Update with relationship change
#[test]
fn test_update_relationship() {
    let serializer = DefaultModelSerializer::<Book>::new();
    let mut book = Book {
        id: Some(1),
        title: "Book".to_string(),
        author_id: Some(1),
        isbn: "123".to_string(),
    };

    let updated_data = Book {
        id: Some(1),
        title: "Book".to_string(),
        author_id: Some(2), // Changed author
        isbn: "123".to_string(),
    };

    let result = serializer.update(&mut book, updated_data);
    assert!(result.is_ok());
    assert_eq!(book.author_id, Some(2));
}

// Test: Nullable OneToOne relationship
#[test]
fn test_nullable_one_to_one() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct UserSettings {
        id: Option<i64>,
        user_id: Option<i64>, // Nullable OneToOne
        theme: String,
        notifications: bool,
    }

    impl Model for UserSettings {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "user_settings"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<UserSettings>::new();
    let settings = UserSettings {
        id: Some(1),
        user_id: None, // No associated user yet
        theme: "dark".to_string(),
        notifications: true,
    };

    let serialized = Serializer::serialize(&serializer, &settings).unwrap();
    let deserialized: UserSettings =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(deserialized.user_id, None);
}

// Test: Relationship field in JSON representation
#[test]
fn test_relationship_json_representation() {
    let serializer = DefaultModelSerializer::<Book>::new();
    let book = Book {
        id: Some(1),
        title: "JSON Test".to_string(),
        author_id: Some(42),
        isbn: "999".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &book).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    // Verify JSON structure includes relationship field
    assert!(json_str.contains("\"author_id\""));
    assert!(json_str.contains("42"));
}

// Test: Multiple levels of nesting with depth configuration
#[test]
fn test_multiple_nesting_levels() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Country {
        id: Option<i64>,
        name: String,
    }

    impl Model for Country {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "countries"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct City {
        id: Option<i64>,
        name: String,
        country: Option<Country>,
    }

    impl Model for City {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "cities"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Address {
        id: Option<i64>,
        street: String,
        city: Option<City>,
    }

    impl Model for Address {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "addresses"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let country = Country {
        id: Some(1),
        name: "Japan".to_string(),
    };

    let city = City {
        id: Some(1),
        name: "Tokyo".to_string(),
        country: Some(country),
    };

    let address = Address {
        id: Some(1),
        street: "Shibuya 1-1".to_string(),
        city: Some(city),
    };

    let serializer = ModelSerializerBuilder::<Address>::new().depth(2).build();

    let serialized = Serializer::serialize(&serializer, &address).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    // Should contain all nested levels
    assert!(json_str.contains("\"Tokyo\""));
    assert!(json_str.contains("\"Japan\""));

    let deserialized: Address =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(address, deserialized);
}

// Test: Relationship with required vs optional fields
#[test]
fn test_required_vs_optional_relationships() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Comment {
        id: Option<i64>,
        text: String,
        post_id: i64,           // Required relationship
        parent_id: Option<i64>, // Optional relationship (for nested comments)
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

    let serializer = DefaultModelSerializer::<Comment>::new();

    // Root comment (no parent)
    let root_comment = Comment {
        id: Some(1),
        text: "Root comment".to_string(),
        post_id: 10,
        parent_id: None,
    };

    // Reply comment (has parent)
    let reply_comment = Comment {
        id: Some(2),
        text: "Reply comment".to_string(),
        post_id: 10,
        parent_id: Some(1),
    };

    let root_serialized = Serializer::serialize(&serializer, &root_comment).unwrap();
    let reply_serialized = Serializer::serialize(&serializer, &reply_comment).unwrap();

    let root_deserialized: Comment =
        ReinhardtDeserializer::deserialize(&serializer, &root_serialized).unwrap();
    let reply_deserialized: Comment =
        ReinhardtDeserializer::deserialize(&serializer, &reply_serialized).unwrap();

    assert_eq!(root_deserialized.parent_id, None);
    assert_eq!(reply_deserialized.parent_id, Some(1));
}

// Test: Circular relationship prevention
#[test]
fn test_circular_relationship_awareness() {
    // This test validates that we can serialize models that could have
    // circular references when depth is properly limited
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Node {
        id: Option<i64>,
        name: String,
        related_node_id: Option<i64>,
    }

    impl Model for Node {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "nodes"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = ModelSerializerBuilder::<Node>::new()
        .depth(1) // Limit depth to prevent infinite recursion
        .build();

    let node = Node {
        id: Some(1),
        name: "Node 1".to_string(),
        related_node_id: Some(2),
    };

    let serialized = Serializer::serialize(&serializer, &node).unwrap();
    let deserialized: Node = ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(node, deserialized);
}

// Test: Relationship with composite data
#[test]
fn test_relationship_with_composite_data() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Tag {
        id: Option<i64>,
        name: String,
        color: String,
    }

    impl Model for Tag {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "tags"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct ArticleWithTags {
        id: Option<i64>,
        title: String,
        primary_tag: Option<Tag>,
        secondary_tag: Option<Tag>,
    }

    impl Model for ArticleWithTags {
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

    let tag1 = Tag {
        id: Some(1),
        name: "rust".to_string(),
        color: "orange".to_string(),
    };

    let tag2 = Tag {
        id: Some(2),
        name: "programming".to_string(),
        color: "blue".to_string(),
    };

    let article = ArticleWithTags {
        id: Some(1),
        title: "Learning Rust".to_string(),
        primary_tag: Some(tag1),
        secondary_tag: Some(tag2),
    };

    let serializer = ModelSerializerBuilder::<ArticleWithTags>::new()
        .depth(1)
        .build();

    let serialized = Serializer::serialize(&serializer, &article).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    assert!(json_str.contains("\"rust\""));
    assert!(json_str.contains("\"programming\""));

    let deserialized: ArticleWithTags =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(article, deserialized);
}

// NestedSerializer Tests

#[test]
fn test_nested_serializer_with_loaded_relationship() {
    use reinhardt_serializers::NestedSerializer;

    // Create book with pre-loaded author (simulating ORM eager loading)
    let author = Author {
        id: Some(1),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    let book = BookWithAuthor {
        id: Some(1),
        title: "Rust Programming".to_string(),
        author: Some(author.clone()),
        isbn: "978-1234567890".to_string(),
    };

    // Serialize with NestedSerializer
    let serializer = NestedSerializer::<BookWithAuthor, Author>::new("author").depth(1);
    let json = Serializer::serialize(&serializer, &book).unwrap();

    // Verify that author data is included
    assert!(json.contains("John Doe"));
    assert!(json.contains("john@example.com"));
    assert!(json.contains("Rust Programming"));
}

#[test]
fn test_nested_serializer_without_loaded_relationship() {
    use reinhardt_serializers::NestedSerializer;

    // Create book WITHOUT pre-loaded author
    let book = BookWithAuthor {
        id: Some(1),
        title: "Rust Programming".to_string(),
        author: None, // Relationship not loaded
        isbn: "978-1234567890".to_string(),
    };

    // Serialize with NestedSerializer
    let serializer = NestedSerializer::<BookWithAuthor, Author>::new("author").depth(1);
    let json = Serializer::serialize(&serializer, &book).unwrap();

    // Verify that author field is null
    assert!(json.contains("\"author\":null"));
    assert!(json.contains("Rust Programming"));
}

#[test]
fn test_nested_serializer_depth_0() {
    use reinhardt_serializers::NestedSerializer;

    let author = Author {
        id: Some(1),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    let book = BookWithAuthor {
        id: Some(1),
        title: "Rust Programming".to_string(),
        author: Some(author),
        isbn: "978-1234567890".to_string(),
    };

    // With depth=0, should still serialize the already-loaded data
    let serializer = NestedSerializer::<BookWithAuthor, Author>::new("author");
    let json = Serializer::serialize(&serializer, &book).unwrap();

    // Data should still be present (depth only affects recursive loading)
    assert!(json.contains("Rust Programming"));
}

#[test]
fn test_writable_nested_serializer_permission_validation() {
    use reinhardt_serializers::WritableNestedSerializer;

    // Test create permission validation
    let serializer = WritableNestedSerializer::<BookWithAuthor, Author>::new("author");

    let json_with_create = r#"{
        "id": 1,
        "title": "New Book",
        "author": {
            "id": null,
            "name": "New Author",
            "email": "new@example.com"
        },
        "isbn": "978-1111111111"
    }"#;

    // Should reject create when not allowed
    let result = ReinhardtDeserializer::deserialize(&serializer, &json_with_create.to_string());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .message
        .contains("Creating nested instances is not allowed"));
}

#[test]
fn test_writable_nested_serializer_allow_create() {
    use reinhardt_serializers::WritableNestedSerializer;

    let serializer =
        WritableNestedSerializer::<BookWithAuthor, Author>::new("author").allow_create(true);

    let json_with_create = r#"{
        "id": 1,
        "title": "New Book",
        "author": {
            "id": null,
            "name": "New Author",
            "email": "new@example.com"
        },
        "isbn": "978-1111111111"
    }"#;

    // Should allow create when enabled
    let result = ReinhardtDeserializer::deserialize(&serializer, &json_with_create.to_string());
    assert!(result.is_ok());
}

#[test]
fn test_writable_nested_serializer_allow_update() {
    use reinhardt_serializers::WritableNestedSerializer;

    let serializer =
        WritableNestedSerializer::<BookWithAuthor, Author>::new("author").allow_update(true);

    let json_with_update = r#"{
        "id": 1,
        "title": "Updated Book",
        "author": {
            "id": 42,
            "name": "Updated Author",
            "email": "updated@example.com"
        },
        "isbn": "978-2222222222"
    }"#;

    // Should allow update when enabled
    let result = ReinhardtDeserializer::deserialize(&serializer, &json_with_update.to_string());
    assert!(result.is_ok());
}
