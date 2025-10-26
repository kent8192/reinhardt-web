//! Integration tests for nested serializers and recursive serialization

use reinhardt_orm::Model;
use reinhardt_serializers::nested_config::{NestedFieldConfig, NestedSerializerConfig};
use reinhardt_serializers::recursive::{circular, depth, SerializationContext};
use reinhardt_serializers::ModelSerializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
    id: Option<i64>,
    title: String,
    content: String,
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

#[test]
fn test_nested_field_config_basic() {
    let config = NestedFieldConfig::new("author");
    assert_eq!(config.field_name, "author");
    assert_eq!(config.depth, 1);
    assert!(!config.read_only);
    assert!(!config.allow_create);
    assert!(!config.allow_update);
}

#[test]
fn test_nested_field_config_with_depth() {
    let config = NestedFieldConfig::new("author").depth(3);
    assert_eq!(config.depth, 3);
}

#[test]
fn test_nested_field_config_read_only() {
    let config = NestedFieldConfig::new("author").read_only();
    assert!(config.read_only);
}

#[test]
fn test_nested_field_config_writable() {
    let config = NestedFieldConfig::new("author").writable();
    assert!(config.allow_create);
    assert!(config.allow_update);
}

#[test]
fn test_nested_serializer_config() {
    let mut config = NestedSerializerConfig::new();
    assert_eq!(config.nested_field_names().len(), 0);

    config.add_nested_field(NestedFieldConfig::new("author").depth(2));
    assert!(config.is_nested_field("author"));
    assert_eq!(config.get_depth("author"), Some(2));

    config.add_nested_field(NestedFieldConfig::new("category"));
    assert!(config.is_nested_field("category"));
    assert_eq!(config.nested_field_names().len(), 2);
}

#[test]
fn test_model_serializer_with_nested_field() {
    let serializer =
        ModelSerializer::<Post>::new().with_nested_field(NestedFieldConfig::new("author").depth(2));

    assert!(serializer.is_nested_field("author"));
    assert!(!serializer.is_nested_field("title"));

    let config = serializer.nested_config();
    assert_eq!(config.get_depth("author"), Some(2));
}

#[test]
fn test_model_serializer_multiple_nested_fields() {
    let serializer = ModelSerializer::<Post>::new()
        .with_nested_field(NestedFieldConfig::new("author").read_only())
        .with_nested_field(NestedFieldConfig::new("category").writable());

    assert!(serializer.is_nested_field("author"));
    assert!(serializer.is_nested_field("category"));

    let author_config = serializer.nested_config().get_nested_field("author");
    assert!(author_config.is_some());
    assert!(author_config.unwrap().read_only);

    let category_config = serializer.nested_config().get_nested_field("category");
    assert!(category_config.is_some());
    assert!(category_config.unwrap().allow_create);
    assert!(category_config.unwrap().allow_update);
}

#[test]
fn test_serialization_context_basic() {
    let context = SerializationContext::new(3);
    assert_eq!(context.current_depth(), 0);
    assert_eq!(context.max_depth(), 3);
    assert_eq!(context.remaining_depth(), 3);
    assert!(context.can_go_deeper());
}

#[test]
fn test_serialization_context_child() {
    let context = SerializationContext::new(3);
    let child = context.child();

    assert_eq!(child.current_depth(), 1);
    assert_eq!(child.remaining_depth(), 2);
    assert!(child.can_go_deeper());
}

#[test]
fn test_serialization_context_max_depth() {
    let context = SerializationContext::new(2);
    let child1 = context.child();
    let child2 = child1.child();

    assert_eq!(child2.current_depth(), 2);
    assert_eq!(child2.remaining_depth(), 0);
    assert!(!child2.can_go_deeper());
}

#[test]
fn test_circular_reference_detection() {
    let mut context = SerializationContext::new(5);

    assert!(!circular::would_be_circular(&context, "user:1"));

    context.mark_visited("user:1".to_string());
    assert!(circular::would_be_circular(&context, "user:1"));
    assert!(!circular::would_be_circular(&context, "user:2"));
}

#[test]
fn test_circular_try_visit() {
    let mut context = SerializationContext::new(5);

    assert!(circular::try_visit(&mut context, "user:1").is_ok());
    assert!(context.is_visited("user:1"));

    assert!(circular::try_visit(&mut context, "user:1").is_err());
}

#[test]
fn test_circular_visit_with_cleanup() {
    let mut context = SerializationContext::new(5);

    let result = circular::visit_with(&mut context, "user:1", |ctx| {
        assert!(ctx.is_visited("user:1"));
        Ok(42)
    });

    assert_eq!(result.unwrap(), 42);
    assert!(!context.is_visited("user:1"));
}

#[test]
fn test_depth_management() {
    let context = SerializationContext::new(2);
    assert!(depth::can_descend(&context));

    let child = depth::try_descend(&context).unwrap();
    assert_eq!(child.current_depth(), 1);
    assert!(depth::can_descend(&child));

    let grandchild = depth::try_descend(&child).unwrap();
    assert_eq!(grandchild.current_depth(), 2);
    assert!(!depth::can_descend(&grandchild));

    assert!(depth::try_descend(&grandchild).is_err());
}

#[test]
fn test_depth_descend_with() {
    let context = SerializationContext::new(3);

    let result = depth::descend_with(&context, |child_ctx| {
        assert_eq!(child_ctx.current_depth(), 1);
        assert_eq!(child_ctx.max_depth(), 3);

        depth::descend_with(child_ctx, |grandchild_ctx| {
            assert_eq!(grandchild_ctx.current_depth(), 2);
            Ok(())
        })
    });

    assert!(result.is_ok());
}

#[test]
fn test_combined_depth_and_circular_detection() {
    let mut context = SerializationContext::new(3);

    circular::visit_with(&mut context, "post:1", |ctx| {
        assert_eq!(ctx.current_depth(), 0);
        assert!(ctx.is_visited("post:1"));

        let child = ctx.child();
        assert_eq!(child.current_depth(), 1);
        assert!(child.is_visited("post:1"));

        Ok(())
    })
    .unwrap();

    assert!(!context.is_visited("post:1"));
}

#[test]
fn test_model_serializer_with_nested_and_meta() {
    let serializer = ModelSerializer::<Post>::new()
        .with_fields(vec![
            "id".to_string(),
            "title".to_string(),
            "author".to_string(),
        ])
        .with_nested_field(NestedFieldConfig::new("author").depth(2))
        .with_read_only_fields(vec!["id".to_string()]);

    assert!(serializer.meta().is_field_included("id"));
    assert!(serializer.meta().is_field_included("title"));
    assert!(serializer.meta().is_field_included("author"));
    assert!(!serializer.meta().is_field_included("content"));

    assert!(serializer.meta().is_read_only("id"));
    assert!(!serializer.meta().is_read_only("title"));

    assert!(serializer.is_nested_field("author"));
    assert_eq!(serializer.nested_config().get_depth("author"), Some(2));
}
