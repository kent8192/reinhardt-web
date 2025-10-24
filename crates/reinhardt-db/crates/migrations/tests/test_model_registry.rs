//! Unit tests for ModelRegistry and ModelMetadata

use reinhardt_migrations::model_registry::{FieldMetadata, ModelMetadata, ModelRegistry};
use std::collections::HashMap;

#[test]
fn test_model_metadata_creation() {
    let metadata = ModelMetadata::new("test_app", "User", "test_app_user");

    assert_eq!(metadata.app_label, "test_app");
    assert_eq!(metadata.model_name, "User");
    assert_eq!(metadata.table_name, "test_app_user");
    assert!(metadata.fields.is_empty());
    assert!(metadata.options.is_empty());
}

#[test]
fn test_model_metadata_add_field() {
    let mut metadata = ModelMetadata::new("test_app", "User", "test_app_user");

    let field = FieldMetadata::new("CharField")
        .with_param("max_length", "100")
        .with_nullable(false);

    metadata.add_field("username".to_string(), field);

    assert_eq!(metadata.fields.len(), 1);
    assert!(metadata.fields.contains_key("username"));

    let username_field = &metadata.fields["username"];
    assert_eq!(username_field.field_type, "CharField");
    assert!(!username_field.nullable);
}

#[test]
fn test_model_metadata_set_option() {
    let mut metadata = ModelMetadata::new("test_app", "User", "test_app_user");

    metadata.set_option("verbose_name".to_string(), "Test User".to_string());
    metadata.set_option("ordering".to_string(), "created_at".to_string());

    assert_eq!(metadata.options.len(), 2);
    assert_eq!(metadata.options["verbose_name"], "Test User");
    assert_eq!(metadata.options["ordering"], "created_at");
}

#[test]
fn test_field_metadata_creation() {
    let field = FieldMetadata::new("IntegerField");

    assert_eq!(field.field_type, "IntegerField");
    assert!(field.nullable);
    assert!(field.params.is_empty());
}

#[test]
fn test_field_metadata_with_params() {
    let field = FieldMetadata::new("CharField")
        .with_param("max_length", "255")
        .with_param("blank", "false")
        .with_nullable(false);

    assert_eq!(field.field_type, "CharField");
    assert!(!field.nullable);
    assert_eq!(field.params.len(), 2);
    assert_eq!(field.params["max_length"], "255");
    assert_eq!(field.params["blank"], "false");
}

#[test]
fn test_model_registry_register() {
    let registry = ModelRegistry::new();

    let metadata = ModelMetadata::new("test_app", "User", "test_app_user");
    registry.register(metadata);

    let models = registry.get_models();
    assert_eq!(models.len(), 1);

    let user_model = &models[0];
    assert_eq!(user_model.app_label, "test_app");
    assert_eq!(user_model.model_name, "User");
    assert_eq!(user_model.table_name, "test_app_user");
}

#[test]
fn test_model_registry_multiple_models() {
    let registry = ModelRegistry::new();

    let user_metadata = ModelMetadata::new("test_app", "User", "test_app_user");
    let post_metadata = ModelMetadata::new("test_app", "Post", "test_app_post");

    registry.register(user_metadata);
    registry.register(post_metadata);

    let models = registry.get_models();
    assert_eq!(models.len(), 2);

    let model_names: Vec<&str> = models.iter().map(|m| m.model_name.as_str()).collect();
    assert!(model_names.contains(&"User"));
    assert!(model_names.contains(&"Post"));
}

#[test]
fn test_model_registry_clear() {
    let registry = ModelRegistry::new();

    let metadata = ModelMetadata::new("test_app", "User", "test_app_user");
    registry.register(metadata);

    assert_eq!(registry.get_models().len(), 1);

    registry.clear();

    assert_eq!(registry.get_models().len(), 0);
}

#[test]
fn test_model_metadata_to_model_state() {
    let mut metadata = ModelMetadata::new("test_app", "User", "test_app_user");

    // Add fields
    let id_field = FieldMetadata::new("IntegerField")
        .with_param("primary_key", "true")
        .with_nullable(false);
    metadata.add_field("id".to_string(), id_field);

    let username_field = FieldMetadata::new("CharField")
        .with_param("max_length", "100")
        .with_nullable(false);
    metadata.add_field("username".to_string(), username_field);

    // Convert to ModelState
    let model_state = metadata.to_model_state();

    assert_eq!(model_state.app_label, "test_app");
    assert_eq!(model_state.name, "User");
    assert!(model_state.has_field("id"));
    assert!(model_state.has_field("username"));

    let id_field_state = model_state.get_field("id").unwrap();
    assert_eq!(id_field_state.name, "id");
    assert_eq!(id_field_state.field_type, "IntegerField");
}

#[test]
fn test_field_metadata_complex_params() {
    let field = FieldMetadata::new("ForeignKey")
        .with_param("to", "User")
        .with_param("on_delete", "CASCADE")
        .with_param("related_name", "posts")
        .with_nullable(true);

    assert_eq!(field.field_type, "ForeignKey");
    assert!(field.nullable);
    assert_eq!(field.params.len(), 3);
    assert_eq!(field.params["to"], "User");
    assert_eq!(field.params["on_delete"], "CASCADE");
    assert_eq!(field.params["related_name"], "posts");
}

#[test]
fn test_model_metadata_with_multiple_fields() {
    let mut metadata = ModelMetadata::new("blog", "Article", "blog_article");

    // Add multiple fields with different types
    metadata.add_field(
        "id".to_string(),
        FieldMetadata::new("AutoField").with_param("primary_key", "true"),
    );

    metadata.add_field(
        "title".to_string(),
        FieldMetadata::new("CharField")
            .with_param("max_length", "200")
            .with_nullable(false),
    );

    metadata.add_field(
        "content".to_string(),
        FieldMetadata::new("TextField").with_nullable(false),
    );

    metadata.add_field(
        "published_at".to_string(),
        FieldMetadata::new("DateTimeField").with_nullable(true),
    );

    metadata.add_field(
        "is_published".to_string(),
        FieldMetadata::new("BooleanField")
            .with_param("default", "false")
            .with_nullable(false),
    );

    assert_eq!(metadata.fields.len(), 5);

    // Verify each field
    assert!(metadata.fields.contains_key("id"));
    assert!(metadata.fields.contains_key("title"));
    assert!(metadata.fields.contains_key("content"));
    assert!(metadata.fields.contains_key("published_at"));
    assert!(metadata.fields.contains_key("is_published"));

    // Check nullable properties
    assert!(!metadata.fields["title"].nullable);
    assert!(!metadata.fields["content"].nullable);
    assert!(metadata.fields["published_at"].nullable);
    assert!(!metadata.fields["is_published"].nullable);
}
