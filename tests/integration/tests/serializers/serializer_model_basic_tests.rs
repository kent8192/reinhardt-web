// Model serializer basic tests - based on django-rest-framework test_model_serializer.py
use reinhardt_orm::Model;
use reinhardt_serializers::{
    DefaultModelSerializer, Deserializer as ReinhardtDeserializer, ModelSerializer,
    ModelSerializerBuilder, RelationshipStrategy, Serializer,
};
use serde::{Deserialize, Serialize};

// Test models

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct User {
    id: Option<i64>,
    name: String,
    email: String,
    age: Option<i32>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Article {
    id: Option<i64>,
    title: String,
    content: String,
    published: bool,
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

// Test: Create method
#[test]
fn test_create_method() {
    let serializer = DefaultModelSerializer::<User>::new();
    let user = User {
        id: None,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        age: Some(30),
    };

    let result = serializer.create(user.clone());
    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.name, user.name);
    assert_eq!(created.email, user.email);
}

// Test: Regular fields mapping
#[test]
fn test_regular_fields() {
    let serializer = DefaultModelSerializer::<User>::new();
    let user = User {
        id: Some(1),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        age: Some(25),
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let deserialized: User = ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(user, deserialized);
    assert_eq!(deserialized.id, Some(1));
    assert_eq!(deserialized.name, "Alice");
    assert_eq!(deserialized.email, "alice@example.com");
    assert_eq!(deserialized.age, Some(25));
}

// Test: Field options (nullable fields)
#[test]
fn test_field_options() {
    let serializer = DefaultModelSerializer::<User>::new();
    let user = User {
        id: Some(2),
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
        age: None, // Optional field
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let deserialized: User = ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(deserialized.age, None);
}

// Test: Primary key fields
#[test]
fn test_pk_fields() {
    let serializer = DefaultModelSerializer::<User>::new();
    let user = User {
        id: Some(42),
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
        age: Some(30),
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    // Should contain the primary key
    assert!(json_str.contains("\"id\":42"));
}

// Test: Model serializer builder
#[test]
fn test_model_serializer_builder_integration() {
    let serializer = ModelSerializerBuilder::<User>::new()
        .relationship_strategy(RelationshipStrategy::Nested)
        .depth(1)
        .validate_unique(true)
        .build();

    let user = User {
        id: Some(1),
        name: "Builder Test".to_string(),
        email: "builder@example.com".to_string(),
        age: Some(35),
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let deserialized: User = ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(user, deserialized);
}

// Test: Empty serializer (minimal fields)
#[test]
fn test_minimal_model() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct MinimalModel {
        id: Option<i64>,
    }

    impl Model for MinimalModel {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "minimal"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<MinimalModel>::new();
    let model = MinimalModel { id: Some(1) };

    let serialized = Serializer::serialize(&serializer, &model).unwrap();
    let deserialized: MinimalModel =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(model, deserialized);
}

// Test: Update method
#[test]
fn test_update_method() {
    let serializer = DefaultModelSerializer::<User>::new();
    let mut user = User {
        id: Some(1),
        name: "Old Name".to_string(),
        email: "old@example.com".to_string(),
        age: Some(25),
    };

    let updated_data = User {
        id: Some(1),
        name: "New Name".to_string(),
        email: "new@example.com".to_string(),
        age: Some(26),
    };

    let result = serializer.update(&mut user, updated_data.clone());
    assert!(result.is_ok());
    assert_eq!(user, updated_data);
}

// Test: Serialization round trip
#[test]
fn test_model_serializer_round_trip() {
    let serializer = DefaultModelSerializer::<Article>::new();
    let article = Article {
        id: Some(100),
        title: "Test Article".to_string(),
        content: "This is test content".to_string(),
        published: true,
    };

    let serialized = Serializer::serialize(&serializer, &article).unwrap();
    let deserialized: Article =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(article, deserialized);
}

// Test: Boolean field
#[test]
fn test_model_serializer_boolean_field() {
    let serializer = DefaultModelSerializer::<Article>::new();
    let article = Article {
        id: Some(1),
        title: "Test".to_string(),
        content: "Content".to_string(),
        published: false,
    };

    let serialized = Serializer::serialize(&serializer, &article).unwrap();
    let deserialized: Article =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(deserialized.published, false);
}

// Test: Multiple instances (using JsonSerializer for Vec)
#[test]
fn test_multiple_instances() {
    use reinhardt_serializers::JsonSerializer;

    let serializer = JsonSerializer::<Vec<User>>::new();
    let users = vec![
        User {
            id: Some(1),
            name: "User1".to_string(),
            email: "user1@example.com".to_string(),
            age: Some(20),
        },
        User {
            id: Some(2),
            name: "User2".to_string(),
            email: "user2@example.com".to_string(),
            age: Some(30),
        },
    ];

    let serialized = Serializer::serialize(&serializer, &users).unwrap();
    let deserialized: Vec<User> =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(users.len(), deserialized.len());
    assert_eq!(users[0], deserialized[0]);
    assert_eq!(users[1], deserialized[1]);
}

// Test: Primary key auto-generation concept
#[test]
fn test_pk_none_to_some() {
    let _serializer = DefaultModelSerializer::<User>::new();
    let mut user = User {
        id: None, // No ID initially
        name: "New User".to_string(),
        email: "newuser@example.com".to_string(),
        age: Some(28),
    };

    // Simulate database insertion by setting PK
    user.set_primary_key(999);

    assert_eq!(user.primary_key(), Some(&999));
}

// Test: Model with string primary key
#[test]
fn test_string_pk() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct StringPkModel {
        id: Option<String>,
        data: String,
    }

    impl Model for StringPkModel {
        type PrimaryKey = String;

        fn table_name() -> &'static str {
            "string_pk_models"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<StringPkModel>::new();
    let model = StringPkModel {
        id: Some("abc123".to_string()),
        data: "test data".to_string(),
    };

    let serialized = Serializer::serialize(&serializer, &model).unwrap();
    let deserialized: StringPkModel =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(model, deserialized);
}

// Test: JSON field representation
#[test]
fn test_json_representation() {
    let serializer = DefaultModelSerializer::<User>::new();
    let user = User {
        id: Some(1),
        name: "JSON Test".to_string(),
        email: "json@example.com".to_string(),
        age: Some(25),
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    // Verify JSON structure
    assert!(json_str.contains("\"name\""));
    assert!(json_str.contains("\"email\""));
    assert!(json_str.contains("\"age\""));
}

// Test: Empty optional fields
#[test]
fn test_all_optional_fields_none() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct OptionalModel {
        id: Option<i64>,
        field1: Option<String>,
        field2: Option<i32>,
        field3: Option<bool>,
    }

    impl Model for OptionalModel {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "optional_models"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<OptionalModel>::new();
    let model = OptionalModel {
        id: Some(1),
        field1: None,
        field2: None,
        field3: None,
    };

    let serialized = Serializer::serialize(&serializer, &model).unwrap();
    let deserialized: OptionalModel =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(model, deserialized);
    assert!(deserialized.field1.is_none());
    assert!(deserialized.field2.is_none());
    assert!(deserialized.field3.is_none());
}

// Test: Large model with many fields
#[test]
fn test_large_model() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct LargeModel {
        id: Option<i64>,
        field1: String,
        field2: String,
        field3: i32,
        field4: i32,
        field5: bool,
        field6: bool,
        field7: Option<String>,
        field8: Option<i32>,
    }

    impl Model for LargeModel {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "large_models"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    let serializer = DefaultModelSerializer::<LargeModel>::new();
    let model = LargeModel {
        id: Some(1),
        field1: "value1".to_string(),
        field2: "value2".to_string(),
        field3: 100,
        field4: 200,
        field5: true,
        field6: false,
        field7: Some("optional".to_string()),
        field8: None,
    };

    let serialized = Serializer::serialize(&serializer, &model).unwrap();
    let deserialized: LargeModel =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(model, deserialized);
}

// Test: Model table name
#[test]
fn test_model_table_name() {
    assert_eq!(User::table_name(), "users");
    assert_eq!(Article::table_name(), "articles");
}

// Test: Model primary key field name
#[test]
fn test_model_pk_field_name() {
    assert_eq!(User::primary_key_field(), "id");
    assert_eq!(Article::primary_key_field(), "id");
}

// Test: Serializer with different depth levels
#[test]
fn test_serializer_depth_levels() {
    let serializer_depth_0 = ModelSerializerBuilder::<User>::new().depth(0).build();

    let serializer_depth_1 = ModelSerializerBuilder::<User>::new().depth(1).build();

    let serializer_depth_2 = ModelSerializerBuilder::<User>::new().depth(2).build();

    let user = User {
        id: Some(1),
        name: "Depth Test".to_string(),
        email: "depth@example.com".to_string(),
        age: Some(30),
    };

    // All should serialize the same way for a flat model
    let s0 = Serializer::serialize(&serializer_depth_0, &user).unwrap();
    let s1 = Serializer::serialize(&serializer_depth_1, &user).unwrap();
    let s2 = Serializer::serialize(&serializer_depth_2, &user).unwrap();

    assert_eq!(s0, s1);
    assert_eq!(s1, s2);
}
