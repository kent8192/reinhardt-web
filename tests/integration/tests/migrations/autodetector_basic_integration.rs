//! Integration tests for basic autodetector functionality
//!
//! Tests core model and field change detection:
//! - Model creation and deletion
//! - Field addition, removal, and alteration
//! - Constraint changes (NOT NULL, UNIQUE, DEFAULT)
//! - Table renaming
//! - Multiple changes on same model
//!
//! **Test Coverage:**
//! - Single and multiple model operations
//! - Field type and attribute modifications
//! - Composite primary keys
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//! - mysql_container: MySQL database container (for cross-DB validation)

use reinhardt_db::migrations::{
	FieldState, FieldType, IndexDefinition, MigrationAutodetector, ModelState, ProjectState,
};
use rstest::*;
use std::collections::BTreeMap;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a simple model with basic fields
fn create_basic_model(app: &str, name: &str, table_name: &str) -> ModelState {
	let mut fields = BTreeMap::new();
	fields.insert(
		"id".to_string(),
		FieldState::new("id".to_string(), FieldType::Integer, false, BTreeMap::new()),
	);

	ModelState {
		app_label: app.to_string(),
		name: name.to_string(),
		table_name: table_name.to_string(),
		fields,
		options: BTreeMap::new(),
		base_model: None,
		inheritance_type: None,
		discriminator_column: None,
		indexes: vec![],
		constraints: vec![],
		many_to_many_fields: vec![],
	}
}

/// Add a field to an existing model
fn add_field_to_model(
	model: &mut ModelState,
	field_name: &str,
	field_type: FieldType,
	nullable: bool,
) {
	model.fields.insert(
		field_name.to_string(),
		FieldState::new(
			field_name.to_string(),
			field_type,
			nullable,
			BTreeMap::new(),
		),
	);
}

// ============================================================================
// Test 1: Single Model Creation Detection
// ============================================================================

/// Test detection of single model creation
///
/// **Test Intent**: Verify that creating a single model is detected correctly
///
/// **Integration Point**: MigrationAutodetector → detect_created_models()
///
/// **Expected Behavior**: DetectedChanges contains the newly created model
#[rstest]
fn test_detect_create_single_model() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: one model
	let mut to_state = ProjectState::new();
	let user_model = create_basic_model("testapp", "User", "testapp_user");
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.created_models.len(),
		1,
		"Should detect 1 created model"
	);
	assert_eq!(detected.created_models[0].0, "testapp");
	assert_eq!(detected.created_models[0].1, "User");
}

// ============================================================================
// Test 2: Multiple Models Creation Detection
// ============================================================================

/// Test detection of multiple models creation at once
///
/// **Test Intent**: Verify that multiple models created simultaneously are all detected
///
/// **Integration Point**: MigrationAutodetector → detect_created_models()
///
/// **Expected Behavior**: DetectedChanges contains all newly created models
#[rstest]
fn test_detect_create_multiple_models() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: three models
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	to_state.add_model(create_basic_model("testapp", "Post", "testapp_post"));
	to_state.add_model(create_basic_model("testapp", "Comment", "testapp_comment"));

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.created_models.len(),
		3,
		"Should detect 3 created models"
	);

	let created_names: Vec<&str> = detected
		.created_models
		.iter()
		.map(|(_, name)| name.as_str())
		.collect();
	assert!(created_names.contains(&"User"));
	assert!(created_names.contains(&"Post"));
	assert!(created_names.contains(&"Comment"));
}

// ============================================================================
// Test 3: Single Model Deletion Detection
// ============================================================================

/// Test detection of single model deletion
///
/// **Test Intent**: Verify that deleting a single model is detected correctly
///
/// **Integration Point**: MigrationAutodetector → detect_deleted_models()
///
/// **Expected Behavior**: DetectedChanges contains the deleted model
#[rstest]
fn test_detect_delete_single_model() {
	// from_state: one model
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));

	// to_state: empty
	let to_state = ProjectState::new();

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.deleted_models.len(),
		1,
		"Should detect 1 deleted model"
	);
	assert_eq!(detected.deleted_models[0].0, "testapp");
	assert_eq!(detected.deleted_models[0].1, "User");
}

// ============================================================================
// Test 4: Multiple Models Deletion Detection
// ============================================================================

/// Test detection of multiple models deletion
///
/// **Test Intent**: Verify that deleting multiple models is detected correctly
///
/// **Integration Point**: MigrationAutodetector → detect_deleted_models()
///
/// **Expected Behavior**: DetectedChanges contains all deleted models
#[rstest]
fn test_detect_delete_multiple_models() {
	// from_state: three models
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	from_state.add_model(create_basic_model("testapp", "Post", "testapp_post"));
	from_state.add_model(create_basic_model("testapp", "Comment", "testapp_comment"));

	// to_state: empty
	let to_state = ProjectState::new();

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.deleted_models.len(),
		3,
		"Should detect 3 deleted models"
	);

	let deleted_names: Vec<&str> = detected
		.deleted_models
		.iter()
		.map(|(_, name)| name.as_str())
		.collect();
	assert!(deleted_names.contains(&"User"));
	assert!(deleted_names.contains(&"Post"));
	assert!(deleted_names.contains(&"Comment"));
}

// ============================================================================
// Test 5: Add Field to Model Detection
// ============================================================================

/// Test detection of field addition to existing model
///
/// **Test Intent**: Verify that adding a field to a model is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: DetectedChanges contains the added field
#[rstest]
fn test_detect_add_field_to_model() {
	// from_state: User with only id
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));

	// to_state: User with id + email
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect 1 added field"
	);
	assert_eq!(detected.added_fields[0].0, "testapp");
	assert_eq!(detected.added_fields[0].1, "User");
	assert_eq!(detected.added_fields[0].2, "email");
}

// ============================================================================
// Test 6: Remove Field from Model Detection
// ============================================================================

/// Test detection of field removal from existing model
///
/// **Test Intent**: Verify that removing a field from a model is detected
///
/// **Integration Point**: MigrationAutodetector → detect_removed_fields()
///
/// **Expected Behavior**: DetectedChanges contains the removed field
#[rstest]
fn test_detect_remove_field_from_model() {
	// from_state: User with id + email
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	from_state.add_model(user_model);

	// to_state: User with only id
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "User", "testapp_user"));

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.removed_fields.len(),
		1,
		"Should detect 1 removed field"
	);
	assert_eq!(detected.removed_fields[0].0, "testapp");
	assert_eq!(detected.removed_fields[0].1, "User");
	assert_eq!(detected.removed_fields[0].2, "email");
}

// ============================================================================
// Test 7: Alter Field Type Detection
// ============================================================================

/// Test detection of field type change
///
/// **Test Intent**: Verify that changing a field's type is detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: DetectedChanges contains the altered field
#[rstest]
fn test_detect_alter_field_type() {
	// from_state: User with email VARCHAR(255)
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	from_state.add_model(user_model);

	// to_state: User with email TEXT
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::Text, false);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect 1 altered field"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "email");
}

// ============================================================================
// Test 8: Alter Field Nullable Detection
// ============================================================================

/// Test detection of field nullable attribute change
///
/// **Test Intent**: Verify that changing a field's nullable attribute is detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: DetectedChanges contains the altered field
#[rstest]
fn test_detect_alter_field_nullable() {
	// from_state: User with email nullable=false
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	from_state.add_model(user_model);

	// to_state: User with email nullable=true
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), true);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect 1 altered field (nullable change)"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "email");
}

// ============================================================================
// Test 9: Add NOT NULL Constraint Detection
// ============================================================================

/// Test detection of NOT NULL constraint addition to existing field
///
/// **Test Intent**: Verify that changing a field from nullable to NOT NULL is detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: DetectedChanges contains the altered field with nullable change
#[rstest]
fn test_detect_add_not_null_constraint() {
	// from_state: User with email nullable=true
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), true);
	from_state.add_model(user_model);

	// to_state: User with email nullable=false (NOT NULL)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect 1 altered field (NOT NULL constraint added)"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "email");
}

// ============================================================================
// Test 10: Add UNIQUE Constraint Detection
// ============================================================================

/// Test detection of UNIQUE constraint addition via index
///
/// **Test Intent**: Verify that adding a UNIQUE index is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: DetectedChanges contains the added unique index
#[rstest]
fn test_detect_add_unique_constraint() {
	// from_state: User without unique index on email
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	from_state.add_model(user_model);

	// to_state: User with unique index on email
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	user_model.indexes.push(IndexDefinition {
		name: "idx_user_email_unique".to_string(),
		fields: vec!["email".to_string()],
		unique: true,
	});
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_indexes.len(),
		1,
		"Should detect 1 added unique index"
	);
	assert_eq!(detected.added_indexes[0].0, "testapp");
	assert_eq!(detected.added_indexes[0].1, "User");
	assert_eq!(detected.added_indexes[0].2.name, "idx_user_email_unique");
	assert!(detected.added_indexes[0].2.unique, "Index should be unique");
}

// ============================================================================
// Test 11: Change Default Value Detection
// ============================================================================

/// Test detection of default value change
///
/// **Test Intent**: Verify that changing a field's default value is detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: DetectedChanges contains the altered field
#[rstest]
fn test_detect_change_default_value() {
	// from_state: User with status default='active'
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	let mut params = BTreeMap::new();
	params.insert("default".to_string(), "active".to_string());
	user_model.fields.insert(
		"status".to_string(),
		FieldState::new("status".to_string(), FieldType::VarChar(50), false, params),
	);
	from_state.add_model(user_model);

	// to_state: User with status default='pending'
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	let mut params = BTreeMap::new();
	params.insert("default".to_string(), "pending".to_string());
	user_model.fields.insert(
		"status".to_string(),
		FieldState::new("status".to_string(), FieldType::VarChar(50), false, params),
	);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect 1 altered field (default value change)"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "status");
}

// ============================================================================
// Test 12: Rename Table Detection
// ============================================================================

/// Test detection of table name change
///
/// **Test Intent**: Verify that changing a model's table name is detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields() or model options
///
/// **Expected Behavior**: Detects table rename or recreate operation
#[rstest]
fn test_detect_rename_table() {
	// from_state: User with table 'testapp_user'
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));

	// to_state: User with table 'testapp_users' (plural)
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "User", "testapp_users"));

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: When table name changes, it may be detected as deletion + creation
	// Or, it may be detected as table_name change
	let total_changes = detected.created_models.len()
		+ detected.deleted_models.len()
		+ detected.renamed_models.len();
	assert!(
		total_changes > 0,
		"Should detect table name change (as rename, delete+create, or other)"
	);
}

// ============================================================================
// Test 13: Composite Primary Key Detection
// ============================================================================

/// Test detection of composite primary key addition
///
/// **Test Intent**: Verify that adding multiple primary key fields is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: Multiple PK fields are detected as additions
#[rstest]
fn test_detect_composite_primary_key() {
	// from_state: UserRole with single id field
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model(
		"testapp",
		"UserRole",
		"testapp_userrole",
	));

	// to_state: UserRole with composite PK (user_id + role_id)
	let mut to_state = ProjectState::new();
	let mut model = ModelState {
		app_label: "testapp".to_string(),
		name: "UserRole".to_string(),
		table_name: "testapp_userrole".to_string(),
		fields: BTreeMap::new(),
		options: BTreeMap::new(),
		base_model: None,
		inheritance_type: None,
		discriminator_column: None,
		indexes: vec![],
		constraints: vec![],
		many_to_many_fields: vec![],
	};
	// Fields that compose the composite primary key
	add_field_to_model(&mut model, "user_id", FieldType::Integer, false);
	add_field_to_model(&mut model, "role_id", FieldType::Integer, false);
	to_state.add_model(model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Confirm new fields were added
	assert!(
		detected.added_fields.len() >= 2,
		"Should detect at least 2 added fields for composite PK"
	);
}

// ============================================================================
// Test 14: Add Field with Default Detection
// ============================================================================

/// Test detection of field addition with default value
///
/// **Test Intent**: Verify that adding a field with a default value is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: DetectedChanges contains the added field with default
#[rstest]
fn test_detect_add_field_with_default() {
	// from_state: User without created_at
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));

	// to_state: User with created_at (default=NOW())
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	let mut params = BTreeMap::new();
	params.insert("default".to_string(), "NOW()".to_string());
	user_model.fields.insert(
		"created_at".to_string(),
		FieldState::new("created_at".to_string(), FieldType::DateTime, false, params),
	);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect 1 added field with default"
	);
	assert_eq!(detected.added_fields[0].0, "testapp");
	assert_eq!(detected.added_fields[0].1, "User");
	assert_eq!(detected.added_fields[0].2, "created_at");
}

// ============================================================================
// Test 15: Multiple Changes Same Model Detection
// ============================================================================

/// Test detection of multiple changes to the same model simultaneously
///
/// **Test Intent**: Verify that multiple changes to one model are all detected
///
/// **Integration Point**: MigrationAutodetector → multiple detect_* methods
///
/// **Expected Behavior**: All changes (add field, remove field, alter field) detected
#[rstest]
fn test_detect_multiple_changes_same_model() {
	// from_state: User with id, email, age
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::VarChar(255), false);
	add_field_to_model(&mut user_model, "age", FieldType::Integer, true);
	from_state.add_model(user_model);

	// to_state: User with id, email (TEXT), username (new)
	// Changes: age deletion, email type change, username addition
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field_to_model(&mut user_model, "email", FieldType::Text, false); // Type change
	add_field_to_model(&mut user_model, "username", FieldType::VarChar(100), false); // New
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect 1 added field (username)"
	);
	assert_eq!(
		detected.removed_fields.len(),
		1,
		"Should detect 1 removed field (age)"
	);
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect 1 altered field (email type change)"
	);
}
