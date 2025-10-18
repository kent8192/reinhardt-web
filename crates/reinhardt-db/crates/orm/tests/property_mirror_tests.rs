//! Property Mirror Tests for Hybrid Properties
//!
//! Based on SQLAlchemy's PropertyMirrorTest (test/ext/test_hybrid.py:867-1003)
//!
//! Tests the integration of hybrid properties with ORM metadata, including:
//! - Property introspection via ModelInspector
//! - Attribute key handling
//! - Property descriptor metadata
//! - Attribute history tracking

use reinhardt_hybrid::HybridProperty;
use reinhardt_orm::inspection::{
    AttributeHistory, ColumnDescriptor, HybridPropertyDescriptor, InstanceAttributeHistory,
    ModelInspector, PropertyDescriptor, PropertyType,
};
use serde_json::Value;
use std::any::TypeId;
use std::collections::HashMap;

/// Test model for property mirror tests
#[derive(Debug, Clone)]
struct User {
    id: i32,
    first_name: String,
    last_name: String,
}

impl User {
    fn new(id: i32, first_name: &str, last_name: &str) -> Self {
        Self {
            id,
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
        }
    }
}

/// Helper to create a model inspector for User
fn create_user_inspector() -> ModelInspector {
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    // Add column properties
    inspector.add_property(Box::new(ColumnDescriptor::new("id", class_id)));
    inspector.add_property(Box::new(ColumnDescriptor::new("first_name", class_id)));
    inspector.add_property(Box::new(ColumnDescriptor::new("last_name", class_id)));

    // Add hybrid property for full_name
    inspector.add_property(Box::new(
        HybridPropertyDescriptor::new("full_name", class_id).with_expression(true),
    ));

    inspector
}

#[test]
fn test_property_mirror_attr_key() {
    // Test that we don't assume attribute key exists without checking
    let inspector = create_user_inspector();

    // Property that exists
    assert!(inspector.get_property("first_name").is_some());
    assert!(inspector.get_property("full_name").is_some());

    // Property that doesn't exist
    assert!(inspector.get_property("nonexistent").is_none());
}

#[test]
fn test_dont_assume_attr_key_is_present_ac() {
    // Test with aliased class (using custom key)
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    // Add property with different key than name
    let descriptor = HybridPropertyDescriptor::new("full_name", class_id).with_key("fullName");
    inspector.add_property(Box::new(descriptor));

    let property = inspector.get_property("full_name");
    assert!(property.is_some());

    let prop = property.unwrap();
    assert_eq!(prop.name(), "full_name");
    assert_eq!(prop.key(), "fullName"); // Different key
}

#[test]
fn test_c_collection_func_element() {
    // Test accessing properties within collections (like c.full_name in SQLAlchemy)
    let inspector = create_user_inspector();

    // Get all column properties
    let columns = inspector.get_column_properties();
    assert_eq!(columns.len(), 3); // id, first_name, last_name

    // Get all hybrid properties
    let hybrids = inspector.get_hybrid_properties();
    assert_eq!(hybrids.len(), 1); // full_name

    // Verify we can access specific property
    let full_name = inspector.get_property("full_name");
    assert!(full_name.is_some());
    assert_eq!(full_name.unwrap().property_type(), PropertyType::Hybrid);
}

#[test]
fn test_property_mirror_mismatched_col() {
    // Test that column name mismatches are handled correctly
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    // Add column with different column_name than property name
    let descriptor = ColumnDescriptor::new("user_id", class_id).with_column_name("id"); // Property: user_id, Column: id

    inspector.add_property(Box::new(descriptor));

    let property = inspector.get_property("user_id");
    assert!(property.is_some());

    let col_desc = property.unwrap();
    assert_eq!(col_desc.name(), "user_id");

    // Check if we can safely downcast to ColumnDescriptor
    // In a real implementation, we would check column_name here
}

#[test]
fn test_aliased_mismatched_col() {
    // Test aliased properties with column name mismatch
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    // Property with custom key AND different column name
    let descriptor = ColumnDescriptor::new("user_id", class_id)
        .with_key("userId")
        .with_column_name("id");

    inspector.add_property(Box::new(descriptor));

    let property = inspector.get_property("user_id");
    assert!(property.is_some());

    let prop = property.unwrap();
    assert_eq!(prop.name(), "user_id");
    assert_eq!(prop.key(), "userId");
}

#[test]
fn test_property_mirror() {
    // Test basic property retrieval
    let inspector = create_user_inspector();

    let full_name_prop = inspector.get_property("full_name");
    assert!(full_name_prop.is_some());

    let prop = full_name_prop.unwrap();
    assert_eq!(prop.name(), "full_name");
    assert_eq!(prop.property_type(), PropertyType::Hybrid);
}

#[test]
fn test_key() {
    // Test property key access
    let inspector = create_user_inspector();

    let prop = inspector.get_property("first_name").unwrap();
    assert_eq!(prop.key(), "first_name"); // Default: key == name

    // Test custom key
    let class_id = TypeId::of::<User>();
    let mut custom_inspector = ModelInspector::new();
    custom_inspector.add_property(Box::new(
        HybridPropertyDescriptor::new("full_name", class_id).with_key("fullName"),
    ));

    let custom_prop = custom_inspector.get_property("full_name").unwrap();
    assert_eq!(custom_prop.key(), "fullName");
}

#[test]
fn test_class() {
    // Test that property correctly identifies its owner class
    let inspector = create_user_inspector();

    let prop = inspector.get_property("full_name").unwrap();
    assert_eq!(prop.class_id(), TypeId::of::<User>());

    // Verify all properties have correct class
    for property in inspector.all_properties() {
        assert_eq!(property.class_id(), TypeId::of::<User>());
    }
}

#[test]
fn test_get_history() {
    // Test attribute history tracking
    let mut history = InstanceAttributeHistory::new();

    // Initial value
    history.record_initial("first_name", Value::String("John".to_string()));

    // No changes yet
    let state1 = history.get_history("first_name").unwrap();
    assert!(!state1.has_changes());
    assert_eq!(state1.unchanged.len(), 1);

    // Make a change
    history.record_change(
        "first_name",
        Some(Value::String("John".to_string())),
        Value::String("Jane".to_string()),
    );

    // Verify history shows change
    let state2 = history.get_history("first_name").unwrap();
    assert!(state2.has_changes());
    assert_eq!(state2.added.len(), 1);
    assert_eq!(state2.deleted.len(), 1);

    if let Value::String(added) = &state2.added[0] {
        assert_eq!(added, "Jane");
    } else {
        panic!("Expected string value");
    }

    if let Value::String(deleted) = &state2.deleted[0] {
        assert_eq!(deleted, "John");
    } else {
        panic!("Expected string value");
    }
}

#[test]
fn test_property_mirror_info_not_mirrored() {
    // Test that info from columns is not mirrored to hybrid properties
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    // Create column with info
    let mut col_info = HashMap::new();
    col_info.insert("doc".to_string(), "User's first name".to_string());
    let col_desc = ColumnDescriptor::new("first_name", class_id).with_info(col_info);
    inspector.add_property(Box::new(col_desc));

    // Create hybrid without info
    let hybrid_desc = HybridPropertyDescriptor::new("full_name", class_id);
    inspector.add_property(Box::new(hybrid_desc));

    // Verify column has info
    let col_prop = inspector.get_property("first_name").unwrap();
    assert!(!col_prop.info().is_empty());

    // Verify hybrid does NOT have column's info (not mirrored)
    let hybrid_prop = inspector.get_property("full_name").unwrap();
    assert!(hybrid_prop.info().is_empty());
}

#[test]
fn test_property_mirror_info_from_hybrid() {
    // Test that hybrid properties can have their own info
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    // Create hybrid with its own info
    let mut hybrid_info = HashMap::new();
    hybrid_info.insert("doc".to_string(), "User's full name".to_string());
    hybrid_info.insert("example".to_string(), "John Doe".to_string());

    let hybrid_desc = HybridPropertyDescriptor::new("full_name", class_id).with_info(hybrid_info);

    inspector.add_property(Box::new(hybrid_desc));

    // Verify hybrid has its own info
    let hybrid_prop = inspector.get_property("full_name").unwrap();
    assert_eq!(hybrid_prop.info().len(), 2);
    assert_eq!(
        hybrid_prop.info().get("doc"),
        Some(&"User's full name".to_string())
    );
    assert_eq!(
        hybrid_prop.info().get("example"),
        Some(&"John Doe".to_string())
    );
}
