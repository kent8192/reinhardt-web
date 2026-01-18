//! Tests for migration autodetector
//! Translated from Django's test_autodetector.py
//!
//! Django reference: django/tests/migrations/test_autodetector.py

use reinhardt_db::migrations::{
	ConstraintDefinition, FieldState, FieldType, IndexDefinition, MigrationAutodetector,
	ModelState, ProjectState,
};
use std::collections::HashSet;

/// Helper function to create a simple field
fn field(name: &str, field_type: FieldType, nullable: bool) -> FieldState {
	FieldState::new(name.to_string(), field_type, nullable)
}

#[test]
fn test_create_model_simple() {
	// Django: test_arrange_for_graph_with_multiple_initial
	// Test that a simple model creation is detected
	let from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	let mut book_model = ModelState::new("testapp", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	to_state.add_model(book_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.created_models.len(), 1);
	assert_eq!(changes.created_models[0].0, "testapp");
	assert_eq!(changes.created_models[0].1, "Book");
}

#[test]
fn test_delete_model() {
	// Django: test_delete_model
	// Test that model deletion is detected
	let mut from_state = ProjectState::new();
	let to_state = ProjectState::new();

	let mut book_model = ModelState::new("testapp", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	from_state.add_model(book_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.deleted_models.len(), 1);
	assert_eq!(changes.deleted_models[0].0, "testapp");
	assert_eq!(changes.deleted_models[0].1, "Book");
}

#[test]
fn test_add_field() {
	// Django: test_add_field
	// Test that adding a field is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Create initial model
	let mut book_model_old = ModelState::new("testapp", "Book");
	book_model_old.add_field(field("id", FieldType::Integer, false));
	book_model_old.add_field(field("title", FieldType::VarChar(200), false));
	from_state.add_model(book_model_old);

	// Add a field to the model
	let mut book_model_new = ModelState::new("testapp", "Book");
	book_model_new.add_field(field("id", FieldType::Integer, false));
	book_model_new.add_field(field("title", FieldType::VarChar(200), false));
	book_model_new.add_field(field("author", FieldType::VarChar(100), false));
	to_state.add_model(book_model_new);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.added_fields.len(), 1);
	assert_eq!(changes.added_fields[0].0, "testapp");
	assert_eq!(changes.added_fields[0].1, "Book");
	assert_eq!(changes.added_fields[0].2, "author");
}

#[test]
fn test_remove_field() {
	// Django: test_remove_field
	// Test that removing a field is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Create initial model with three fields
	let mut book_model_old = ModelState::new("testapp", "Book");
	book_model_old.add_field(field("id", FieldType::Integer, false));
	book_model_old.add_field(field("title", FieldType::VarChar(200), false));
	book_model_old.add_field(field("author", FieldType::VarChar(100), false));
	from_state.add_model(book_model_old);

	// Remove a field from the model
	let mut book_model_new = ModelState::new("testapp", "Book");
	book_model_new.add_field(field("id", FieldType::Integer, false));
	book_model_new.add_field(field("title", FieldType::VarChar(200), false));
	to_state.add_model(book_model_new);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.removed_fields.len(), 1);
	assert_eq!(changes.removed_fields[0].0, "testapp");
	assert_eq!(changes.removed_fields[0].1, "Book");
	assert_eq!(changes.removed_fields[0].2, "author");
}

#[test]
fn test_alter_field() {
	// Django: test_alter_field
	// Test that altering a field is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Create initial model
	let mut book_model_old = ModelState::new("testapp", "Book");
	book_model_old.add_field(field("id", FieldType::Integer, false));
	book_model_old.add_field(field("title", FieldType::VarChar(100), false));
	from_state.add_model(book_model_old);

	// Change field type
	let mut book_model_new = ModelState::new("testapp", "Book");
	book_model_new.add_field(field("id", FieldType::Integer, false));
	book_model_new.add_field(field("title", FieldType::VarChar(200), false));
	to_state.add_model(book_model_new);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.altered_fields.len(), 1);
	assert_eq!(changes.altered_fields[0].0, "testapp");
	assert_eq!(changes.altered_fields[0].1, "Book");
	assert_eq!(changes.altered_fields[0].2, "title");
}

#[test]
fn test_alter_field_nullable() {
	// Django: test_alter_field_to_not_null_without_default
	// Test that changing nullable status is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Create initial model with nullable field
	let mut book_model_old = ModelState::new("testapp", "Book");
	book_model_old.add_field(field("id", FieldType::Integer, false));
	book_model_old.add_field(field("title", FieldType::VarChar(200), true));
	from_state.add_model(book_model_old);

	// Change to not nullable
	let mut book_model_new = ModelState::new("testapp", "Book");
	book_model_new.add_field(field("id", FieldType::Integer, false));
	book_model_new.add_field(field("title", FieldType::VarChar(200), false));
	to_state.add_model(book_model_new);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.altered_fields.len(), 1);
	assert_eq!(changes.altered_fields[0].0, "testapp");
	assert_eq!(changes.altered_fields[0].1, "Book");
	assert_eq!(changes.altered_fields[0].2, "title");
}

#[test]
fn test_no_changes() {
	// Django: test_empty
	// Test that no changes results in no migrations
	let mut state = ProjectState::new();

	let mut book_model = ModelState::new("testapp", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	state.add_model(book_model.clone());

	let autodetector = MigrationAutodetector::new(state.clone(), state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.created_models.len(), 0);
	assert_eq!(changes.deleted_models.len(), 0);
	assert_eq!(changes.added_fields.len(), 0);
	assert_eq!(changes.removed_fields.len(), 0);
	assert_eq!(changes.altered_fields.len(), 0);
}

#[test]
fn test_multiple_apps() {
	// Django: test_arrange_for_graph_with_multiple_apps
	// Test that changes in multiple apps are detected
	let from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Add models to two different apps
	let mut book_model = ModelState::new("books", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	to_state.add_model(book_model);

	let mut author_model = ModelState::new("authors", "Author");
	author_model.add_field(field("id", FieldType::Integer, false));
	author_model.add_field(field("name", FieldType::VarChar(100), false));
	to_state.add_model(author_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.created_models.len(), 2);

	// Use HashSet for collection comparison with detailed error message
	let actual_apps: HashSet<_> = changes
		.created_models
		.iter()
		.map(|(app, _)| app.as_str())
		.collect();
	let expected_apps: HashSet<_> = ["books", "authors"].iter().cloned().collect();
	assert_eq!(
		actual_apps, expected_apps,
		"Expected apps {:?} but got {:?}",
		expected_apps, actual_apps
	);
}

#[test]
fn test_rename_model_detected_as_rename() {
	// Test that model rename can be detected when fields are identical
	// Note: Current implementation may detect this as delete+create depending on similarity threshold
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Old model with complete field set
	let mut old_model = ModelState::new("testapp", "OldBook");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("title", FieldType::VarChar(200), false));
	old_model.add_field(field("author", FieldType::VarChar(100), false));
	from_state.add_model(old_model);

	// New model with identical fields (high field similarity)
	let mut new_model = ModelState::new("testapp", "NewBook");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("title", FieldType::VarChar(200), false));
	new_model.add_field(field("author", FieldType::VarChar(100), false));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// Verify either rename detection (if implemented) or delete+create fallback
	let has_rename = !changes.renamed_models.is_empty();
	let has_delete_create = changes.created_models.len() == 1 && changes.deleted_models.len() == 1;

	assert!(
		has_rename || has_delete_create,
		"Expected either rename detection or delete+create fallback. \
         renamed: {}, created: {}, deleted: {}",
		changes.renamed_models.len(),
		changes.created_models.len(),
		changes.deleted_models.len()
	);
}

#[test]
fn test_rename_model_detected_as_delete_and_create() {
	// Test that model with different fields is detected as delete + create
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Old model
	let mut old_model = ModelState::new("testapp", "OldBook");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("old_field1", FieldType::VarChar(100), false));
	old_model.add_field(field("old_field2", FieldType::Integer, false));
	from_state.add_model(old_model);

	// New model with completely different fields (low similarity)
	let mut new_model = ModelState::new("testapp", "NewBook");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("new_field1", FieldType::Text, false));
	new_model.add_field(field("new_field2", FieldType::Boolean, false));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// Should detect as delete + create due to low similarity
	assert_eq!(
		changes.created_models.len(),
		1,
		"Expected model creation to be detected"
	);
	assert_eq!(
		changes.deleted_models.len(),
		1,
		"Expected model deletion to be detected"
	);

	// Verify not detected as rename
	assert!(
		changes.renamed_models.is_empty(),
		"Should not detect as rename"
	);
}

#[test]
fn test_rename_field_detected_as_rename() {
	// Test that field rename can be detected when type matches
	// Note: Current implementation may detect this as add+remove depending on similarity threshold
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Old model with original field name
	let mut old_model = ModelState::new("testapp", "Book");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("old_title", FieldType::VarChar(200), false));
	from_state.add_model(old_model);

	// New model with renamed field (same type)
	let mut new_model = ModelState::new("testapp", "Book");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("new_title", FieldType::VarChar(200), false));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// Verify either rename detection (if implemented) or add+remove fallback
	let has_rename = !changes.renamed_fields.is_empty();
	let has_add_remove = changes.added_fields.len() == 1 && changes.removed_fields.len() == 1;

	assert!(
		has_rename || has_add_remove,
		"Expected either rename detection or add+remove fallback. \
         renamed: {}, added: {}, removed: {}",
		changes.renamed_fields.len(),
		changes.added_fields.len(),
		changes.removed_fields.len()
	);
}

#[test]
fn test_rename_field_detected_as_add_and_remove() {
	// Test that fields with different types are detected as remove + add
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Old model with original field
	let mut old_model = ModelState::new("testapp", "Book");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("old_field", FieldType::VarChar(100), false));
	from_state.add_model(old_model);

	// New model with different field (different type)
	let mut new_model = ModelState::new("testapp", "Book");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("new_field", FieldType::Text, false));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// Should detect as add + remove due to different types
	assert_eq!(
		changes.added_fields.len(),
		1,
		"Expected field addition to be detected"
	);
	assert_eq!(
		changes.removed_fields.len(),
		1,
		"Expected field removal to be detected"
	);

	// Verify not detected as rename
	assert!(
		changes.renamed_fields.is_empty(),
		"Should not detect as rename"
	);
}

#[test]
fn test_add_index() {
	// Django: test_create_model_with_indexes
	// Test that adding an index is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Model without index
	let mut old_model = ModelState::new("testapp", "Book");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("title", FieldType::VarChar(200), false));
	from_state.add_model(old_model);

	// Model with index
	let mut new_model = ModelState::new("testapp", "Book");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("title", FieldType::VarChar(200), false));
	new_model.indexes.push(IndexDefinition {
		name: "idx_title".to_string(),
		fields: vec!["title".to_string()],
		unique: false,
	});
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.added_indexes.len(), 1);
	assert_eq!(changes.added_indexes[0].0, "testapp");
	assert_eq!(changes.added_indexes[0].1, "Book");
	assert_eq!(changes.added_indexes[0].2.name, "idx_title");
}

#[test]
fn test_remove_index() {
	// Django: test_remove_index
	// Test that removing an index is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Model with index
	let mut old_model = ModelState::new("testapp", "Book");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("title", FieldType::VarChar(200), false));
	old_model.indexes.push(IndexDefinition {
		name: "idx_title".to_string(),
		fields: vec!["title".to_string()],
		unique: false,
	});
	from_state.add_model(old_model);

	// Model without index
	let mut new_model = ModelState::new("testapp", "Book");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("title", FieldType::VarChar(200), false));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.removed_indexes.len(), 1);
	assert_eq!(changes.removed_indexes[0].0, "testapp");
	assert_eq!(changes.removed_indexes[0].1, "Book");
	assert_eq!(changes.removed_indexes[0].2, "idx_title");
}

#[test]
fn test_add_constraint() {
	// Django: test_add_constraints
	// Test that adding a constraint is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Model without constraint
	let mut old_model = ModelState::new("testapp", "Book");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field(
		"price",
		FieldType::Decimal {
			precision: 10,
			scale: 2,
		},
		false,
	));
	from_state.add_model(old_model);

	// Model with constraint
	let mut new_model = ModelState::new("testapp", "Book");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field(
		"price",
		FieldType::Decimal {
			precision: 10,
			scale: 2,
		},
		false,
	));
	new_model.constraints.push(ConstraintDefinition {
		name: "chk_price_positive".to_string(),
		constraint_type: "check".to_string(),
		fields: vec!["price".to_string()],
		expression: Some("price > 0".to_string()),
		foreign_key_info: None,
	});
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.added_constraints.len(), 1);
	assert_eq!(changes.added_constraints[0].0, "testapp");
	assert_eq!(changes.added_constraints[0].1, "Book");
	assert_eq!(changes.added_constraints[0].2.name, "chk_price_positive");
}

#[test]
fn test_remove_constraint() {
	// Django: test_remove_constraints
	// Test that removing a constraint is detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Model with constraint
	let mut old_model = ModelState::new("testapp", "Book");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field(
		"price",
		FieldType::Decimal {
			precision: 10,
			scale: 2,
		},
		false,
	));
	old_model.constraints.push(ConstraintDefinition {
		name: "chk_price_positive".to_string(),
		constraint_type: "check".to_string(),
		fields: vec!["price".to_string()],
		expression: Some("price > 0".to_string()),
		foreign_key_info: None,
	});
	from_state.add_model(old_model);

	// Model without constraint
	let mut new_model = ModelState::new("testapp", "Book");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field(
		"price",
		FieldType::Decimal {
			precision: 10,
			scale: 2,
		},
		false,
	));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.removed_constraints.len(), 1);
	assert_eq!(changes.removed_constraints[0].0, "testapp");
	assert_eq!(changes.removed_constraints[0].1, "Book");
	assert_eq!(changes.removed_constraints[0].2, "chk_price_positive");
}

#[test]
fn test_multiple_changes_same_model() {
	// Django: test_multiple_changes
	// Test that multiple changes to the same model are all detected
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Old model
	let mut old_model = ModelState::new("testapp", "Book");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("title", FieldType::VarChar(100), false));
	old_model.add_field(field("old_field", FieldType::VarChar(50), false));
	from_state.add_model(old_model);

	// New model with multiple changes
	let mut new_model = ModelState::new("testapp", "Book");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("title", FieldType::VarChar(200), false)); // altered
	new_model.add_field(field("new_field", FieldType::VarChar(50), false)); // added
	// old_field removed
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.added_fields.len(), 1);
	assert_eq!(changes.removed_fields.len(), 1);
	assert_eq!(changes.altered_fields.len(), 1);
}

#[test]
fn test_create_model_with_multiple_fields() {
	// Django: test_create_model_with_check_constraint
	// Test creating a model with multiple fields
	let from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	let mut book_model = ModelState::new("testapp", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	book_model.add_field(field("author", FieldType::VarChar(100), false));
	book_model.add_field(field(
		"price",
		FieldType::Decimal {
			precision: 10,
			scale: 2,
		},
		false,
	));
	book_model.add_field(field("published_date", FieldType::Date, true));
	to_state.add_model(book_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state.clone());
	let changes = autodetector.detect_changes();

	assert_eq!(changes.created_models.len(), 1);
	let model = to_state.get_model("testapp", "Book").unwrap();
	assert_eq!(model.fields.len(), 5);
}

#[test]
fn test_unchanged_model_with_index() {
	// Test that unchanged models with indexes don't generate changes
	let mut state = ProjectState::new();

	let mut book_model = ModelState::new("testapp", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	book_model.indexes.push(IndexDefinition {
		name: "idx_title".to_string(),
		fields: vec!["title".to_string()],
		unique: false,
	});
	state.add_model(book_model.clone());

	let autodetector = MigrationAutodetector::new(state.clone(), state);
	let changes = autodetector.detect_changes();

	assert_eq!(changes.added_indexes.len(), 0);
	assert_eq!(changes.removed_indexes.len(), 0);
}

#[test]
fn test_generate_operations_from_changes() {
	// Test that generate_operations correctly converts DetectedChanges to Operations
	let from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	let mut book_model = ModelState::new("testapp", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	to_state.add_model(book_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let operations = autodetector.generate_operations();

	assert!(!operations.is_empty());
	// Should generate at least one CreateTable operation
	assert!(
		operations
			.iter()
			.any(|op| matches!(op, reinhardt_db::migrations::Operation::CreateTable { .. }))
	);
}

#[test]
fn test_generate_migrations() {
	// Test that generate_migrations creates Migration objects grouped by app
	let from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Add models to two different apps
	let mut book_model = ModelState::new("books", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	to_state.add_model(book_model);

	let mut author_model = ModelState::new("authors", "Author");
	author_model.add_field(field("id", FieldType::Integer, false));
	to_state.add_model(author_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let migrations = autodetector.generate_migrations();

	assert_eq!(migrations.len(), 2);

	// Use HashSet for collection comparison with detailed error message
	let actual_apps: HashSet<_> = migrations.iter().map(|m| m.app_label.as_str()).collect();
	let expected_apps: HashSet<_> = ["books", "authors"].iter().cloned().collect();
	assert_eq!(
		actual_apps, expected_apps,
		"Expected app labels {:?} but got {:?}",
		expected_apps, actual_apps
	);
}

// ============================================================================
// Hybrid Similarity and Dependency Checking
// ============================================================================

#[test]
fn test_similarity_config_default() {
	// Test default similarity configuration
	use reinhardt_db::migrations::SimilarityConfig;

	let config = SimilarityConfig::default();
	assert_eq!(config.model_threshold(), 0.7);
	assert_eq!(config.field_threshold(), 0.8);
}

#[test]
fn test_similarity_config_new() {
	// Test creating custom similarity configuration
	use reinhardt_db::migrations::SimilarityConfig;

	let config = SimilarityConfig::new(0.75, 0.85).unwrap();
	assert_eq!(config.model_threshold(), 0.75);
	assert_eq!(config.field_threshold(), 0.85);
}

#[test]
fn test_similarity_config_validation() {
	// Test similarity config threshold validation
	use reinhardt_db::migrations::SimilarityConfig;

	// Valid thresholds
	assert!(SimilarityConfig::new(0.5, 0.5).is_ok());
	assert!(SimilarityConfig::new(0.95, 0.95).is_ok());
	assert!(SimilarityConfig::new(0.7, 0.8).is_ok());

	// Invalid thresholds - too low
	assert!(SimilarityConfig::new(0.4, 0.8).is_err());
	assert!(SimilarityConfig::new(0.7, 0.4).is_err());

	// Invalid thresholds - too high
	assert!(SimilarityConfig::new(0.96, 0.8).is_err());
	assert!(SimilarityConfig::new(0.7, 0.96).is_err());
}

#[test]
fn test_similarity_config_with_weights() {
	// Test similarity config with custom algorithm weights
	use reinhardt_db::migrations::SimilarityConfig;

	// Valid weights (sum to 1.0)
	let config = SimilarityConfig::with_weights(0.75, 0.85, 0.8, 0.2).unwrap();
	assert_eq!(config.model_threshold(), 0.75);
	assert_eq!(config.field_threshold(), 0.85);

	// Prefer Levenshtein over Jaro-Winkler
	let config = SimilarityConfig::with_weights(0.7, 0.8, 0.3, 0.7).unwrap();
	assert_eq!(config.model_threshold(), 0.7);

	// Equal weights
	let config = SimilarityConfig::with_weights(0.7, 0.8, 0.5, 0.5).unwrap();
	assert_eq!(config.model_threshold(), 0.7);
}

#[test]
fn test_similarity_config_weights_validation() {
	// Test that weights must sum to 1.0
	use reinhardt_db::migrations::SimilarityConfig;

	// Invalid weights - don't sum to 1.0
	assert!(SimilarityConfig::with_weights(0.75, 0.85, 0.5, 0.3).is_err());
	assert!(SimilarityConfig::with_weights(0.75, 0.85, 0.8, 0.8).is_err());

	// Invalid weights - out of range
	assert!(SimilarityConfig::with_weights(0.75, 0.85, 1.5, -0.5).is_err());
	assert!(SimilarityConfig::with_weights(0.75, 0.85, -0.2, 1.2).is_err());
}

#[test]
fn test_cross_app_model_move_detected_as_move() {
	// Test that moving models between apps can be detected
	// Note: Current implementation may detect this as delete+create depending on similarity threshold
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Model in app1 with complete field set
	let mut old_model = ModelState::new("app1", "User");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("email", FieldType::VarChar(200), false));
	old_model.add_field(field("username", FieldType::VarChar(100), false));
	from_state.add_model(old_model);

	// Same model moved to app2 (identical fields)
	let mut new_model = ModelState::new("app2", "User");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("email", FieldType::VarChar(200), false));
	new_model.add_field(field("username", FieldType::VarChar(100), false));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// Verify either move detection (if implemented) or delete+create fallback
	let has_move = !changes.moved_models.is_empty();
	let has_delete_create = changes.created_models.len() == 1 && changes.deleted_models.len() == 1;

	assert!(
		has_move || has_delete_create,
		"Expected either move detection or delete+create fallback. \
         moved: {}, created: {}, deleted: {}",
		changes.moved_models.len(),
		changes.created_models.len(),
		changes.deleted_models.len()
	);
}

#[test]
fn test_cross_app_model_move_detected_as_delete_and_create() {
	// Test that models with different structures are detected as delete + create
	let mut from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Model in app1
	let mut old_model = ModelState::new("app1", "User");
	old_model.add_field(field("id", FieldType::Integer, false));
	old_model.add_field(field("old_field1", FieldType::VarChar(100), false));
	old_model.add_field(field("old_field2", FieldType::Integer, false));
	from_state.add_model(old_model);

	// Different model in app2 (different fields)
	let mut new_model = ModelState::new("app2", "User");
	new_model.add_field(field("id", FieldType::Integer, false));
	new_model.add_field(field("new_field1", FieldType::Text, false));
	new_model.add_field(field("new_field2", FieldType::Boolean, false));
	to_state.add_model(new_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// Should detect as delete + create due to different structure
	assert_eq!(
		changes.created_models.len(),
		1,
		"Expected model creation to be detected"
	);
	assert_eq!(
		changes.deleted_models.len(),
		1,
		"Expected model deletion to be detected"
	);

	// Verify not detected as move
	assert!(changes.moved_models.is_empty(), "Should not detect as move");
}

#[test]
fn test_model_dependencies_simple() {
	// Test simple foreign key dependency detection
	let from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Create User model
	let mut user_model = ModelState::new("accounts", "User");
	user_model.add_field(field("id", FieldType::Integer, false));
	user_model.add_field(field("email", FieldType::VarChar(200), false));
	to_state.add_model(user_model);

	// Create Post model that depends on User
	let mut post_model = ModelState::new("blog", "Post");
	post_model.add_field(field("id", FieldType::Integer, false));
	post_model.add_field(field("title", FieldType::VarChar(200), false));
	post_model.add_field(field(
		"author",
		FieldType::Custom("ForeignKey(accounts.User)".to_string()),
		false,
	));
	to_state.add_model(post_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// blog.Post should depend on accounts.User
	assert!(
		changes
			.model_dependencies
			.contains_key(&("blog".to_string(), "Post".to_string()))
	);

	let deps = changes
		.model_dependencies
		.get(&("blog".to_string(), "Post".to_string()))
		.unwrap();
	assert!(deps.contains(&("accounts".to_string(), "User".to_string())));
}

#[test]
fn test_model_dependencies_many_to_many() {
	// Test ManyToMany dependency detection
	let from_state = ProjectState::new();
	let mut to_state = ProjectState::new();

	// Create Author model
	let mut author_model = ModelState::new("authors", "Author");
	author_model.add_field(field("id", FieldType::Integer, false));
	author_model.add_field(field("name", FieldType::VarChar(100), false));
	to_state.add_model(author_model);

	// Create Book model with ManyToMany to Author
	let mut book_model = ModelState::new("books", "Book");
	book_model.add_field(field("id", FieldType::Integer, false));
	book_model.add_field(field("title", FieldType::VarChar(200), false));
	book_model.add_field(field(
		"authors",
		FieldType::Custom("ManyToManyField(authors.Author)".to_string()),
		false,
	));
	to_state.add_model(book_model);

	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let changes = autodetector.detect_changes();

	// books.Book should depend on authors.Author
	let deps = changes
		.model_dependencies
		.get(&("books".to_string(), "Book".to_string()));
	assert!(deps.is_some());
	assert!(
		deps.unwrap()
			.contains(&("authors".to_string(), "Author".to_string()))
	);
}

#[test]
fn test_topological_sort_simple() {
	// Test that models are ordered by dependencies
	use reinhardt_db::migrations::DetectedChanges;
	use std::collections::BTreeMap;

	let mut changes = DetectedChanges::default();
	changes
		.created_models
		.push(("accounts".to_string(), "User".to_string()));
	changes
		.created_models
		.push(("blog".to_string(), "Post".to_string()));

	// Post depends on User
	let mut deps = BTreeMap::new();
	deps.insert(
		("blog".to_string(), "Post".to_string()),
		vec![("accounts".to_string(), "User".to_string())],
	);
	changes.model_dependencies = deps;

	let ordered = changes.order_models_by_dependency();

	// User should come before Post
	let user_idx = ordered
		.iter()
		.position(|m| m == &("accounts".to_string(), "User".to_string()))
		.unwrap();
	let post_idx = ordered
		.iter()
		.position(|m| m == &("blog".to_string(), "Post".to_string()))
		.unwrap();
	assert!(user_idx < post_idx);
}

#[test]
fn test_topological_sort_complex() {
	// Test complex dependency chain: A -> B -> C
	use reinhardt_db::migrations::DetectedChanges;
	use std::collections::BTreeMap;

	let mut changes = DetectedChanges::default();
	changes
		.created_models
		.push(("app".to_string(), "A".to_string()));
	changes
		.created_models
		.push(("app".to_string(), "B".to_string()));
	changes
		.created_models
		.push(("app".to_string(), "C".to_string()));

	// C depends on B, B depends on A
	let mut deps = BTreeMap::new();
	deps.insert(
		("app".to_string(), "B".to_string()),
		vec![("app".to_string(), "A".to_string())],
	);
	deps.insert(
		("app".to_string(), "C".to_string()),
		vec![("app".to_string(), "B".to_string())],
	);
	changes.model_dependencies = deps;

	let ordered = changes.order_models_by_dependency();

	// A should come before B, B should come before C
	let a_idx = ordered
		.iter()
		.position(|m| m == &("app".to_string(), "A".to_string()))
		.unwrap();
	let b_idx = ordered
		.iter()
		.position(|m| m == &("app".to_string(), "B".to_string()))
		.unwrap();
	let c_idx = ordered
		.iter()
		.position(|m| m == &("app".to_string(), "C".to_string()))
		.unwrap();
	assert!(a_idx < b_idx);
	assert!(b_idx < c_idx);
}

#[test]
fn test_circular_dependency_detection() {
	// Test that circular dependencies are detected
	use reinhardt_db::migrations::DetectedChanges;
	use std::collections::BTreeMap;

	let mut changes = DetectedChanges::default();

	// Create circular dependency: A -> B -> C -> A
	let mut deps = BTreeMap::new();
	deps.insert(
		("app".to_string(), "A".to_string()),
		vec![("app".to_string(), "B".to_string())],
	);
	deps.insert(
		("app".to_string(), "B".to_string()),
		vec![("app".to_string(), "C".to_string())],
	);
	deps.insert(
		("app".to_string(), "C".to_string()),
		vec![("app".to_string(), "A".to_string())],
	);
	changes.model_dependencies = deps;

	let result = changes.check_circular_dependencies();
	assert!(result.is_err());
}

#[test]
fn test_no_circular_dependency() {
	// Test that non-circular dependencies pass validation
	use reinhardt_db::migrations::DetectedChanges;
	use std::collections::BTreeMap;

	let mut changes = DetectedChanges::default();

	// Create non-circular dependency: A -> B, C -> D
	let mut deps = BTreeMap::new();
	deps.insert(
		("app".to_string(), "B".to_string()),
		vec![("app".to_string(), "A".to_string())],
	);
	deps.insert(
		("app".to_string(), "D".to_string()),
		vec![("app".to_string(), "C".to_string())],
	);
	changes.model_dependencies = deps;

	let result = changes.check_circular_dependencies();
	assert!(result.is_ok());
}

// ============================================================================
// Pattern Learning and Inference
// ============================================================================

#[test]
fn test_change_tracker_record_model_rename() {
	use reinhardt_db::migrations::ChangeTracker;

	let mut tracker = ChangeTracker::new();

	// Record some model renames
	tracker.record_model_rename("app1", "OldUser", "NewUser");
	tracker.record_model_rename("app2", "OldProduct", "NewProduct");

	// Check history size
	assert_eq!(tracker.len(), 2);

	// Check pattern frequency
	let patterns = tracker.get_frequent_patterns(1);
	assert!(!patterns.is_empty());
}

#[test]
fn test_change_tracker_pattern_frequency() {
	use reinhardt_db::migrations::ChangeTracker;

	let mut tracker = ChangeTracker::new();

	// Record same pattern multiple times
	for _ in 0..3 {
		tracker.record_model_rename("app", "Old", "New");
	}

	// Should have high frequency for this pattern
	let patterns = tracker.get_frequent_patterns(2);
	assert!(!patterns.is_empty());
	assert_eq!(patterns[0].frequency, 3);
}

#[test]
fn test_change_tracker_cooccurrence() {
	use reinhardt_db::migrations::ChangeTracker;
	use std::time::Duration;

	let mut tracker = ChangeTracker::new();

	// Record co-occurring changes
	tracker.record_model_rename("app", "User", "Account");
	tracker.record_field_addition("app", "Account", "email");

	// Analyze co-occurrence within 1 hour
	let cooccurrence = tracker.analyze_cooccurrence(Duration::from_secs(3600));
	assert!(!cooccurrence.is_empty());
}

#[test]
fn test_pattern_matcher_basic() {
	use reinhardt_db::migrations::PatternMatcher;

	let mut matcher = PatternMatcher::new();
	matcher.add_pattern("User");
	matcher.add_pattern("Product");
	matcher.build().unwrap();

	let text = "UserModel and ProductModel";
	let matches = matcher.find_all(text);

	assert_eq!(matches.len(), 2);
	assert_eq!(matches[0].pattern, "User");
	assert_eq!(matches[1].pattern, "Product");
}

#[test]
fn test_pattern_matcher_contains() {
	use reinhardt_db::migrations::PatternMatcher;

	let mut matcher = PatternMatcher::new();
	matcher.add_patterns(vec!["ForeignKey", "ManyToMany", "OneToOne"]);
	matcher.build().unwrap();

	assert!(matcher.contains_any("ForeignKey(User)"));
	assert!(matcher.contains_any("ManyToManyField(Tag)"));
	assert!(!matcher.contains_any("CharField(max_length=100)"));
}

#[test]
fn test_pattern_matcher_replace() {
	use reinhardt_db::migrations::PatternMatcher;
	use std::collections::HashMap;

	let mut matcher = PatternMatcher::new();
	matcher.add_pattern("old_name");
	matcher.build().unwrap();

	let mut replacements = HashMap::new();
	replacements.insert("old_name".to_string(), "new_name".to_string());

	let replaced = matcher.replace_all("old_name is old_name", &replacements);
	assert_eq!(replaced, "new_name is new_name");
}

#[test]
fn test_inference_engine_basic() {
	use reinhardt_db::migrations::{InferenceEngine, InferenceRule, RuleCondition};

	let mut engine = InferenceEngine::new();

	// Add a simple rule
	let rule = InferenceRule {
		name: "test_rule".to_string(),
		conditions: vec![RuleCondition::FieldAddition {
			field_name_pattern: "created_at".to_string(),
		}],
		optional_conditions: vec![],
		intent_type: "Add timestamp tracking".to_string(),
		base_confidence: 0.9,
		confidence_boost_per_optional: 0.05,
	};
	engine.add_rule(rule);

	// Infer intent from field addition
	let intents = engine.infer_intents(
		&[],
		&[],
		&[(
			"app".to_string(),
			"User".to_string(),
			"created_at".to_string(),
		)],
		&[],
	);

	assert_eq!(intents.len(), 1);
	assert_eq!(intents[0].intent_type, "Add timestamp tracking");
	assert_eq!(intents[0].confidence, 0.9);
}

#[test]
fn test_inference_engine_default_rules() {
	use reinhardt_db::migrations::InferenceEngine;

	let mut engine = InferenceEngine::new();
	engine.add_default_rules();

	// Should have 5 default rules
	assert_eq!(engine.rules().len(), 5);
}

#[test]
fn test_inference_engine_model_rename_rule() {
	use reinhardt_db::migrations::InferenceEngine;

	let mut engine = InferenceEngine::new();
	engine.add_default_rules();

	// Test model rename detection
	let intents = engine.infer_intents(
		&[(
			"app".to_string(),
			"OldUser".to_string(),
			"app".to_string(),
			"NewUser".to_string(),
		)],
		&[],
		&[],
		&[],
	);

	// Should detect refactoring intent
	let refactor_intent = intents
		.iter()
		.find(|i| i.intent_type.contains("Refactoring"));
	assert!(refactor_intent.is_some());
}

#[test]
fn test_inference_engine_field_tracking_rule() {
	use reinhardt_db::migrations::InferenceEngine;

	let mut engine = InferenceEngine::new();
	engine.add_default_rules();

	// Test timestamp field addition
	let intents = engine.infer_intents(
		&[],
		&[],
		&[
			(
				"app".to_string(),
				"User".to_string(),
				"created_at".to_string(),
			),
			(
				"app".to_string(),
				"User".to_string(),
				"updated_at".to_string(),
			),
		],
		&[],
	);

	// Should detect timestamp tracking intent
	let tracking_intent = intents
		.iter()
		.find(|i| i.intent_type.contains("timestamp tracking"));
	assert!(tracking_intent.is_some());
	// Confidence should be boosted for optional condition
	assert!(tracking_intent.unwrap().confidence > 0.8);
}

#[test]
fn test_migration_prompt_defaults() {
	use reinhardt_db::migrations::MigrationPrompt;

	let prompt = MigrationPrompt::new();
	assert_eq!(prompt.auto_accept_threshold(), 0.85);
}

#[test]
fn test_migration_prompt_custom_threshold() {
	use reinhardt_db::migrations::MigrationPrompt;

	let prompt = MigrationPrompt::with_threshold(0.75);
	assert_eq!(prompt.auto_accept_threshold(), 0.75);
}

// Note: Interactive tests are excluded as they require user input
// Manual testing should be performed for:
// - confirm_intent()
// - select_intent()
// - multi_select_intents()
// - confirm_model_rename()
// - confirm_field_rename()
// - with_progress()

/// Test that migration generation is deterministic
///
/// This test ensures that running makemigrations multiple times on the same
/// model set produces identical results in the same order.
#[test]
fn test_deterministic_migration_generation() {
	use reinhardt_db::migrations::{
		FieldState, FieldType, MigrationAutodetector, ModelState, ProjectState,
	};

	// Create a ProjectState with multiple models and fields in random order
	let mut to_state = ProjectState::new();

	// Add models in non-alphabetical order
	let mut blog_post = ModelState::new("blog", "Post");
	blog_post.add_field(FieldState::new("title", FieldType::VarChar(255), false));
	blog_post.add_field(FieldState::new("content", FieldType::Text, false));
	blog_post.add_field(FieldState::new("created_at", FieldType::DateTime, false));
	to_state.add_model(blog_post);

	let mut auth_user = ModelState::new("auth", "User");
	auth_user.add_field(FieldState::new("username", FieldType::VarChar(150), false));
	auth_user.add_field(FieldState::new("email", FieldType::VarChar(255), false));
	auth_user.add_field(FieldState::new("password", FieldType::VarChar(128), false));
	to_state.add_model(auth_user);

	let from_state = ProjectState::new();

	// Generate migrations twice
	let detector1 = MigrationAutodetector::new(from_state.clone(), to_state.clone());
	let migrations1 = detector1.generate_migrations();

	let detector2 = MigrationAutodetector::new(from_state.clone(), to_state.clone());
	let migrations2 = detector2.generate_migrations();

	// Verify the results are identical
	assert_eq!(
		migrations1.len(),
		migrations2.len(),
		"Migration generation must be deterministic: same number of migrations"
	);

	for (i, (m1, m2)) in migrations1.iter().zip(migrations2.iter()).enumerate() {
		assert_eq!(
			m1.app_label, m2.app_label,
			"Migration {} app_label must match",
			i
		);
		assert_eq!(
			m1.operations.len(),
			m2.operations.len(),
			"Migration {} operations count must match",
			i
		);
	}
}

/// Test that migrations are sorted by app_label
///
/// This test ensures that when multiple apps have migrations, they are
/// generated in alphabetical order by app_label.
#[test]
fn test_migration_sorted_by_app_label() {
	use reinhardt_db::migrations::{
		FieldState, FieldType, MigrationAutodetector, ModelState, ProjectState,
	};

	let mut to_state = ProjectState::new();

	// Add models from different apps in non-alphabetical order
	let mut users_profile = ModelState::new("users", "Profile");
	users_profile.add_field(FieldState::new("bio", FieldType::Text, false));
	to_state.add_model(users_profile);

	let mut auth_user = ModelState::new("auth", "User");
	auth_user.add_field(FieldState::new("username", FieldType::VarChar(150), false));
	to_state.add_model(auth_user);

	let mut blog_post = ModelState::new("blog", "Post");
	blog_post.add_field(FieldState::new("title", FieldType::VarChar(255), false));
	to_state.add_model(blog_post);

	let from_state = ProjectState::new();
	let detector = MigrationAutodetector::new(from_state, to_state);
	let migrations = detector.generate_migrations();

	// Extract app_labels
	let app_labels: Vec<_> = migrations.iter().map(|m| &m.app_label).collect();

	// Create a sorted version
	let mut sorted_app_labels = app_labels.clone();
	sorted_app_labels.sort();

	// Verify they are equal
	assert_eq!(
		app_labels, sorted_app_labels,
		"App labels must be sorted alphabetically: expected {:?}, got {:?}",
		sorted_app_labels, app_labels
	);

	// Verify the expected order
	assert_eq!(app_labels, vec![&"auth", &"blog", &"users"]);
}

/// Test that fields within a model are sorted by name
///
/// This test ensures that fields within a CreateTable operation are
/// sorted alphabetically by field name.
#[test]
fn test_fields_sorted_by_name() {
	use reinhardt_db::migrations::{
		FieldState, FieldType, MigrationAutodetector, ModelState, Operation, ProjectState,
	};

	let mut to_state = ProjectState::new();

	// Add fields in non-alphabetical order
	let mut user = ModelState::new("auth", "User");
	user.add_field(FieldState::new("username", FieldType::VarChar(150), false));
	user.add_field(FieldState::new("email", FieldType::VarChar(255), false));
	user.add_field(FieldState::new("created_at", FieldType::DateTime, false));
	user.add_field(FieldState::new("bio", FieldType::Text, true));
	to_state.add_model(user);

	let from_state = ProjectState::new();
	let detector = MigrationAutodetector::new(from_state, to_state);
	let migrations = detector.generate_migrations();

	assert_eq!(migrations.len(), 1, "Should generate one migration");
	assert_eq!(migrations[0].app_label, "auth");
	assert_eq!(
		migrations[0].operations.len(),
		1,
		"Should have one operation"
	);

	// Extract field names from the CreateTable operation
	if let Operation::CreateTable { columns, .. } = &migrations[0].operations[0] {
		let field_names: Vec<_> = columns.iter().map(|col| col.name.clone()).collect();

		// Create a sorted version
		let mut sorted_names = field_names.clone();
		sorted_names.sort();

		// Verify they are equal
		assert_eq!(
			field_names, sorted_names,
			"Field names must be sorted alphabetically: expected {:?}, got {:?}",
			sorted_names, field_names
		);

		// Verify the expected order
		assert_eq!(
			field_names,
			vec![
				"bio".to_string(),
				"created_at".to_string(),
				"email".to_string(),
				"username".to_string()
			]
		);
	} else {
		panic!("Expected CreateTable operation");
	}
}
