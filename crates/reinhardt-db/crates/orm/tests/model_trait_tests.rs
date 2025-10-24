//! Unit tests for Model trait implementation

use reinhardt_orm::{FieldInfo, Model};

// Test struct that implements Model trait manually
struct TestUser {
    id: Option<i32>,
    username: String,
    email: Option<String>,
}

impl Model for TestUser {
    type PrimaryKey = i32;

    fn table_name() -> &'static str {
        "test_users"
    }

    fn app_label() -> &'static str {
        "test_app"
    }

    fn primary_key_field() -> &'static str {
        "id"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }

    fn field_metadata() -> Vec<FieldInfo> {
        vec![
            FieldInfo {
                name: "id".to_string(),
                field_type: "reinhardt.orm.models.IntegerField".to_string(),
                nullable: true,
                primary_key: true,
                attributes: std::collections::HashMap::new(),
            },
            FieldInfo {
                name: "username".to_string(),
                field_type: "reinhardt.orm.models.CharField".to_string(),
                nullable: false,
                primary_key: false,
                attributes: {
                    let mut attrs = std::collections::HashMap::new();
                    attrs.insert("max_length".to_string(), "100".to_string());
                    attrs
                },
            },
            FieldInfo {
                name: "email".to_string(),
                field_type: "reinhardt.orm.models.CharField".to_string(),
                nullable: true,
                primary_key: false,
                attributes: {
                    let mut attrs = std::collections::HashMap::new();
                    attrs.insert("max_length".to_string(), "255".to_string());
                    attrs
                },
            },
        ]
    }
}

#[test]
fn test_model_table_name() {
    assert_eq!(TestUser::table_name(), "test_users");
}

#[test]
fn test_model_app_label() {
    assert_eq!(TestUser::app_label(), "test_app");
}

#[test]
fn test_model_primary_key_field() {
    assert_eq!(TestUser::primary_key_field(), "id");
}

#[test]
fn test_model_field_metadata() {
    let fields = TestUser::field_metadata();
    assert_eq!(fields.len(), 3);

    // Check id field
    let id_field = &fields[0];
    assert_eq!(id_field.name, "id");
    assert_eq!(id_field.field_type, "reinhardt.orm.models.IntegerField");
    assert!(id_field.nullable);
    assert!(id_field.primary_key);

    // Check username field
    let username_field = &fields[1];
    assert_eq!(username_field.name, "username");
    assert_eq!(username_field.field_type, "reinhardt.orm.models.CharField");
    assert!(!username_field.nullable);
    assert!(!username_field.primary_key);
    assert_eq!(username_field.attributes.get("max_length").unwrap(), "100");

    // Check email field
    let email_field = &fields[2];
    assert_eq!(email_field.name, "email");
    assert_eq!(email_field.field_type, "reinhardt.orm.models.CharField");
    assert!(email_field.nullable);
    assert!(!email_field.primary_key);
    assert_eq!(email_field.attributes.get("max_length").unwrap(), "255");
}

#[test]
fn test_model_primary_key_access() {
    let mut user = TestUser {
        id: None,
        username: "testuser".to_string(),
        email: Some("test@example.com".to_string()),
    };

    // Initially no primary key
    assert!(user.primary_key().is_none());

    // Set primary key
    user.set_primary_key(42);
    assert_eq!(user.primary_key(), Some(&42));
    assert_eq!(user.id, Some(42));
}

#[test]
fn test_model_with_existing_pk() {
    let user = TestUser {
        id: Some(100),
        username: "anotheruser".to_string(),
        email: None,
    };

    assert_eq!(user.primary_key(), Some(&100));
}

#[test]
fn test_model_update_primary_key() {
    let mut user = TestUser {
        id: Some(1),
        username: "user1".to_string(),
        email: None,
    };

    assert_eq!(user.primary_key(), Some(&1));

    // Update primary key
    user.set_primary_key(999);
    assert_eq!(user.primary_key(), Some(&999));
    assert_eq!(user.id, Some(999));
}

// Test struct with non-Option primary key
struct TestPost {
    id: i64,
    title: String,
}

impl Model for TestPost {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "test_posts"
    }

    fn app_label() -> &'static str {
        "test_app"
    }

    fn primary_key_field() -> &'static str {
        "id"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        Some(&self.id)
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = value;
    }

    fn field_metadata() -> Vec<FieldInfo> {
        vec![
            FieldInfo {
                name: "id".to_string(),
                field_type: "reinhardt.orm.models.BigIntegerField".to_string(),
                nullable: false,
                primary_key: true,
                attributes: std::collections::HashMap::new(),
            },
            FieldInfo {
                name: "title".to_string(),
                field_type: "reinhardt.orm.models.CharField".to_string(),
                nullable: false,
                primary_key: false,
                attributes: {
                    let mut attrs = std::collections::HashMap::new();
                    attrs.insert("max_length".to_string(), "200".to_string());
                    attrs
                },
            },
        ]
    }
}

#[test]
fn test_non_option_primary_key() {
    let mut post = TestPost {
        id: 42,
        title: "Test Post".to_string(),
    };

    // Primary key is always Some for non-Option types
    assert_eq!(post.primary_key(), Some(&42));

    // Update primary key
    post.set_primary_key(100);
    assert_eq!(post.primary_key(), Some(&100));
    assert_eq!(post.id, 100);
}

#[test]
fn test_non_option_primary_key_metadata() {
    let fields = TestPost::field_metadata();
    assert_eq!(fields.len(), 2);

    let id_field = &fields[0];
    assert_eq!(id_field.name, "id");
    assert!(!id_field.nullable); // Non-Option type should not be nullable
    assert!(id_field.primary_key);
}
