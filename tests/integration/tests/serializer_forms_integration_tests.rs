// Forms integration tests for Serializers
// Tests integration between reinhardt-serializers and reinhardt-forms
// Note: Many tests are marked #[ignore] due to incomplete Forms API

use reinhardt_orm::Model;
use reinhardt_serializers::{
    DefaultModelSerializer, Deserializer as ReinhardtDeserializer, JsonSerializer, ModelSerializer,
    Serializer,
};
use serde::{Deserialize, Serialize};

// Test models

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct User {
    id: Option<i64>,
    username: String,
    email: String,
    is_active: bool,
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

// Test: Basic serializer functionality (always works)
#[test]
fn test_serializer_basic_functionality() {
    let user = User {
        id: Some(1),
        username: "john".to_string(),
        email: "john@example.com".to_string(),
        is_active: true,
    };

    let serializer = DefaultModelSerializer::<User>::new();
    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    assert!(!serialized.is_empty());
}

// Test: Form to serializer data flow
#[test]
fn test_form_to_serializer_data_flow() {
    // Simulate validated form data
    #[derive(Serialize, Deserialize)]
    struct FormData {
        username: String,
        email: String,
        is_active: bool,
    }

    let form_data = FormData {
        username: "john_doe".to_string(),
        email: "john@example.com".to_string(),
        is_active: true,
    };

    // Serialize form data
    let serializer = JsonSerializer::<FormData>::new();
    let serialized = Serializer::serialize(&serializer, &form_data).unwrap();

    // Deserialize and create model
    let deserialized: FormData =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(form_data.username, deserialized.username);
    assert_eq!(form_data.email, deserialized.email);
}

// Test: Form errors serialization
#[test]
fn test_form_errors_serialization() {
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct FormErrors {
        errors: HashMap<String, Vec<String>>,
    }

    let mut errors_map = HashMap::new();
    errors_map.insert(
        "username".to_string(),
        vec!["This field is required".to_string()],
    );
    errors_map.insert(
        "email".to_string(),
        vec!["Enter a valid email address".to_string()],
    );

    let form_errors = FormErrors { errors: errors_map };

    let serializer = JsonSerializer::<FormErrors>::new();
    let serialized = Serializer::serialize(&serializer, &form_errors).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("username"));
    assert!(json_str.contains("email"));
}

// Test: Nested form serialization
#[test]
fn test_nested_form_serialization() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Address {
        street: String,
        city: String,
        zipcode: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct UserWithAddress {
        username: String,
        email: String,
        address: Address,
    }

    let user = UserWithAddress {
        username: "john".to_string(),
        email: "john@example.com".to_string(),
        address: Address {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zipcode: "12345".to_string(),
        },
    };

    let serializer = JsonSerializer::<UserWithAddress>::new();
    let serialized = Serializer::serialize(&serializer, &user).unwrap();
    let json_str = String::from_utf8(serialized.clone()).unwrap();

    assert!(json_str.contains("address"));
    assert!(json_str.contains("123 Main St"));

    let deserialized: UserWithAddress =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(user, deserialized);
}

// Test: Form data to model workflow
#[test]
fn test_form_data_to_model_workflow() {
    // Simulate form submission
    let form_data = User {
        id: None,
        username: "newuser".to_string(),
        email: "newuser@example.com".to_string(),
        is_active: false,
    };

    // Create using ModelSerializer
    let serializer = DefaultModelSerializer::<User>::new();
    let created = serializer.create(form_data.clone()).unwrap();

    // Serialize created instance
    let serialized = Serializer::serialize(&serializer, &created).unwrap();
    let deserialized: User = ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();

    assert_eq!(form_data, deserialized);
}

// Test: Validation state serialization
#[test]
fn test_validation_state_serialization() {
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct ValidationState {
        is_valid: bool,
        errors: HashMap<String, Vec<String>>,
    }

    // Valid state
    let valid_state = ValidationState {
        is_valid: true,
        errors: HashMap::new(),
    };

    let serializer = JsonSerializer::<ValidationState>::new();
    let serialized = Serializer::serialize(&serializer, &valid_state).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("\"is_valid\":true"));

    // Invalid state
    let mut errors = HashMap::new();
    errors.insert("username".to_string(), vec!["Required".to_string()]);

    let invalid_state = ValidationState {
        is_valid: false,
        errors,
    };

    let serialized = Serializer::serialize(&serializer, &invalid_state).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("\"is_valid\":false"));
    assert!(json_str.contains("Required"));
}
