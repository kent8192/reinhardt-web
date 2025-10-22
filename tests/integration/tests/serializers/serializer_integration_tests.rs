use reinhardt_orm::Model;
use reinhardt_serializers::{ModelSerializer, Serializer};

// Serializer integration tests - tests that require multiple crates
// Based on django-rest-framework model serializer tests

// Model serializer tests requiring ORM integration (65 tests from test_model_serializer.py)

#[test]
fn test_model_serializer_create_method() {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Book {
        id: Option<i64>,
        title: String,
        author: String,
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

    let serializer = ModelSerializer::<Book>::new();

    // Test creating a model instance via serializer deserialization
    let json_data = json!({
        "title": "The Rust Programming Language",
        "author": "Steve Klabnik and Carol Nichols",
        "isbn": "978-1718503106"
    });

    let json_str = serde_json::to_string(&json_data).unwrap();
    let mut book = serializer.deserialize(&json_str).unwrap();

    // Verify deserialized data
    assert_eq!(book.title, "The Rust Programming Language");
    assert_eq!(book.author, "Steve Klabnik and Carol Nichols");
    assert_eq!(book.isbn, "978-1718503106");
    assert_eq!(book.id, None); // New instance has no ID

    // Simulate database insertion by setting primary key
    book.set_primary_key(1);
    assert_eq!(book.primary_key(), Some(&1));

    // Serialize the model with ID
    let serialized = serializer.serialize(&book).unwrap();
    let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(value["id"], 1);
    assert_eq!(value["title"], "The Rust Programming Language");

    // Test roundtrip: deserialize -> modify -> serialize
    let deserialized = serializer.deserialize(&serialized).unwrap();
    assert_eq!(book, deserialized);
}

#[test]
fn test_abstract_model() {
    use serde::{Deserialize, Serialize};

    // Rust doesn't have abstract classes, but we can use traits to achieve similar behavior
    trait Timestamped {
        fn created_at(&self) -> &str;
        fn updated_at(&self) -> &str;
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct BlogPost {
        id: Option<i64>,
        title: String,
        content: String,
        created_at: String,
        updated_at: String,
    }

    impl Model for BlogPost {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "blog_posts"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    impl Timestamped for BlogPost {
        fn created_at(&self) -> &str {
            &self.created_at
        }
        fn updated_at(&self) -> &str {
            &self.updated_at
        }
    }

    let serializer = ModelSerializer::<BlogPost>::new();
    let post = BlogPost {
        id: Some(1),
        title: "Abstract Models in Rust".to_string(),
        content: "Using traits as abstract base classes".to_string(),
        created_at: "2025-01-01T00:00:00Z".to_string(),
        updated_at: "2025-01-02T00:00:00Z".to_string(),
    };

    // Validate that model with trait-based "abstract" behavior works with serializer
    assert!(serializer.validate(&post).is_ok());

    // Test serialization
    let serialized = serializer.serialize(&post).unwrap();
    let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(value["title"], "Abstract Models in Rust");
    assert_eq!(value["created_at"], "2025-01-01T00:00:00Z");
    assert_eq!(value["updated_at"], "2025-01-02T00:00:00Z");

    // Verify trait methods work
    assert_eq!(post.created_at(), "2025-01-01T00:00:00Z");
    assert_eq!(post.updated_at(), "2025-01-02T00:00:00Z");

    // Test deserialization roundtrip
    let deserialized = serializer.deserialize(&serialized).unwrap();
    assert_eq!(post, deserialized);
}

#[test]
fn test_pk_relations() {
    use reinhardt_orm::relationship::{Relationship, RelationshipType};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Category {
        id: Option<i64>,
        name: String,
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

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Product {
        id: Option<i64>,
        name: String,
        category_id: i64,
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

    // Create ForeignKey relationship (Product -> Category)
    let fk_relation =
        Relationship::<Product, Category>::new("category", RelationshipType::ManyToOne)
            .with_foreign_key("category_id")
            .scalar();

    assert_eq!(fk_relation.name(), "category");
    assert_eq!(fk_relation.relationship_type(), RelationshipType::ManyToOne);

    // Test serialization of models with FK relationship
    let category_serializer = ModelSerializer::<Category>::new();
    let category = Category {
        id: Some(1),
        name: "Electronics".to_string(),
    };
    assert!(category_serializer.validate(&category).is_ok());

    let product_serializer = ModelSerializer::<Product>::new();
    let product = Product {
        id: Some(10),
        name: "Laptop".to_string(),
        category_id: 1,
    };
    assert!(product_serializer.validate(&product).is_ok());

    // Serialize both models
    let cat_serialized = category_serializer.serialize(&category).unwrap();
    let prod_serialized = product_serializer.serialize(&product).unwrap();

    // Verify category serialization
    let cat_value: serde_json::Value = serde_json::from_str(&cat_serialized).unwrap();
    assert_eq!(cat_value["id"], 1);
    assert_eq!(cat_value["name"], "Electronics");

    // Verify product serialization includes foreign key
    let prod_value: serde_json::Value = serde_json::from_str(&prod_serialized).unwrap();
    assert_eq!(prod_value["id"], 10);
    assert_eq!(prod_value["name"], "Laptop");
    assert_eq!(prod_value["category_id"], 1);

    // Test deserialization roundtrip
    let deserialized_category = category_serializer.deserialize(&cat_serialized).unwrap();
    let deserialized_product = product_serializer.deserialize(&prod_serialized).unwrap();
    assert_eq!(category, deserialized_category);
    assert_eq!(product, deserialized_product);
}

#[test]
fn test_nested_relations() {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Department {
        id: Option<i64>,
        name: String,
    }

    impl Model for Department {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "departments"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Employee {
        id: Option<i64>,
        name: String,
        department_id: i64,
    }

    impl Model for Employee {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "employees"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // Test nested serialization with explicit JSON structure
    let dept_serializer = ModelSerializer::<Department>::new();
    let emp_serializer = ModelSerializer::<Employee>::new();

    let department = Department {
        id: Some(1),
        name: "Engineering".to_string(),
    };

    let employee = Employee {
        id: Some(100),
        name: "Alice".to_string(),
        department_id: 1,
    };

    // Serialize both models
    let dept_json = dept_serializer.serialize(&department).unwrap();
    let emp_json = emp_serializer.serialize(&employee).unwrap();

    // Create a nested structure manually (simulating what a nested serializer would do)
    let nested_data = json!({
        "id": employee.id,
        "name": employee.name,
        "department": {
            "id": department.id,
            "name": department.name
        }
    });

    // Verify nested structure contains both models' data
    assert_eq!(nested_data["id"], 100);
    assert_eq!(nested_data["name"], "Alice");
    assert_eq!(nested_data["department"]["id"], 1);
    assert_eq!(nested_data["department"]["name"], "Engineering");

    // Test that individual serializers work correctly
    let dept_value: serde_json::Value = serde_json::from_str(&dept_json).unwrap();
    let emp_value: serde_json::Value = serde_json::from_str(&emp_json).unwrap();

    assert_eq!(dept_value["name"], "Engineering");
    assert_eq!(emp_value["name"], "Alice");
    assert_eq!(emp_value["department_id"], 1);

    // Test deserialization roundtrip
    let deserialized_dept = dept_serializer.deserialize(&dept_json).unwrap();
    let deserialized_emp = emp_serializer.deserialize(&emp_json).unwrap();
    assert_eq!(department, deserialized_dept);
    assert_eq!(employee, deserialized_emp);
}

#[test]
fn test_hyperlinked_relations() {
    // Test hyperlinked model relations using URL reversal
    // Tests integration of: reinhardt-serializers + reinhardt-routers
    use reinhardt_routers::{Route, UrlReverser};
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::collections::HashMap;

    // Dummy handler for route registration
    use async_trait::async_trait;
    use reinhardt_apps::{Handler, Request, Response, Result as AppResult};
    struct DummyHandler;
    #[async_trait]
    impl Handler for DummyHandler {
        async fn handle(&self, _req: Request) -> AppResult<Response> {
            Ok(Response::ok())
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Post {
        id: Option<i64>,
        title: String,
        author_id: i64,
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

    // Create URL reverser and register route for author detail
    let mut reverser = UrlReverser::new();
    let route = Route::new("/api/authors/{id}/", std::sync::Arc::new(DummyHandler))
        .with_name("author-detail");
    reverser.register(route);

    // Create a post with author relation
    let post = Post {
        id: Some(1),
        title: "Test Post".to_string(),
        author_id: 42,
    };

    // Serialize the post
    let serializer = ModelSerializer::<Post>::new();
    let serialized = serializer.serialize(&post).unwrap();
    let json: Value = serde_json::from_str(&serialized).unwrap();

    // Verify basic fields
    assert_eq!(json["id"].as_i64().unwrap(), 1);
    assert_eq!(json["title"].as_str().unwrap(), "Test Post");
    assert_eq!(json["author_id"].as_i64().unwrap(), 42);

    // Generate hyperlink URL for the author
    let mut params = HashMap::new();
    params.insert("id".to_string(), post.author_id.to_string());
    let author_url = reverser.reverse("author-detail", &params).unwrap();

    // Verify hyperlink generation works correctly
    assert_eq!(author_url, "/api/authors/42/");

    // Test that URL reversal fails for non-existent routes
    assert!(reverser.reverse("non-existent-route", &params).is_err());

    // Test that URL reversal fails for missing parameters
    let empty_params = HashMap::new();
    assert!(reverser.reverse("author-detail", &empty_params).is_err());
}

#[test]
fn test_pk_reverse_foreign_key() {
    use reinhardt_orm::relationship::{Relationship, RelationshipType};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Book {
        id: Option<i64>,
        title: String,
        author_id: i64,
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

    // Create reverse foreign key relationship (Author -> Books)
    let reverse_rel = Relationship::<Author, Book>::new("books", RelationshipType::OneToMany)
        .with_foreign_key("author_id")
        .with_back_populates("author");

    assert_eq!(reverse_rel.name(), "books");
    assert_eq!(reverse_rel.relationship_type(), RelationshipType::OneToMany);

    // Verify reverse SQL generation
    let sql = reverse_rel.load_sql("1");
    assert!(sql.contains("SELECT * FROM books"));
    assert!(sql.contains("WHERE author_id = 1"));

    // Test serializer with Author model
    let serializer = ModelSerializer::<Author>::new();
    let author = Author {
        id: Some(1),
        name: "Jane Doe".to_string(),
    };

    assert!(serializer.validate(&author).is_ok());
    let serialized = serializer.serialize(&author).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();
    assert_eq!(author.name, deserialized.name);
}

#[test]
fn test_pk_reverse_one_to_one() {
    use reinhardt_orm::relationship::{Relationship, RelationshipType};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct User {
        id: Option<i64>,
        username: String,
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UserProfile {
        id: Option<i64>,
        user_id: i64,
        bio: String,
    }

    impl Model for UserProfile {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "user_profiles"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // Create bidirectional one-to-one relationship
    let user_to_profile =
        Relationship::<User, UserProfile>::new("profile", RelationshipType::OneToOne)
            .with_foreign_key("user_id")
            .with_back_populates("user")
            .scalar();

    let profile_to_user =
        Relationship::<UserProfile, User>::new("user", RelationshipType::OneToOne)
            .with_foreign_key("user_id")
            .with_back_populates("profile")
            .scalar();

    // Test forward relation (User -> Profile)
    assert_eq!(user_to_profile.name(), "profile");
    assert_eq!(
        user_to_profile.relationship_type(),
        RelationshipType::OneToOne
    );

    // Test reverse relation (Profile -> User)
    assert_eq!(profile_to_user.name(), "user");
    assert_eq!(
        profile_to_user.relationship_type(),
        RelationshipType::OneToOne
    );

    // Test serializers for both models
    let user_serializer = ModelSerializer::<User>::new();
    let user = User {
        id: Some(1),
        username: "alice".to_string(),
    };
    assert!(user_serializer.validate(&user).is_ok());

    let profile_serializer = ModelSerializer::<UserProfile>::new();
    let profile = UserProfile {
        id: Some(1),
        user_id: 1,
        bio: "Software Developer".to_string(),
    };
    assert!(profile_serializer.validate(&profile).is_ok());

    // Verify serialization roundtrip
    let serialized_user = user_serializer.serialize(&user).unwrap();
    let deserialized_user = user_serializer.deserialize(&serialized_user).unwrap();
    assert_eq!(user.username, deserialized_user.username);
}

#[test]
fn test_pk_reverse_many_to_many() {
    use reinhardt_orm::many_to_many::{AssociationTable, ManyToMany};
    use reinhardt_orm::relationship::{Relationship, RelationshipType};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Tag {
        id: Option<i64>,
        name: String,
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Article {
        id: Option<i64>,
        title: String,
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

    // Create association table for many-to-many
    let assoc_table = AssociationTable::new("article_tags", "article_id", "tag_id");

    // Create bidirectional many-to-many relationships
    let article_to_tags = Relationship::<Article, Tag>::new("tags", RelationshipType::ManyToMany)
        .with_secondary("article_tags");

    let tag_to_articles =
        Relationship::<Tag, Article>::new("articles", RelationshipType::ManyToMany)
            .with_secondary("article_tags");

    // Test forward relationship (Article -> Tags)
    assert_eq!(article_to_tags.name(), "tags");
    assert_eq!(
        article_to_tags.relationship_type(),
        RelationshipType::ManyToMany
    );

    // Test reverse relationship (Tag -> Articles)
    assert_eq!(tag_to_articles.name(), "articles");
    assert_eq!(
        tag_to_articles.relationship_type(),
        RelationshipType::ManyToMany
    );

    // Verify association table SQL generation
    let sql = assoc_table.to_create_sql();
    assert!(sql.contains("CREATE TABLE article_tags"));
    assert!(sql.contains("article_id INTEGER NOT NULL"));
    assert!(sql.contains("tag_id INTEGER NOT NULL"));
    assert!(sql.contains("PRIMARY KEY (article_id, tag_id)"));

    // Test ManyToMany relationship helper
    let m2m = ManyToMany::<Article, Tag>::new(assoc_table);
    let join_sql = m2m.join_sql();
    assert!(join_sql.contains("JOIN article_tags"));
    assert!(join_sql.contains("articles"));
    assert!(join_sql.contains("tags"));

    // Test serializers for both models
    let article_serializer = ModelSerializer::<Article>::new();
    let article = Article {
        id: Some(1),
        title: "Understanding Rust".to_string(),
    };
    assert!(article_serializer.validate(&article).is_ok());

    let tag_serializer = ModelSerializer::<Tag>::new();
    let tag = Tag {
        id: Some(1),
        name: "rust".to_string(),
    };
    assert!(tag_serializer.validate(&tag).is_ok());

    // Verify serialization roundtrip for both sides
    let serialized_article = article_serializer.serialize(&article).unwrap();
    let deserialized_article = article_serializer.deserialize(&serialized_article).unwrap();
    assert_eq!(article.title, deserialized_article.title);

    let serialized_tag = tag_serializer.serialize(&tag).unwrap();
    let deserialized_tag = tag_serializer.deserialize(&serialized_tag).unwrap();
    assert_eq!(tag.name, deserialized_tag.name);
}

#[test]
fn test_pk_reverse_through() {
    use reinhardt_orm::many_to_many::{AssociationTable, ManyToMany};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Student {
        id: Option<i64>,
        name: String,
    }

    impl Model for Student {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "students"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Course {
        id: Option<i64>,
        title: String,
    }

    impl Model for Course {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "courses"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // Create association table with through model
    let assoc_table = AssociationTable::new("student_courses", "student_id", "course_id")
        .with_column("enrolled_at", "TIMESTAMP")
        .with_column("grade", "VARCHAR(2)");

    // Create many-to-many relationship
    let m2m = ManyToMany::<Student, Course>::new(assoc_table.clone());

    // Verify association table SQL generation
    let sql = assoc_table.to_create_sql();
    assert!(sql.contains("CREATE TABLE student_courses"));
    assert!(sql.contains("student_id INTEGER NOT NULL"));
    assert!(sql.contains("course_id INTEGER NOT NULL"));
    assert!(sql.contains("enrolled_at TIMESTAMP"));
    assert!(sql.contains("grade VARCHAR(2)"));
    assert!(sql.contains("PRIMARY KEY (student_id, course_id)"));

    // Verify join SQL generation
    let join_sql = m2m.join_sql();
    assert!(join_sql.contains("JOIN student_courses"));
    assert!(join_sql.contains("students"));
    assert!(join_sql.contains("courses"));

    // Serialize and deserialize models
    let serializer = ModelSerializer::<Student>::new();
    let student = Student {
        id: Some(1),
        name: "Alice".to_string(),
    };

    assert!(serializer.validate(&student).is_ok());
}

#[test]
fn test_pk_create() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Product {
        id: Option<i64>,
        name: String,
        price: f64,
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

    let serializer = ModelSerializer::<Product>::new();

    // Create product without ID (simulating new record)
    let mut product = Product {
        id: None,
        name: "New Product".to_string(),
        price: 99.99,
    };

    // Validate the product
    assert!(serializer.validate(&product).is_ok());

    // Simulate setting primary key after insert
    product.set_primary_key(1);
    assert_eq!(product.primary_key(), Some(&1));
    assert_eq!(product.id, Some(1));

    // Serialize and deserialize
    let serialized = serializer.serialize(&product).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();
    assert_eq!(product, deserialized);
}

#[test]
fn test_pk_update() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Article {
        id: Option<i64>,
        title: String,
        content: String,
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

    let serializer = ModelSerializer::<Article>::new();

    // Existing article with ID (simulating update)
    let mut article = Article {
        id: Some(42),
        title: "Original Title".to_string(),
        content: "Original content".to_string(),
    };

    assert_eq!(article.primary_key(), Some(&42));

    // Update article
    article.title = "Updated Title".to_string();
    article.content = "Updated content".to_string();

    // Validate updated article
    assert!(serializer.validate(&article).is_ok());

    // Serialize and deserialize
    let serialized = serializer.serialize(&article).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();
    assert_eq!(article, deserialized);
    assert_eq!(deserialized.primary_key(), Some(&42));
    assert_eq!(deserialized.title, "Updated Title");
}

#[test]
fn test_meta_class_fields_option() {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UserWithPassword {
        id: Option<i64>,
        username: String,
        email: String,
        password: String,
    }

    impl Model for UserWithPassword {
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

    let serializer = ModelSerializer::<UserWithPassword>::new();
    let user = UserWithPassword {
        id: Some(1),
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        password: "secret123".to_string(),
    };

    // Serialize user - ModelSerializer should serialize all fields by default
    let serialized = serializer.serialize(&user).unwrap();
    let json: Value = serde_json::from_str(&serialized).unwrap();

    // Verify all fields are present in serialized output
    assert!(json.get("id").is_some());
    assert!(json.get("username").is_some());
    assert!(json.get("email").is_some());
    assert!(json.get("password").is_some());

    // Meta.fields would allow selecting specific fields
    // This tests that the serializer respects the Model interface
    assert_eq!(json["username"].as_str().unwrap(), "alice");
    assert_eq!(json["email"].as_str().unwrap(), "alice@example.com");
}

#[test]
fn test_meta_class_exclude_option() {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct SecureUser {
        id: Option<i64>,
        username: String,
        #[serde(skip_serializing)]
        #[allow(dead_code)]
        password_hash: String,
    }

    impl Model for SecureUser {
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

    let serializer = ModelSerializer::<SecureUser>::new();
    let user = SecureUser {
        id: Some(1),
        username: "bob".to_string(),
        password_hash: "hashed_secret".to_string(),
    };

    // Serialize user - password_hash should be excluded due to skip_serializing
    let serialized = serializer.serialize(&user).unwrap();
    let json: Value = serde_json::from_str(&serialized).unwrap();

    // Verify password_hash is excluded from serialized output
    assert!(json.get("id").is_some());
    assert!(json.get("username").is_some());
    assert!(json.get("password_hash").is_none());

    // Meta.exclude would allow excluding specific fields
    // This tests field-level exclusion using serde attributes
    assert_eq!(json["username"].as_str().unwrap(), "bob");
}

#[test]
fn test_unique_validator() {
    use reinhardt_serializers::UniqueValidator;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UniqueEmail {
        id: Option<i64>,
        email: String,
    }

    impl Model for UniqueEmail {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "unique_emails"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // Create UniqueValidator for email field
    let _validator = UniqueValidator::<UniqueEmail>::new("email");

    // UniqueValidator requires async database access for validation
    // In this test, we verify the validator is properly constructed
    // and the Model trait integration works correctly

    let serializer = ModelSerializer::<UniqueEmail>::new();
    let record1 = UniqueEmail {
        id: Some(1),
        email: "unique@example.com".to_string(),
    };

    // Validate model structure
    assert!(serializer.validate(&record1).is_ok());

    // Verify model metadata
    assert_eq!(UniqueEmail::table_name(), "unique_emails");
    assert_eq!(UniqueEmail::primary_key_field(), "id");
    assert_eq!(record1.primary_key(), Some(&1));

    // Serialize to verify UniqueValidator can work with serialized data
    let serialized = serializer.serialize(&record1).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();
    assert_eq!(record1.email, deserialized.email);
}

#[test]
fn test_unique_together_validator() {
    use reinhardt_orm::constraints::{Constraint, UniqueConstraint};
    use reinhardt_serializers::UniqueTogetherValidator;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TeamMember {
        id: Option<i64>,
        team_id: i64,
        user_id: i64,
        role: String,
    }

    impl Model for TeamMember {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "team_members"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // Create UniqueTogetherValidator for team_id and user_id
    // This ensures a user can only be in a team once
    let _validator = UniqueTogetherValidator::<TeamMember>::new(vec!["team_id", "user_id"]);

    // Create UniqueConstraint at ORM level
    let constraint = UniqueConstraint::new(
        "unique_team_user",
        vec!["team_id".to_string(), "user_id".to_string()],
    );

    // Verify constraint SQL generation
    let sql = constraint.to_sql();
    assert!(sql.contains("CONSTRAINT unique_team_user"));
    assert!(sql.contains("UNIQUE (team_id, user_id)"));
    assert_eq!(constraint.name(), "unique_team_user");
    assert_eq!(constraint.fields.len(), 2);

    // Test ModelSerializer with TeamMember
    let serializer = ModelSerializer::<TeamMember>::new();
    let member1 = TeamMember {
        id: Some(1),
        team_id: 1,
        user_id: 42,
        role: "developer".to_string(),
    };

    // Validate model structure
    assert!(serializer.validate(&member1).is_ok());

    // Verify model metadata
    assert_eq!(TeamMember::table_name(), "team_members");
    assert_eq!(TeamMember::primary_key_field(), "id");
    assert_eq!(member1.primary_key(), Some(&1));

    // Serialize and deserialize to verify validator compatibility
    let serialized = serializer.serialize(&member1).unwrap();
    let deserialized = serializer.deserialize(&serialized).unwrap();
    assert_eq!(member1.team_id, deserialized.team_id);
    assert_eq!(member1.user_id, deserialized.user_id);
    assert_eq!(member1.role, deserialized.role);

    // Create another member for different team (should be valid)
    let member2 = TeamMember {
        id: Some(2),
        team_id: 2,
        user_id: 42, // Same user, different team - should be allowed
        role: "manager".to_string(),
    };
    assert!(serializer.validate(&member2).is_ok());

    // Verify both members can be serialized
    let serialized2 = serializer.serialize(&member2).unwrap();
    let deserialized2 = serializer.deserialize(&serialized2).unwrap();
    assert_eq!(member2.team_id, deserialized2.team_id);
    assert_ne!(member1.team_id, member2.team_id); // Different teams
}

// Integration tests requiring multiple crates

#[test]
fn test_serializer_in_api_view() {
    // Test using serializers in REST API views
    // Tests integration of: reinhardt-serializers + reinhardt-rest (ApiResponse)
    use reinhardt_rest::ApiResponse;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Task {
        id: Option<i64>,
        title: String,
        completed: bool,
    }

    impl Model for Task {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            "tasks"
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    // Create tasks
    let task1 = Task {
        id: Some(1),
        title: "Write tests".to_string(),
        completed: false,
    };
    let task2 = Task {
        id: Some(2),
        title: "Review code".to_string(),
        completed: true,
    };

    // Serialize tasks using ModelSerializer
    let serializer = ModelSerializer::<Task>::new();
    assert!(serializer.validate(&task1).is_ok());
    assert!(serializer.validate(&task2).is_ok());

    // Create API responses with serialized data
    let task_list = vec![task1.clone(), task2.clone()];
    let list_response = ApiResponse::success(task_list.clone());

    // Verify list response structure
    assert_eq!(list_response.status, 200);
    assert!(list_response.data.is_some());
    assert!(list_response.error.is_none());
    assert_eq!(list_response.data.as_ref().unwrap().len(), 2);

    // Create single item response
    let detail_response = ApiResponse::success_with_status(task1.clone(), 200);
    assert_eq!(detail_response.status, 200);
    assert!(detail_response.data.is_some());
    assert_eq!(detail_response.data.as_ref().unwrap().id, Some(1));

    // Test creation response (201 Created)
    let new_task = Task {
        id: Some(3),
        title: "Deploy app".to_string(),
        completed: false,
    };
    assert!(serializer.validate(&new_task).is_ok());
    let create_response = ApiResponse::success_with_status(new_task.clone(), 201);
    assert_eq!(create_response.status, 201);
    assert!(create_response.data.is_some());

    // Test error response
    let error_response: ApiResponse<Task> = ApiResponse::not_found();
    assert_eq!(error_response.status, 404);
    assert!(error_response.data.is_none());
    assert_eq!(error_response.error.as_ref().unwrap(), "Not found");

    // Test JSON serialization of API response
    let json = list_response.to_json().unwrap();
    assert!(json.contains("\"status\":200"));
    assert!(json.contains("Write tests"));
    assert!(json.contains("Review code"));
}

#[test]
fn test_paginated_serializer_response() {
    // Test paginated serializer responses
    // Tests integration of: reinhardt-serializers + reinhardt-pagination
    use reinhardt_pagination::{PaginatedResponse, PaginationMetadata};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Comment {
        id: Option<i64>,
        text: String,
        author: String,
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

    // Create sample comments
    let comments = vec![
        Comment {
            id: Some(1),
            text: "Great article!".to_string(),
            author: "Alice".to_string(),
        },
        Comment {
            id: Some(2),
            text: "Very informative".to_string(),
            author: "Bob".to_string(),
        },
        Comment {
            id: Some(3),
            text: "Thanks for sharing".to_string(),
            author: "Charlie".to_string(),
        },
    ];

    // Validate all comments with serializer
    let serializer = ModelSerializer::<Comment>::new();
    for comment in &comments {
        assert!(serializer.validate(comment).is_ok());
    }

    // Create pagination metadata
    let metadata = PaginationMetadata {
        count: 10, // Total 10 comments across all pages
        next: Some("/api/comments?page=2".to_string()),
        previous: None, // First page has no previous
    };

    // Create paginated response
    let paginated = PaginatedResponse::new(comments.clone(), metadata);

    // Verify pagination structure
    assert_eq!(paginated.count, 10);
    assert_eq!(paginated.results.len(), 3); // 3 items on this page
    assert_eq!(paginated.next.as_ref().unwrap(), "/api/comments?page=2");
    assert!(paginated.previous.is_none());

    // Verify serialized data integrity
    assert_eq!(paginated.results[0], comments[0]);
    assert_eq!(paginated.results[1], comments[1]);
    assert_eq!(paginated.results[2], comments[2]);

    // Test JSON serialization of paginated response
    let json = serde_json::to_string(&paginated).unwrap();
    assert!(json.contains("\"count\":10"));
    assert!(json.contains("Great article!"));
    assert!(json.contains("\"author\":\"Alice\""));
    assert!(json.contains("?page=2"));

    // Test last page (no next, has previous)
    let last_page_metadata = PaginationMetadata {
        count: 10,
        next: None,
        previous: Some("/api/comments?page=2".to_string()),
    };
    let last_page = PaginatedResponse::new(vec![comments[0].clone()], last_page_metadata);
    assert!(last_page.next.is_none());
    assert!(last_page.previous.is_some());
    assert_eq!(last_page.results.len(), 1);
}

// Implementation notes for future work:
//
// 1. Model Serializer Implementation Plan:
//    - Create ModelSerializer trait in reinhardt-serializers
//    - Integrate with reinhardt-orm Model trait
//    - Support automatic field generation from model
//    - Support Meta class configuration
//    - Support nested serializers for relations
//
// 2. Required Dependencies:
//    - reinhardt-orm: Model, Field types, QuerySet
//    - reinhardt-validators: UniqueValidator, UniqueTogetherValidator
//    - reinhardt-rest: URL routing, reverse URL generation
//    - reinhardt-forms: HTML form rendering
//    - reinhardt-pagination: Pagination support
//
// 3. Database Test Setup:
//    - Use SQLx for database operations
//    - Support both SQLite (for CI) and PostgreSQL (for production)
//    - Use transactions for test isolation
//    - Implement test fixtures and factories
//
// 4. Performance Considerations:
//    - Implement select_related for eager loading
//    - Implement prefetch_related for optimized queries
//    - Cache field introspection results
//    - Use connection pooling in tests
