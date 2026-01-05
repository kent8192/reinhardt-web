//! Integration tests for constraint and index detection in autodetector
//!
//! Tests constraint and index-related change detection:
//! - Single and composite indexes
//! - CHECK constraints
//! - Partial indexes (PostgreSQL)
//! - Database-specific types (ENUM, Array, JSONB for PostgreSQL)
//! - Expression-based indexes
//! - UNIQUE TOGETHER constraints
//!
//! **Test Coverage:**
//! - Index addition and removal
//! - Database-specific constraint types
//! - Complex index patterns
//!
//! **Fixtures Used:**
//! - None (pure ProjectState manipulation)

use reinhardt_migrations::{
	ConstraintDefinition, FieldState, FieldType, IndexDefinition, MigrationAutodetector,
	ModelState, ProjectState,
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

/// Add a field to a model
fn add_field(model: &mut ModelState, name: &str, field_type: FieldType, nullable: bool) {
	model.fields.insert(
		name.to_string(),
		FieldState::new(name.to_string(), field_type, nullable, BTreeMap::new()),
	);
}

// ============================================================================
// Test 24: Add Single Column Index Detection
// ============================================================================

/// Test detection of single column index addition
///
/// **Test Intent**: Verify that adding an index on a single column is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: Index addition detected
#[rstest]
#[test]
fn test_detect_add_single_column_index() {
	// from_state: User without index on email
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), false);
	from_state.add_model(user_model);

	// to_state: User with index on email
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), false);
	user_model.indexes.push(IndexDefinition {
		name: "idx_user_email".to_string(),
		fields: vec!["email".to_string()],
		unique: false,
	});
	to_state.add_model(user_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_indexes.len(),
		1,
		"Should detect single column index addition"
	);
	assert_eq!(detected.added_indexes[0].2.name, "idx_user_email");
	assert_eq!(detected.added_indexes[0].2.fields, vec!["email"]);
}

// ============================================================================
// Test 25: Add Composite Index Detection
// ============================================================================

/// Test detection of composite (multi-column) index addition
///
/// **Test Intent**: Verify that adding a composite index is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: Composite index addition detected
#[rstest]
#[test]
fn test_detect_add_composite_index() {
	// from_state: User without composite index
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(
		&mut user_model,
		"first_name",
		FieldType::VarChar(100),
		false,
	);
	add_field(&mut user_model, "last_name", FieldType::VarChar(100), false);
	from_state.add_model(user_model);

	// to_state: User with composite index on (first_name, last_name)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(
		&mut user_model,
		"first_name",
		FieldType::VarChar(100),
		false,
	);
	add_field(&mut user_model, "last_name", FieldType::VarChar(100), false);
	user_model.indexes.push(IndexDefinition {
		name: "idx_user_name".to_string(),
		fields: vec!["first_name".to_string(), "last_name".to_string()],
		unique: false,
	});
	to_state.add_model(user_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_indexes.len(),
		1,
		"Should detect composite index addition"
	);
	assert_eq!(detected.added_indexes[0].2.name, "idx_user_name");
	assert_eq!(
		detected.added_indexes[0].2.fields,
		vec!["first_name", "last_name"]
	);
}

// ============================================================================
// Test 26: Remove Index Detection
// ============================================================================

/// Test detection of index removal
///
/// **Test Intent**: Verify that removing an index is detected
///
/// **Integration Point**: MigrationAutodetector → detect_removed_indexes()
///
/// **Expected Behavior**: Index removal detected
#[rstest]
#[test]
fn test_detect_remove_index() {
	// from_state: User with index on email
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), false);
	user_model.indexes.push(IndexDefinition {
		name: "idx_user_email".to_string(),
		fields: vec!["email".to_string()],
		unique: false,
	});
	from_state.add_model(user_model);

	// to_state: User without index
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), false);
	to_state.add_model(user_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.removed_indexes.len(),
		1,
		"Should detect index removal"
	);
	assert_eq!(detected.removed_indexes[0].2.name, "idx_user_email");
}

// ============================================================================
// Test 27: Add CHECK Constraint (PostgreSQL)
// ============================================================================

/// Test detection of CHECK constraint addition (PostgreSQL-specific)
///
/// **Test Intent**: Verify that CHECK constraint addition is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_constraints()
///
/// **Expected Behavior**: CHECK constraint addition detected
#[rstest]
#[test]
fn test_detect_add_check_constraint_postgres() {
	// from_state: Product without CHECK constraint
	let mut from_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	add_field(
		&mut product_model,
		"price",
		FieldType::Decimal(10, 2),
		false,
	);
	from_state.add_model(product_model);

	// to_state: Product with CHECK (price > 0)
	let mut to_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	add_field(
		&mut product_model,
		"price",
		FieldType::Decimal(10, 2),
		false,
	);
	product_model.constraints.push(ConstraintDefinition {
		name: "check_price_positive".to_string(),
		constraint_type: "Check".to_string(),
		fields: vec!["price".to_string()],
		expression: Some("price > 0".to_string()),
		foreign_key_info: None,
	});
	to_state.add_model(product_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect CHECK constraint addition"
	);
	assert_eq!(detected.added_constraints[0].2.constraint_type, "Check");
}

// ============================================================================
// Test 28: Add Partial Index (PostgreSQL)
// ============================================================================

/// Test detection of partial index addition (PostgreSQL WHERE clause)
///
/// **Test Intent**: Verify that partial index with filter is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: Partial index addition detected
///
/// **Note**: Partial index requires condition in IndexDefinition,
/// but current implementation treats it as regular index for simplicity
#[rstest]
#[test]
fn test_detect_add_partial_index_postgres() {
	// from_state: User without partial index
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), true);
	add_field(&mut user_model, "is_active", FieldType::Boolean, false);
	from_state.add_model(user_model);

	// to_state: User with partial index (WHERE is_active = true)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), true);
	add_field(&mut user_model, "is_active", FieldType::Boolean, false);
	// Represent partial index (WHERE is_active = true)
	// Note: If IndexDefinition doesn't have condition field, represent it via comment
	user_model.indexes.push(IndexDefinition {
		name: "idx_active_users_email".to_string(),
		fields: vec!["email".to_string()],
		unique: false,
	});
	to_state.add_model(user_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_indexes.len(),
		1,
		"Should detect partial index addition"
	);
}

// ============================================================================
// Test 29: Add ENUM Field (PostgreSQL)
// ============================================================================

/// Test detection of ENUM field addition (PostgreSQL-specific)
///
/// **Test Intent**: Verify that ENUM type field addition is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: ENUM field addition detected
#[rstest]
#[test]
fn test_detect_add_enum_field_postgres() {
	// from_state: User without status field
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));

	// to_state: User with status ENUM field
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	// ENUM type (PostgreSQL)
	add_field(
		&mut user_model,
		"status",
		FieldType::Custom("user_status_enum".to_string()),
		false,
	);
	to_state.add_model(user_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect ENUM field addition"
	);
	assert_eq!(detected.added_fields[0].2, "status");
}

// ============================================================================
// Test 30: Add Array Field (PostgreSQL)
// ============================================================================

/// Test detection of array field addition (PostgreSQL-specific)
///
/// **Test Intent**: Verify that array type field addition is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: Array field addition detected
#[rstest]
#[test]
fn test_detect_add_array_field_postgres() {
	// from_state: Post without tags
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "Post", "testapp_post"));

	// to_state: Post with tags array
	let mut to_state = ProjectState::new();
	let mut post_model = create_basic_model("testapp", "Post", "testapp_post");
	// Array type (PostgreSQL): text[]
	add_field(
		&mut post_model,
		"tags",
		FieldType::Custom("text[]".to_string()),
		true,
	);
	to_state.add_model(post_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect array field addition"
	);
	assert_eq!(detected.added_fields[0].2, "tags");
}

// ============================================================================
// Test 31: Add JSONB Field (PostgreSQL)
// ============================================================================

/// Test detection of JSONB field addition (PostgreSQL-specific)
///
/// **Test Intent**: Verify that JSONB type field addition is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: JSONB field addition detected
#[rstest]
#[test]
fn test_detect_add_jsonb_field_postgres() {
	// from_state: Product without metadata
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "Product", "testapp_product"));

	// to_state: Product with metadata JSONB field
	let mut to_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	// JSONB type (PostgreSQL)
	add_field(
		&mut product_model,
		"metadata",
		FieldType::Custom("jsonb".to_string()),
		true,
	);
	to_state.add_model(product_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect JSONB field addition"
	);
	assert_eq!(detected.added_fields[0].2, "metadata");
}

// ============================================================================
// Test 32: Expression-Based Index Detection
// ============================================================================

/// Test detection of expression-based index
///
/// **Test Intent**: Verify that functional/expression index is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: Expression index detected
///
/// **Note**: Current IndexDefinition only supports column names.
/// Expression-based indexes are represented via comments for future extension
#[rstest]
#[test]
fn test_detect_index_with_expression() {
	// from_state: User without expression index
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), false);
	from_state.add_model(user_model);

	// to_state: User with index on LOWER(email)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "email", FieldType::VarChar(255), false);
	// Expression-based index: CREATE INDEX idx_email_lower ON users(LOWER(email))
	// Current IndexDefinition only supports column names, so simplified
	user_model.indexes.push(IndexDefinition {
		name: "idx_email_lower".to_string(),
		fields: vec!["email".to_string()], // Actually LOWER(email)
		unique: false,
	});
	to_state.add_model(user_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_indexes.len(),
		1,
		"Should detect expression-based index"
	);
}

// ============================================================================
// Test 33: UNIQUE TOGETHER Constraint Detection
// ============================================================================

/// Test detection of UNIQUE TOGETHER constraint
///
/// **Test Intent**: Verify that composite unique constraint is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_constraints()
///
/// **Expected Behavior**: UNIQUE constraint on multiple columns detected
#[rstest]
#[test]
fn test_detect_unique_together_constraint() {
	// from_state: UserRole without unique constraint
	let mut from_state = ProjectState::new();
	let mut userrole_model = create_basic_model("testapp", "UserRole", "testapp_userrole");
	add_field(&mut userrole_model, "user_id", FieldType::Integer, false);
	add_field(&mut userrole_model, "role_id", FieldType::Integer, false);
	from_state.add_model(userrole_model);

	// to_state: UserRole with UNIQUE(user_id, role_id)
	let mut to_state = ProjectState::new();
	let mut userrole_model = create_basic_model("testapp", "UserRole", "testapp_userrole");
	add_field(&mut userrole_model, "user_id", FieldType::Integer, false);
	add_field(&mut userrole_model, "role_id", FieldType::Integer, false);
	// UNIQUE TOGETHER constraint
	userrole_model.constraints.push(ConstraintDefinition {
		name: "unique_user_role".to_string(),
		constraint_type: "Unique".to_string(),
		fields: vec!["user_id".to_string(), "role_id".to_string()],
		expression: None,
		foreign_key_info: None,
	});
	to_state.add_model(userrole_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect UNIQUE TOGETHER constraint"
	);
	assert_eq!(detected.added_constraints[0].2.constraint_type, "Unique");
	assert_eq!(
		detected.added_constraints[0].2.fields,
		vec!["user_id", "role_id"]
	);
}
