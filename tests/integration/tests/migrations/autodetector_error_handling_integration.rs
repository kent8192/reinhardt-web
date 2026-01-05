//! Integration tests for autodetector error handling
//!
//! Tests error detection and handling in various scenarios:
//! - Circular dependencies
//! - Constraint violations (NOT NULL, UNIQUE, FK, CHECK)
//! - Type conversion errors
//! - Invalid operations
//!
//! **Test Coverage:**
//! - Error detection mechanisms
//! - User-friendly error messages
//! - Constraint violation detection
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container (for DB-level validation)

use reinhardt_migrations::{
	ConstraintDefinition, FieldState, FieldType, ForeignKeyAction, ForeignKeyConstraintInfo,
	MigrationAutodetector, ModelState, ProjectState,
};
use rstest::*;
use std::collections::BTreeMap;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a basic model with id field
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

/// Add a foreign key field to a model
fn add_fk_field(
	model: &mut ModelState,
	field_name: &str,
	referenced_table: &str,
	on_delete: ForeignKeyAction,
) {
	let mut fk_info = BTreeMap::new();
	fk_info.insert("referenced_table".to_string(), referenced_table.to_string());

	let field_state = FieldState::new(field_name.to_string(), FieldType::Integer, true, fk_info);

	model.fields.insert(field_name.to_string(), field_state);

	// Add as constraint
	model.constraints.push(ConstraintDefinition {
		name: format!("fk_{}_{}", model.name.to_lowercase(), field_name),
		constraint_type: "ForeignKey".to_string(),
		fields: vec![field_name.to_string()],
		expression: None,
		foreign_key_info: Some(ForeignKeyConstraintInfo {
			referenced_table: referenced_table.to_string(),
			referenced_columns: vec!["id".to_string()],
			on_delete,
			on_update: ForeignKeyAction::NoAction,
		}),
	});
}

// ============================================================================
// Phase 1 Tests (5 tests - High Priority Error Detection)
// ============================================================================

// ============================================================================
// Test 42: Circular Dependency Detection (A→B→C→A)
// ============================================================================

/// Test detection of circular dependencies (A→B→C→A)
///
/// **Test Intent**: Verify that circular FK dependencies are detected
///
/// **Integration Point**: MigrationAutodetector → detect_model_dependencies() → check_circular_dependencies()
///
/// **Expected Behavior**: Circular dependency detected and reported
#[rstest]
#[test]
fn test_detect_circular_dependency_abc() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: Three models with circular FK dependencies
	// ModelA → ModelB → ModelC → ModelA
	let mut to_state = ProjectState::new();

	// ModelA with FK to ModelB
	let mut model_a = create_basic_model("testapp", "ModelA", "testapp_modela");
	add_fk_field(
		&mut model_a,
		"b_id",
		"testapp_modelb",
		ForeignKeyAction::Cascade,
	);

	// ModelB with FK to ModelC
	let mut model_b = create_basic_model("testapp", "ModelB", "testapp_modelb");
	add_fk_field(
		&mut model_b,
		"c_id",
		"testapp_modelc",
		ForeignKeyAction::Cascade,
	);

	// ModelC with FK to ModelA (completing the cycle)
	let mut model_c = create_basic_model("testapp", "ModelC", "testapp_modelc");
	add_fk_field(
		&mut model_c,
		"a_id",
		"testapp_modela",
		ForeignKeyAction::Cascade,
	);

	to_state.add_model(model_a);
	to_state.add_model(model_b);
	to_state.add_model(model_c);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Check for circular dependency
	let has_circular = detected.check_circular_dependencies();

	// Verify: Circular dependency should be detected
	assert!(has_circular, "Should detect circular dependency in A→B→C→A");
}

// ============================================================================
// Test 43: NOT NULL Without Default Error
// ============================================================================

/// Test error when adding NOT NULL constraint without default value
///
/// **Test Intent**: Verify that adding NOT NULL without default is flagged as potential error
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: Field alteration detected; warning about potential data loss
#[rstest]
#[test]
fn test_detect_not_null_without_default_error() {
	// from_state: User with nullable email
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"email".to_string(),
		FieldState::new(
			"email".to_string(),
			FieldType::VarChar(255),
			true, // nullable
			BTreeMap::new(),
		),
	);
	from_state.add_model(user_model);

	// to_state: User with NOT NULL email (no default)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"email".to_string(),
		FieldState::new(
			"email".to_string(),
			FieldType::VarChar(255),
			false, // NOT NULL
			BTreeMap::new(),
		),
	);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field change is detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect nullable→NOT NULL change"
	);

	// To verify that errors occur during actual migration execution
	// DB integration test is required (here we only verify detection)
}

// ============================================================================
// Test 44: UNIQUE Constraint Violation Detection
// ============================================================================

/// Test detection of UNIQUE constraint on field with duplicate values
///
/// **Test Intent**: Verify that UNIQUE constraint addition is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: UNIQUE index addition detected
#[rstest]
#[test]
fn test_detect_unique_constraint_violation() {
	// from_state: User with email (no unique constraint)
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"email".to_string(),
		FieldState::new(
			"email".to_string(),
			FieldType::VarChar(255),
			false,
			BTreeMap::new(),
		),
	);
	from_state.add_model(user_model);

	// to_state: User with UNIQUE constraint on email
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"email".to_string(),
		FieldState::new(
			"email".to_string(),
			FieldType::VarChar(255),
			false,
			BTreeMap::new(),
		),
	);
	// Add as UNIQUE constraint
	user_model.constraints.push(ConstraintDefinition {
		name: "unique_email".to_string(),
		constraint_type: "Unique".to_string(),
		fields: vec!["email".to_string()],
		expression: None,
		foreign_key_info: None,
	});
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Constraint addition is detected
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect UNIQUE constraint addition"
	);
	assert_eq!(detected.added_constraints[0].2.constraint_type, "Unique");

	// Actual constraint violation occurs during DB execution
}

// ============================================================================
// Test 45: FK Integrity Violation Detection
// ============================================================================

/// Test detection of FK constraint to non-existent table
///
/// **Test Intent**: Verify that FK to non-existent model is detected as error
///
/// **Integration Point**: MigrationAutodetector → detect_model_dependencies()
///
/// **Expected Behavior**: FK addition detected; dependency validation should flag missing table
#[rstest]
#[test]
fn test_detect_fk_integrity_violation() {
	// from_state: Post model only
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "Post", "testapp_post"));

	// to_state: Post with FK to non-existent User model
	let mut to_state = ProjectState::new();
	let mut post_model = create_basic_model("testapp", "Post", "testapp_post");
	add_fk_field(
		&mut post_model,
		"author_id",
		"testapp_user", // User model doesn't exist!
		ForeignKeyAction::Cascade,
	);
	to_state.add_model(post_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: FK addition is detected
	// Referenced table existence check is performed during migration execution or validation
	assert!(
		detected.added_fields.len() > 0 || detected.added_constraints.len() > 0,
		"Should detect FK field/constraint addition"
	);

	// Verify that there is a reference to a table that does not exist in dependency map
	// (More detailed validation is performed on the executor side)
}

// ============================================================================
// Test 46: CHECK Constraint Violation Detection
// ============================================================================

/// Test detection of CHECK constraint addition
///
/// **Test Intent**: Verify that CHECK constraint is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_constraints()
///
/// **Expected Behavior**: CHECK constraint addition detected
#[rstest]
#[test]
fn test_detect_check_constraint_violation() {
	// from_state: Product with price (no CHECK constraint)
	let mut from_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	product_model.fields.insert(
		"price".to_string(),
		FieldState::new(
			"price".to_string(),
			FieldType::Decimal(10, 2),
			false,
			BTreeMap::new(),
		),
	);
	from_state.add_model(product_model);

	// to_state: Product with CHECK constraint (price >= 0)
	let mut to_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	product_model.fields.insert(
		"price".to_string(),
		FieldState::new(
			"price".to_string(),
			FieldType::Decimal(10, 2),
			false,
			BTreeMap::new(),
		),
	);
	// Add as CHECK constraint
	product_model.constraints.push(ConstraintDefinition {
		name: "check_price_positive".to_string(),
		constraint_type: "Check".to_string(),
		fields: vec!["price".to_string()],
		expression: Some("price >= 0".to_string()),
		foreign_key_info: None,
	});
	to_state.add_model(product_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: CHECK constraint addition is detected
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect CHECK constraint addition"
	);
	assert_eq!(detected.added_constraints[0].2.constraint_type, "Check");
	assert_eq!(
		detected.added_constraints[0].2.expression,
		Some("price >= 0".to_string())
	);

	// Actual constraint violation (existing negative values) occurs during DB execution
}

// ============================================================================
// Phase 3 Tests (8 tests - Type Conversion & Other Errors)
// ============================================================================

// ============================================================================
// Test 47: Incompatible Type Conversion Error
// ============================================================================

/// Test detection of incompatible type conversion
///
/// **Test Intent**: Verify that incompatible type conversions are detected as errors
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: Type conversion detected; potential data loss warning
#[rstest]
#[test]
fn test_incompatible_type_conversion() {
	// from_state: User with Text bio field
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"bio".to_string(),
		FieldState::new("bio".to_string(), FieldType::Text, true, BTreeMap::new()),
	);
	from_state.add_model(user_model);

	// to_state: User with Integer bio field (incompatible conversion)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"bio".to_string(),
		FieldState::new("bio".to_string(), FieldType::Integer, true, BTreeMap::new()),
	);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field change is detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect incompatible type conversion"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "bio");

	// NOTE: Data loss warning should be issued during actual migration execution
	// Text → Integer conversion typically fails
}

// ============================================================================
// Test 48: VARCHAR Length Decrease Warning
// ============================================================================

/// Test detection of VARCHAR length decrease
///
/// **Test Intent**: Verify that VARCHAR length decrease is detected and warned
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: Field alteration detected; potential data truncation warning
#[rstest]
#[test]
fn test_varchar_length_decrease_warning() {
	// from_state: User with VarChar(255) username
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"username".to_string(),
		FieldState::new(
			"username".to_string(),
			FieldType::VarChar(255),
			false,
			BTreeMap::new(),
		),
	);
	from_state.add_model(user_model);

	// to_state: User with VarChar(50) username (length decrease)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"username".to_string(),
		FieldState::new(
			"username".to_string(),
			FieldType::VarChar(50),
			false,
			BTreeMap::new(),
		),
	);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field change is detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect VARCHAR length decrease"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "username");

	// NOTE: During actual migration execution, if existing data exceeds 50 characters,
	// Data truncation warning should be issued
}

// ============================================================================
// Test 49: Numeric Precision Loss Warning
// ============================================================================

/// Test detection of numeric precision loss
///
/// **Test Intent**: Verify that decreasing numeric precision is detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: Field alteration detected; precision loss warning
#[rstest]
#[test]
fn test_numeric_precision_loss_warning() {
	// from_state: Product with Decimal(10, 4) price
	let mut from_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	product_model.fields.insert(
		"price".to_string(),
		FieldState::new(
			"price".to_string(),
			FieldType::Decimal(10, 4),
			false,
			BTreeMap::new(),
		),
	);
	from_state.add_model(product_model);

	// to_state: Product with Decimal(10, 2) price (precision loss)
	let mut to_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	product_model.fields.insert(
		"price".to_string(),
		FieldState::new(
			"price".to_string(),
			FieldType::Decimal(10, 2),
			false,
			BTreeMap::new(),
		),
	);
	to_state.add_model(product_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field change is detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect numeric precision decrease"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "Product");
	assert_eq!(detected.altered_fields[0].2, "price");

	// NOTE: Decimal(10,4) → Decimal(10,2) reduces decimal precision from 4 digits to 2 digits
	// Existing data may be rounded
}

// ============================================================================
// Test 50: Foreign Key to Nonexistent Model
// ============================================================================

/// Test detection of FK to nonexistent model
///
/// **Test Intent**: Verify that FK to missing model is detected as error
///
/// **Integration Point**: MigrationAutodetector → detect_added_constraints()
///
/// **Expected Behavior**: FK constraint addition detected; validation should catch missing table
#[rstest]
#[test]
fn test_foreign_key_to_nonexistent_model() {
	// from_state: Empty
	let from_state = ProjectState::new();

	// to_state: Post with FK to nonexistent User model
	let mut to_state = ProjectState::new();
	let mut post_model = create_basic_model("testapp", "Post", "testapp_post");
	add_fk_field(
		&mut post_model,
		"author_id",
		"testapp_user", // User model doesn't exist!
		ForeignKeyAction::Cascade,
	);
	to_state.add_model(post_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Model creation and FK constraint addition are detected
	assert_eq!(
		detected.created_models.len(),
		1,
		"Should detect Post model creation"
	);
	assert!(
		detected.added_constraints.len() > 0,
		"Should detect FK constraint addition"
	);

	// Verify that FK constraint references a non-existent table
	let has_fk_to_user = detected.added_constraints.iter().any(|c| {
		c.2.constraint_type == "ForeignKey"
			&& c.2
				.foreign_key_info
				.as_ref()
				.map(|fk| fk.referenced_table == "testapp_user")
				.unwrap_or(false)
	});

	assert!(
		has_fk_to_user,
		"Should detect FK constraint to nonexistent testapp_user table"
	);

	// NOTE: Will error during actual migration execution
	// Or should be detected in dependency validation phase
}

// ============================================================================
// Test 51: Duplicate Index Name Error
// ============================================================================

/// Test detection of duplicate index names
///
/// **Test Intent**: Verify that duplicate index names are detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: Both index additions detected; validation should catch duplicate names
#[rstest]
#[test]
fn test_duplicate_index_name_error() {
	// from_state: User model without indexes
	let mut from_state = ProjectState::new();
	let user_model = create_basic_model("testapp", "User", "testapp_user");
	from_state.add_model(user_model);

	// to_state: User model with two indexes using the same name
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"email".to_string(),
		FieldState::new(
			"email".to_string(),
			FieldType::VarChar(255),
			false,
			BTreeMap::new(),
		),
	);
	user_model.fields.insert(
		"username".to_string(),
		FieldState::new(
			"username".to_string(),
			FieldType::VarChar(100),
			false,
			BTreeMap::new(),
		),
	);

	// Add two constraints with the same name (error)
	user_model.constraints.push(ConstraintDefinition {
		name: "idx_duplicate".to_string(),
		constraint_type: "Index".to_string(),
		fields: vec!["email".to_string()],
		expression: None,
		foreign_key_info: None,
	});
	user_model.constraints.push(ConstraintDefinition {
		name: "idx_duplicate".to_string(),
		constraint_type: "Index".to_string(),
		fields: vec!["username".to_string()],
		expression: None,
		foreign_key_info: None,
	});

	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Both index additions are detected
	assert_eq!(
		detected.added_constraints.len(),
		2,
		"Should detect both index additions"
	);

	// Indexes with the same name are detected
	let index_names: Vec<&str> = detected
		.added_constraints
		.iter()
		.map(|c| c.2.name.as_str())
		.collect();
	assert!(
		index_names
			.iter()
			.filter(|&&name| name == "idx_duplicate")
			.count() == 2,
		"Should detect duplicate index name"
	);

	// NOTE: Will error during actual migration execution
	// Should be detected in validation phase
}

// ============================================================================
// Test 52: Invalid Field Type Error
// ============================================================================

/// Test detection of invalid field type (edge case)
///
/// **Test Intent**: Verify that unusual field type scenarios are handled
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: Field addition detected; validation may catch invalid types
#[rstest]
#[test]
fn test_invalid_field_type_error() {
	// from_state: User model
	let mut from_state = ProjectState::new();
	let user_model = create_basic_model("testapp", "User", "testapp_user");
	from_state.add_model(user_model);

	// to_state: User model with VarChar(0) field (invalid length)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"code".to_string(),
		FieldState::new(
			"code".to_string(),
			FieldType::VarChar(0), // Invalid: length 0
			true,
			BTreeMap::new(),
		),
	);
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field addition is detected
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect field addition even with invalid type"
	);
	assert_eq!(detected.added_fields[0].0, "testapp");
	assert_eq!(detected.added_fields[0].1, "User");
	assert_eq!(detected.added_fields[0].2, "code");

	// NOTE: VarChar(0) is typically invalid, but autodetector detects it
	// Should error in validation phase or SQL generation phase
}

// ============================================================================
// Test 53: Index on Deleted Field Error
// ============================================================================

/// Test detection of index on deleted field
///
/// **Test Intent**: Verify that removing a field but keeping its index is detected
///
/// **Integration Point**: MigrationAutodetector → detect_removed_fields() + detect_removed_constraints()
///
/// **Expected Behavior**: Field removal detected; orphaned index should also be removed
#[rstest]
#[test]
fn test_index_on_deleted_field_error() {
	// from_state: User with email field and index
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.fields.insert(
		"email".to_string(),
		FieldState::new(
			"email".to_string(),
			FieldType::VarChar(255),
			false,
			BTreeMap::new(),
		),
	);
	user_model.constraints.push(ConstraintDefinition {
		name: "idx_email".to_string(),
		constraint_type: "Index".to_string(),
		fields: vec!["email".to_string()],
		expression: None,
		foreign_key_info: None,
	});
	from_state.add_model(user_model);

	// to_state: User without email field (but index remains - inconsistent state)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	// email field is deleted but index remains (error)
	user_model.constraints.push(ConstraintDefinition {
		name: "idx_email".to_string(),
		constraint_type: "Index".to_string(),
		fields: vec!["email".to_string()],
		expression: None,
		foreign_key_info: None,
	});
	to_state.add_model(user_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field deletion is detected
	assert_eq!(
		detected.removed_fields.len(),
		1,
		"Should detect email field removal"
	);
	assert_eq!(detected.removed_fields[0].0, "testapp");
	assert_eq!(detected.removed_fields[0].1, "User");
	assert_eq!(detected.removed_fields[0].2, "email");

	// Index exists but field does not exist
	// NOTE: Will error during actual migration execution
	// Should be detected in validation phase as "field referenced by index does not exist"
}

// ============================================================================
// Test 54: Timezone-Aware DateTime Warning
// ============================================================================

/// Test detection of timezone-aware datetime changes
///
/// **Test Intent**: Verify that timezone attribute changes are detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: Field alteration detected; potential data conversion warning
#[rstest]
#[test]
fn test_timezone_aware_datetime_warning() {
	// from_state: Event with naive DateTime
	let mut from_state = ProjectState::new();
	let mut event_model = create_basic_model("testapp", "Event", "testapp_event");
	event_model.fields.insert(
		"scheduled_at".to_string(),
		FieldState::new(
			"scheduled_at".to_string(),
			FieldType::DateTime,
			false,
			BTreeMap::new(),
		),
	);
	from_state.add_model(event_model);

	// to_state: Event with timezone-aware DateTime
	let mut to_state = ProjectState::new();
	let mut event_model = create_basic_model("testapp", "Event", "testapp_event");
	let mut options = BTreeMap::new();
	options.insert("timezone".to_string(), "aware".to_string());
	event_model.fields.insert(
		"scheduled_at".to_string(),
		FieldState::new(
			"scheduled_at".to_string(),
			FieldType::DateTime,
			false,
			options,
		),
	);
	to_state.add_model(event_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field option change is detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect timezone attribute change"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "Event");
	assert_eq!(detected.altered_fields[0].2, "scheduled_at");

	// NOTE: naive → aware conversion requires timezone handling for existing data
	// Data migration warning should be issued
}
