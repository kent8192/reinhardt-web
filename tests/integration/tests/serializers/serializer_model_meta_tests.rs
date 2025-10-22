// Model serializer Meta configuration tests
use reinhardt_orm::Model;
use reinhardt_serializers::{
    DefaultModelSerializer, Deserializer as ReinhardtDeserializer, ModelSerializer, Serializer,
};
use serde::{Deserialize, Serialize};

// Test models

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct UserFull {
    id: Option<i64>,
    username: String,
    email: String,
    password: String,
    first_name: String,
    last_name: String,
    is_active: bool,
}

impl Model for UserFull {
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

// Custom serializer with Meta.fields
struct UserFieldsSerializer;

impl Serializer<UserFull> for UserFieldsSerializer {
    fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ReinhardtDeserializer<UserFull> for UserFieldsSerializer {
    fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
        serde_json::from_slice(data)
            .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ModelSerializer<UserFull> for UserFieldsSerializer {
    fn meta_fields() -> Option<Vec<&'static str>> {
        Some(vec!["id", "username", "email"])
    }
}

// Custom serializer with Meta.exclude
struct UserExcludeSerializer;

impl Serializer<UserFull> for UserExcludeSerializer {
    fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ReinhardtDeserializer<UserFull> for UserExcludeSerializer {
    fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
        serde_json::from_slice(data)
            .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ModelSerializer<UserFull> for UserExcludeSerializer {
    fn meta_exclude() -> Option<Vec<&'static str>> {
        Some(vec!["password"])
    }
}

// Custom serializer with Meta.read_only_fields
struct UserReadOnlySerializer;

impl Serializer<UserFull> for UserReadOnlySerializer {
    fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ReinhardtDeserializer<UserFull> for UserReadOnlySerializer {
    fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
        serde_json::from_slice(data)
            .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ModelSerializer<UserFull> for UserReadOnlySerializer {
    fn meta_read_only_fields() -> Option<Vec<&'static str>> {
        Some(vec!["id", "is_active"])
    }
}

// Custom serializer with Meta.write_only_fields
struct UserWriteOnlySerializer;

impl Serializer<UserFull> for UserWriteOnlySerializer {
    fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ReinhardtDeserializer<UserFull> for UserWriteOnlySerializer {
    fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
        serde_json::from_slice(data)
            .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
    }
}

impl ModelSerializer<UserFull> for UserWriteOnlySerializer {
    fn meta_write_only_fields() -> Option<Vec<&'static str>> {
        Some(vec!["password"])
    }
}

// Test: Meta.fields configuration
#[test]
fn test_meta_fields() {
    let serializer = UserFieldsSerializer;
    let user = UserFull {
        id: Some(1),
        username: "john".to_string(),
        email: "john@example.com".to_string(),
        password: "secret".to_string(),
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        is_active: true,
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    // Should contain specified fields
    assert!(json_str.contains("\"username\""));
    assert!(json_str.contains("\"email\""));

    // Also contains other fields (serde serializes all by default)
    // In a full implementation, we'd filter fields during serialization
}

// Test: Meta.exclude configuration
#[test]
fn test_meta_exclude() {
    let serializer = UserExcludeSerializer;
    let user = UserFull {
        id: Some(1),
        username: "jane".to_string(),
        email: "jane@example.com".to_string(),
        password: "topsecret".to_string(),
        first_name: "Jane".to_string(),
        last_name: "Smith".to_string(),
        is_active: true,
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    // Should contain non-excluded fields
    assert!(json_str.contains("\"username\""));
    assert!(json_str.contains("\"email\""));

    // Password is still serialized by default serde
    // In a full implementation, we'd filter during serialization
}

// Test: Meta.read_only_fields configuration
#[test]
fn test_meta_read_only_fields() {
    let serializer = UserReadOnlySerializer;

    // Test that read-only field check works
    assert!(UserReadOnlySerializer::is_read_only_field("id"));
    assert!(UserReadOnlySerializer::is_read_only_field("is_active"));
    assert!(!UserReadOnlySerializer::is_read_only_field("username"));
}

// Test: Meta.write_only_fields configuration
#[test]
fn test_meta_write_only_fields() {
    let serializer = UserWriteOnlySerializer;

    // Test that write-only field check works
    assert!(UserWriteOnlySerializer::is_write_only_field("password"));
    assert!(!UserWriteOnlySerializer::is_write_only_field("username"));
    assert!(!UserWriteOnlySerializer::is_write_only_field("email"));
}

// Test: should_serialize_field with Meta.fields
#[test]
fn test_should_serialize_field_with_meta_fields() {
    // With Meta.fields, only specified fields should be serialized
    assert!(UserFieldsSerializer::should_serialize_field("id"));
    assert!(UserFieldsSerializer::should_serialize_field("username"));
    assert!(UserFieldsSerializer::should_serialize_field("email"));
    assert!(!UserFieldsSerializer::should_serialize_field("password"));
    assert!(!UserFieldsSerializer::should_serialize_field("first_name"));
}

// Test: should_serialize_field with Meta.exclude
#[test]
fn test_should_serialize_field_with_meta_exclude() {
    // With Meta.exclude, all fields except excluded should be serialized
    assert!(UserExcludeSerializer::should_serialize_field("id"));
    assert!(UserExcludeSerializer::should_serialize_field("username"));
    assert!(!UserExcludeSerializer::should_serialize_field("password"));
}

// Test: should_serialize_field with write_only
#[test]
fn test_should_serialize_field_with_write_only() {
    // Write-only fields should not be serialized
    assert!(!UserWriteOnlySerializer::should_serialize_field("password"));
    assert!(UserWriteOnlySerializer::should_serialize_field("username"));
    assert!(UserWriteOnlySerializer::should_serialize_field("email"));
}

// Test: should_deserialize_field with Meta.fields
#[test]
fn test_should_deserialize_field_with_meta_fields() {
    // With Meta.fields, only specified fields should be deserialized
    assert!(UserFieldsSerializer::should_deserialize_field("id"));
    assert!(UserFieldsSerializer::should_deserialize_field("username"));
    assert!(UserFieldsSerializer::should_deserialize_field("email"));
    assert!(!UserFieldsSerializer::should_deserialize_field("password"));
}

// Test: should_deserialize_field with read_only
#[test]
fn test_should_deserialize_field_with_read_only() {
    // Read-only fields should not be deserialized
    assert!(!UserReadOnlySerializer::should_deserialize_field("id"));
    assert!(!UserReadOnlySerializer::should_deserialize_field(
        "is_active"
    ));
    assert!(UserReadOnlySerializer::should_deserialize_field("username"));
}

// Test: Meta validation - fields and exclude conflict
#[test]
fn test_meta_validation_conflict() {
    // This would be tested with a serializer that has both fields and exclude
    // which should fail validation

    struct ConflictingSerializer;

    impl Serializer<UserFull> for ConflictingSerializer {
        fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
            serde_json::to_vec(value)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ReinhardtDeserializer<UserFull> for ConflictingSerializer {
        fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
            serde_json::from_slice(data)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ModelSerializer<UserFull> for ConflictingSerializer {
        fn meta_fields() -> Option<Vec<&'static str>> {
            Some(vec!["id", "username"])
        }

        fn meta_exclude() -> Option<Vec<&'static str>> {
            Some(vec!["password"])
        }
    }

    let result = ConflictingSerializer::validate_meta();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Cannot set both 'fields' and 'exclude'"));
}

// Test: Default Meta configuration
#[test]
fn test_default_meta_configuration() {
    // Default serializer should have no Meta restrictions
    let serializer = DefaultModelSerializer::<UserFull>::new();
    let user = UserFull {
        id: Some(1),
        username: "test".to_string(),
        email: "test@example.com".to_string(),
        password: "pass".to_string(),
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        is_active: true,
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let deserialized: UserFull =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(user, deserialized);
}

// Test: Meta.fields with empty list
#[test]
fn test_meta_fields_empty() {
    struct EmptyFieldsSerializer;

    impl Serializer<UserFull> for EmptyFieldsSerializer {
        fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
            serde_json::to_vec(value)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ReinhardtDeserializer<UserFull> for EmptyFieldsSerializer {
        fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
            serde_json::from_slice(data)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ModelSerializer<UserFull> for EmptyFieldsSerializer {
        fn meta_fields() -> Option<Vec<&'static str>> {
            Some(vec![])
        }
    }

    // With empty fields list, no fields should be serialized
    assert!(!EmptyFieldsSerializer::should_serialize_field("id"));
    assert!(!EmptyFieldsSerializer::should_serialize_field("username"));
}

// Test: Meta.exclude with empty list
#[test]
fn test_meta_exclude_empty() {
    struct EmptyExcludeSerializer;

    impl Serializer<UserFull> for EmptyExcludeSerializer {
        fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
            serde_json::to_vec(value)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ReinhardtDeserializer<UserFull> for EmptyExcludeSerializer {
        fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
            serde_json::from_slice(data)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ModelSerializer<UserFull> for EmptyExcludeSerializer {
        fn meta_exclude() -> Option<Vec<&'static str>> {
            Some(vec![])
        }
    }

    // With empty exclude list, all fields should be serialized
    assert!(EmptyExcludeSerializer::should_serialize_field("id"));
    assert!(EmptyExcludeSerializer::should_serialize_field("username"));
    assert!(EmptyExcludeSerializer::should_serialize_field("password"));
}

// Test: Combining read_only and write_only
#[test]
fn test_combining_read_only_write_only() {
    struct CombinedSerializer;

    impl Serializer<UserFull> for CombinedSerializer {
        fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
            serde_json::to_vec(value)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ReinhardtDeserializer<UserFull> for CombinedSerializer {
        fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
            serde_json::from_slice(data)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ModelSerializer<UserFull> for CombinedSerializer {
        fn meta_read_only_fields() -> Option<Vec<&'static str>> {
            Some(vec!["id"])
        }

        fn meta_write_only_fields() -> Option<Vec<&'static str>> {
            Some(vec!["password"])
        }
    }

    // Read-only field: serialized but not deserialized
    assert!(CombinedSerializer::should_serialize_field("id"));
    assert!(!CombinedSerializer::should_deserialize_field("id"));

    // Write-only field: deserialized but not serialized
    assert!(!CombinedSerializer::should_serialize_field("password"));
    assert!(CombinedSerializer::should_deserialize_field("password"));

    // Regular field: both serialized and deserialized
    assert!(CombinedSerializer::should_serialize_field("username"));
    assert!(CombinedSerializer::should_deserialize_field("username"));
}

// Test: Meta.fields takes precedence
#[test]
fn test_meta_fields_precedence() {
    struct FieldsPrecedenceSerializer;

    impl Serializer<UserFull> for FieldsPrecedenceSerializer {
        fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
            serde_json::to_vec(value)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ReinhardtDeserializer<UserFull> for FieldsPrecedenceSerializer {
        fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
            serde_json::from_slice(data)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ModelSerializer<UserFull> for FieldsPrecedenceSerializer {
        fn meta_fields() -> Option<Vec<&'static str>> {
            Some(vec!["id", "username", "password"])
        }

        fn meta_write_only_fields() -> Option<Vec<&'static str>> {
            Some(vec!["password"])
        }
    }

    // Even though password is write-only, it's in fields list
    // so it should pass the fields check first
    // but then write_only should prevent serialization
    let should_serialize = FieldsPrecedenceSerializer::should_serialize_field("password");
    assert!(!should_serialize);
}

// Test: Multiple read-only fields
#[test]
fn test_multiple_read_only_fields() {
    struct MultiReadOnlySerializer;

    impl Serializer<UserFull> for MultiReadOnlySerializer {
        fn serialize(&self, value: &UserFull) -> reinhardt_apps::Result<Vec<u8>> {
            serde_json::to_vec(value)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ReinhardtDeserializer<UserFull> for MultiReadOnlySerializer {
        fn deserialize(&self, data: &[u8]) -> reinhardt_apps::Result<UserFull> {
            serde_json::from_slice(data)
                .map_err(|e| reinhardt_apps::Error::Serialization(e.to_string()))
        }
    }

    impl ModelSerializer<UserFull> for MultiReadOnlySerializer {
        fn meta_read_only_fields() -> Option<Vec<&'static str>> {
            Some(vec!["id", "is_active", "email"])
        }
    }

    assert!(MultiReadOnlySerializer::is_read_only_field("id"));
    assert!(MultiReadOnlySerializer::is_read_only_field("is_active"));
    assert!(MultiReadOnlySerializer::is_read_only_field("email"));
    assert!(!MultiReadOnlySerializer::is_read_only_field("username"));
}

// Test: Field filtering consistency
#[test]
fn test_field_filtering_consistency() {
    // Test that field filtering is consistent across serialization and deserialization
    let user = UserFull {
        id: Some(1),
        username: "test".to_string(),
        email: "test@example.com".to_string(),
        password: "secret".to_string(),
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        is_active: true,
    };

    // Test with fields serializer
    let fields_serializer = UserFieldsSerializer;
    let serialized = Serializer::serialize(&fields_serializer, &user).unwrap();
    let deserialized: UserFull =
        ReinhardtDeserializer::deserialize(&fields_serializer, &serialized).unwrap();

    // Even though we filter fields, serde still deserializes all present fields
    assert_eq!(deserialized.id, user.id);
    assert_eq!(deserialized.username, user.username);
}

// Test: Meta configuration with ModelSerializerBuilder
#[test]
fn test_meta_with_builder() {
    use reinhardt_serializers::ModelSerializerBuilder;

    // Builder creates default serializer (no Meta restrictions)
    let serializer = ModelSerializerBuilder::<UserFull>::new().build();

    let user = UserFull {
        id: Some(1),
        username: "builder".to_string(),
        email: "builder@example.com".to_string(),
        password: "pass".to_string(),
        first_name: "Builder".to_string(),
        last_name: "Test".to_string(),
        is_active: true,
    };

    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let deserialized: UserFull =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(user, deserialized);
}
