//! Integration tests for nested serializers and recursive serialization

use reinhardt_orm::Model;
use reinhardt_serializers::ModelSerializer;
use reinhardt_serializers::nested_config::{NestedFieldConfig, NestedSerializerConfig};
use reinhardt_serializers::recursive::{SerializationContext, circular, depth};
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
	let author = Author {
		id: Some(1),
		name: "John".to_string(),
		email: "john@example.com".to_string(),
	};

	let mut context = SerializationContext::new(5);

	// First visit succeeds
	assert!(context.visit(&author));

	// Second visit fails (circular reference detected)
	assert!(!context.visit(&author));
}

#[test]
fn test_circular_visit_cleanup() {
	let author = Author {
		id: Some(1),
		name: "John".to_string(),
		email: "john@example.com".to_string(),
	};

	let mut context = SerializationContext::new(5);

	context.visit(&author);
	context.leave(&author);

	// After leaving, can visit again
	assert!(context.visit(&author));
}

#[test]
fn test_circular_visit_with_cleanup() {
	let author = Author {
		id: Some(1),
		name: "John".to_string(),
		email: "john@example.com".to_string(),
	};

	let mut context = SerializationContext::new(5);

	let result = circular::visit_with(&mut context, &author, |_ctx| Ok(42));

	assert_eq!(result.unwrap(), 42);
	// Object is automatically unmarked after the function completes
	assert!(context.visit(&author)); // Can visit again
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
	let post = Post {
		id: Some(1),
		title: "Test".to_string(),
		content: "Content".to_string(),
		author_id: 1,
	};

	let mut context = SerializationContext::new(3);

	circular::visit_with(&mut context, &post, |ctx| {
		assert_eq!(ctx.current_depth(), 1);

		let child = ctx.child();
		assert_eq!(child.current_depth(), 2);

		Ok(())
	})
	.unwrap();

	// Object is automatically unmarked after the function completes
	assert!(context.visit(&post)); // Can visit again
}

#[test]
fn test_different_objects_same_data() {
	let author1 = Author {
		id: Some(1),
		name: "John".to_string(),
		email: "john@example.com".to_string(),
	};
	let author2 = Author {
		id: Some(1),
		name: "John".to_string(),
		email: "john@example.com".to_string(),
	};

	let mut context = SerializationContext::new(5);

	// Both authors have same data but different memory addresses
	assert!(context.visit(&author1));
	assert!(context.visit(&author2)); // Should succeed - different objects

	context.leave(&author1);
	context.leave(&author2);
}

#[test]
fn test_same_object_multiple_references() {
	let author = Author {
		id: Some(1),
		name: "John".to_string(),
		email: "john@example.com".to_string(),
	};

	let mut context = SerializationContext::new(5);

	// Visit the same object
	assert!(context.visit(&author));

	// Create another reference to the same object
	let author_ref = &author;

	// Second visit with different reference should fail (same object)
	assert!(!context.visit(author_ref));
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

// Arena Allocation Tests
mod arena_tests {
	use super::*;
	use reinhardt_serializers::arena::{FieldValue, SerializationArena};
	use std::collections::HashMap;

	#[test]
	fn test_arena_basic_serialization() {
		let arena = SerializationArena::new();
		let author = Author {
			id: Some(1),
			name: "Alice".to_string(),
			email: "alice@example.com".to_string(),
		};

		let serialized = arena.serialize_model(&author, 3);
		let json = arena.to_json(serialized);

		// Verify JSON structure with detailed error messages
		assert_eq!(
			json["id"].as_f64().unwrap() as i64,
			1,
			"Expected author id to be 1, but got {:?}",
			json["id"]
		);
		assert_eq!(
			json["name"].as_str().unwrap(),
			"Alice",
			"Expected author name to be 'Alice', but got {:?}",
			json["name"]
		);
		assert_eq!(
			json["email"].as_str().unwrap(),
			"alice@example.com",
			"Expected author email to be 'alice@example.com', but got {:?}",
			json["email"]
		);
	}

	#[test]
	fn test_arena_nested_structure() {
		let arena = SerializationArena::new();

		let mut inner_map = HashMap::new();
		inner_map.insert("city".to_string(), FieldValue::String("Tokyo".to_string()));
		inner_map.insert(
			"country".to_string(),
			FieldValue::String("Japan".to_string()),
		);

		let mut outer_map = HashMap::new();
		outer_map.insert("name".to_string(), FieldValue::String("Alice".to_string()));
		outer_map.insert("address".to_string(), FieldValue::Object(inner_map));

		let serialized = arena.allocate_field(&FieldValue::Object(outer_map));
		let json = arena.to_json(serialized);

		// Verify nested structure with detailed error messages
		assert_eq!(
			json["name"].as_str().unwrap(),
			"Alice",
			"Expected name to be 'Alice', but got {:?}",
			json["name"]
		);
		assert_eq!(
			json["address"]["city"].as_str().unwrap(),
			"Tokyo",
			"Expected city to be 'Tokyo', but got {:?}",
			json["address"]["city"]
		);
		assert_eq!(
			json["address"]["country"].as_str().unwrap(),
			"Japan",
			"Expected country to be 'Japan', but got {:?}",
			json["address"]["country"]
		);
	}

	#[test]
	fn test_arena_deeply_nested_structure() {
		let arena = SerializationArena::new();

		// Create depth-5 nested structure
		let mut level5 = HashMap::new();
		level5.insert("value".to_string(), FieldValue::String("deep".to_string()));

		let mut level4 = HashMap::new();
		level4.insert("level5".to_string(), FieldValue::Object(level5));

		let mut level3 = HashMap::new();
		level3.insert("level4".to_string(), FieldValue::Object(level4));

		let mut level2 = HashMap::new();
		level2.insert("level3".to_string(), FieldValue::Object(level3));

		let mut level1 = HashMap::new();
		level1.insert("level2".to_string(), FieldValue::Object(level2));

		let serialized = arena.allocate_field(&FieldValue::Object(level1));
		let json = arena.to_json(serialized);

		// Verify deeply nested structure with detailed error messages
		assert_eq!(
			json["level2"]["level3"]["level4"]["level5"]["value"]
				.as_str()
				.unwrap(),
			"deep",
			"Expected deeply nested value to be 'deep', but got {:?}",
			json["level2"]["level3"]["level4"]["level5"]["value"]
		);
	}

	#[test]
	fn test_arena_large_array() {
		let arena = SerializationArena::new();

		// Create large array (1000 elements)
		let arr: Vec<FieldValue> = (0..1000).map(|i| FieldValue::Integer(i)).collect();

		let serialized = arena.allocate_field(&FieldValue::Array(arr));
		let json = arena.to_json(serialized);

		assert!(json.is_array(), "Expected JSON to be an array");
		assert_eq!(
			json.as_array().unwrap().len(),
			1000,
			"Expected array to have 1000 elements"
		);
		assert_eq!(
			json[0].as_i64().unwrap(),
			0,
			"Expected first element to be 0, but got {:?}",
			json[0]
		);
		assert_eq!(
			json[999].as_i64().unwrap(),
			999,
			"Expected last element to be 999, but got {:?}",
			json[999]
		);
	}

	#[test]
	fn test_arena_mixed_nested_structure() {
		let arena = SerializationArena::new();

		// Object containing arrays containing objects
		let mut inner_obj = HashMap::new();
		inner_obj.insert("id".to_string(), FieldValue::Integer(1));
		inner_obj.insert(
			"title".to_string(),
			FieldValue::String("Post 1".to_string()),
		);

		let arr = vec![FieldValue::Object(inner_obj)];

		let mut outer_obj = HashMap::new();
		outer_obj.insert("items".to_string(), FieldValue::Array(arr));
		outer_obj.insert("count".to_string(), FieldValue::Integer(1));

		let serialized = arena.allocate_field(&FieldValue::Object(outer_obj));
		let json = arena.to_json(serialized);

		// Verify mixed nested structure with detailed error messages
		assert_eq!(
			json["items"][0]["id"].as_i64().unwrap(),
			1,
			"Expected item id to be 1, but got {:?}",
			json["items"][0]["id"]
		);
		assert_eq!(
			json["items"][0]["title"].as_str().unwrap(),
			"Post 1",
			"Expected item title to be 'Post 1', but got {:?}",
			json["items"][0]["title"]
		);
		assert_eq!(
			json["count"].as_i64().unwrap(),
			1,
			"Expected count to be 1, but got {:?}",
			json["count"]
		);
	}

	#[test]
	fn test_arena_complex_nested_model() {
		let arena = SerializationArena::new();

		// Create a complex nested model structure
		let post1 = Post {
			id: Some(1),
			title: "First Post".to_string(),
			content: "Content 1".to_string(),
			author_id: 1,
		};

		let post2 = Post {
			id: Some(2),
			title: "Second Post".to_string(),
			content: "Content 2".to_string(),
			author_id: 1,
		};

		let serialized1 = arena.serialize_model(&post1, 3);
		let serialized2 = arena.serialize_model(&post2, 3);

		let json1 = arena.to_json(serialized1);
		let json2 = arena.to_json(serialized2);

		// Verify complex nested models with detailed error messages
		assert_eq!(
			json1["id"].as_f64().unwrap() as i64,
			1,
			"Expected post1 id to be 1, but got {:?}",
			json1["id"]
		);
		assert_eq!(
			json1["title"].as_str().unwrap(),
			"First Post",
			"Expected post1 title to be 'First Post', but got {:?}",
			json1["title"]
		);
		assert_eq!(
			json2["id"].as_f64().unwrap() as i64,
			2,
			"Expected post2 id to be 2, but got {:?}",
			json2["id"]
		);
		assert_eq!(
			json2["title"].as_str().unwrap(),
			"Second Post",
			"Expected post2 title to be 'Second Post', but got {:?}",
			json2["title"]
		);
	}

	#[test]
	fn test_arena_with_depth_10() {
		let arena = SerializationArena::new();

		// Create depth-10 nested structure
		let mut current = HashMap::new();
		current.insert("value".to_string(), FieldValue::Integer(10));

		for i in (1..=10).rev() {
			let mut next = HashMap::new();
			next.insert(format!("level{}", i), FieldValue::Object(current.clone()));
			current = next;
		}

		let serialized = arena.allocate_field(&FieldValue::Object(current));
		let json = arena.to_json(serialized);

		// Verify the deep nesting with detailed error messages
		let mut current_json = &json;
		for i in 1..=10 {
			current_json = &current_json[format!("level{}", i)];
		}
		assert_eq!(
			current_json["value"].as_i64().unwrap(),
			10,
			"Expected deeply nested value to be 10, but got {:?}",
			current_json["value"]
		);
	}

	#[test]
	fn test_arena_multiple_allocations_in_same_arena() {
		let arena = SerializationArena::new();

		// Allocate multiple independent structures in the same arena
		let field1 = FieldValue::String("value1".to_string());
		let field2 = FieldValue::Integer(42);
		let field3 = FieldValue::Boolean(true);

		let serialized1 = arena.allocate_field(&field1);
		let serialized2 = arena.allocate_field(&field2);
		let serialized3 = arena.allocate_field(&field3);

		let json1 = arena.to_json(serialized1);
		let json2 = arena.to_json(serialized2);
		let json3 = arena.to_json(serialized3);

		// Verify multiple allocations with detailed error messages
		assert_eq!(
			json1.as_str().unwrap(),
			"value1",
			"Expected first allocation to be 'value1', but got {:?}",
			json1
		);
		assert_eq!(
			json2.as_i64().unwrap(),
			42,
			"Expected second allocation to be 42, but got {:?}",
			json2
		);
		assert_eq!(
			json3.as_bool().unwrap(),
			true,
			"Expected third allocation to be true, but got {:?}",
			json3
		);
	}

	#[test]
	fn test_arena_many_nodes_structure() {
		let arena = SerializationArena::new();

		// Create a structure with many nodes (100 objects)
		let mut objects = vec![];
		for i in 0..100 {
			let mut obj = HashMap::new();
			obj.insert("id".to_string(), FieldValue::Integer(i));
			obj.insert(
				"name".to_string(),
				FieldValue::String(format!("Item {}", i)),
			);
			objects.push(FieldValue::Object(obj));
		}

		let serialized = arena.allocate_field(&FieldValue::Array(objects));
		let json = arena.to_json(serialized);

		assert!(json.is_array(), "Expected JSON to be an array");
		assert_eq!(
			json.as_array().unwrap().len(),
			100,
			"Expected array to have 100 elements"
		);
		assert_eq!(
			json[0]["id"].as_i64().unwrap(),
			0,
			"Expected first item id to be 0, but got {:?}",
			json[0]["id"]
		);
		assert_eq!(
			json[0]["name"].as_str().unwrap(),
			"Item 0",
			"Expected first item name to be 'Item 0', but got {:?}",
			json[0]["name"]
		);
		assert_eq!(
			json[99]["id"].as_i64().unwrap(),
			99,
			"Expected last item id to be 99, but got {:?}",
			json[99]["id"]
		);
		assert_eq!(
			json[99]["name"].as_str().unwrap(),
			"Item 99",
			"Expected last item name to be 'Item 99', but got {:?}",
			json[99]["name"]
		);
	}
}
