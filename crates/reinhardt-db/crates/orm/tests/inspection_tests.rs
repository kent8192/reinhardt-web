//! Inspection and Metadata Tests for Hybrid Properties
//!
//! Based on SQLAlchemy's PropertyMirrorTest from test_hybrid.py

use reinhardt_orm::inspection::{
    ColumnDescriptor, HybridPropertyDescriptor, ModelInspector, PropertyDescriptor, PropertyType,
};
use std::any::TypeId;
use std::collections::HashMap;

#[derive(Debug)]
struct User {
    id: i32,
    first_name: String,
    last_name: String,
}

// Test 1: Property name and type attribute inspection
#[test]
fn test_name_and_type_attribute_inspection() {
    let class_id = TypeId::of::<User>();
    let descriptor = HybridPropertyDescriptor::new("full_name", class_id);

    assert_eq!(descriptor.name(), "full_name");
    assert_eq!(descriptor.property_type(), PropertyType::Hybrid);
}

// Test 2: Descriptor inspection
#[test]
fn test_descriptor_inspection() {
    let class_id = TypeId::of::<User>();
    let descriptor = HybridPropertyDescriptor::new("full_name", class_id)
        .with_expression(true)
        .with_comparator(true);

    assert!(descriptor.has_expression());
    assert!(descriptor.has_comparator());
}

// Test 3: Info inspection from hybrid property
#[test]
fn test_inspection_info_from_hybrid() {
    let class_id = TypeId::of::<User>();
    let mut info = HashMap::new();
    info.insert("description".to_string(), "User's full name".to_string());
    info.insert("searchable".to_string(), "true".to_string());

    let descriptor = HybridPropertyDescriptor::new("full_name", class_id).with_info(info.clone());

    assert_eq!(descriptor.info().len(), 2);
    assert_eq!(
        descriptor.info().get("description"),
        Some(&"User's full name".to_string())
    );
    assert_eq!(
        descriptor.info().get("searchable"),
        Some(&"true".to_string())
    );
}

// Test 4: Property key (different from name)
#[test]
fn test_property_key() {
    let class_id = TypeId::of::<User>();
    let descriptor = HybridPropertyDescriptor::new("full_name", class_id).with_key("fullName");

    assert_eq!(descriptor.name(), "full_name");
    assert_eq!(descriptor.key(), "fullName");
}

// Test 5: Property class inspection
#[test]
fn test_property_class() {
    let class_id = TypeId::of::<User>();
    let descriptor = HybridPropertyDescriptor::new("full_name", class_id);

    assert_eq!(descriptor.class_id(), TypeId::of::<User>());
}

// Test 6: Don't assume attr key is present
#[test]
fn test_inspection_attr_key() {
    let class_id = TypeId::of::<User>();
    let inspector = ModelInspector::new();

    // Property doesn't exist yet
    let property = inspector.get_property("nonexistent");
    assert!(property.is_none());
}

// Test 7: Filter by mismatched column
#[test]
fn test_inspection_mismatched_col() {
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    // Add a hybrid property
    inspector.add_property(Box::new(HybridPropertyDescriptor::new(
        "full_name",
        class_id,
    )));

    // Add a column with different name and key
    inspector.add_property(Box::new(
        ColumnDescriptor::new("first_name", class_id).with_column_name("fname"),
    ));

    // Get hybrid properties only
    let hybrids = inspector.get_hybrid_properties();
    assert_eq!(hybrids.len(), 1);
    assert_eq!(hybrids[0].name(), "full_name");
}

// Test 8: Info not mirrored (column info != hybrid info)
#[test]
fn test_inspection_info_not_mirrored() {
    let class_id = TypeId::of::<User>();

    let mut column_info = HashMap::new();
    column_info.insert("indexed".to_string(), "true".to_string());

    let mut hybrid_info = HashMap::new();
    hybrid_info.insert("computed".to_string(), "true".to_string());

    let column = ColumnDescriptor::new("name", class_id).with_info(column_info);
    let hybrid = HybridPropertyDescriptor::new("name", class_id).with_info(hybrid_info);

    // Info should not be shared
    assert_eq!(column.info().get("indexed"), Some(&"true".to_string()));
    assert!(column.info().get("computed").is_none());

    assert_eq!(hybrid.info().get("computed"), Some(&"true".to_string()));
    assert!(hybrid.info().get("indexed").is_none());
}

// Test 9: Get all properties from inspector
#[test]
fn test_get_all_properties() {
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    inspector.add_property(Box::new(HybridPropertyDescriptor::new(
        "full_name",
        class_id,
    )));
    inspector.add_property(Box::new(ColumnDescriptor::new("id", class_id)));
    inspector.add_property(Box::new(ColumnDescriptor::new("first_name", class_id)));

    let all = inspector.all_properties();
    assert_eq!(all.len(), 3);
}

// Test 10: Get only hybrid properties
#[test]
fn test_get_only_hybrid_properties() {
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    inspector.add_property(Box::new(HybridPropertyDescriptor::new(
        "full_name",
        class_id,
    )));
    inspector.add_property(Box::new(HybridPropertyDescriptor::new(
        "display_name",
        class_id,
    )));
    inspector.add_property(Box::new(ColumnDescriptor::new("id", class_id)));

    let hybrids = inspector.get_hybrid_properties();
    assert_eq!(hybrids.len(), 2);
}

// Test 11: Get only column properties
#[test]
fn test_get_only_column_properties() {
    let class_id = TypeId::of::<User>();
    let mut inspector = ModelInspector::new();

    inspector.add_property(Box::new(ColumnDescriptor::new("id", class_id)));
    inspector.add_property(Box::new(ColumnDescriptor::new("first_name", class_id)));
    inspector.add_property(Box::new(HybridPropertyDescriptor::new(
        "full_name",
        class_id,
    )));

    let columns = inspector.get_column_properties();
    assert_eq!(columns.len(), 2);
}
