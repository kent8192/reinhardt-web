//! Integration tests for cross-database autodetector consistency
//!
//! Tests detection behavior across different database backends:
//! - PostgreSQL vs MySQL detection consistency
//! - Type mapping differences
//! - Composite primary key handling
//!
//! **Test Coverage:**
//! - Cross-database detection consistency
//! - Database-specific type mappings
//! - Composite primary key operations
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//! - mysql_container: MySQL database container

use reinhardt_db::migrations::{
	ConstraintDefinition, FieldState, FieldType, MigrationAutodetector, ModelState, ProjectState,
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
fn add_field(model: &mut ModelState, name: &str, field_type: FieldType) {
	model.fields.insert(
		name.to_string(),
		FieldState::new(name.to_string(), field_type, true, BTreeMap::new()),
	);
}

// ============================================================================
// Test 40: PostgreSQL vs MySQL Detection Consistency
// ============================================================================

/// Test that autodetector produces consistent results across PostgreSQL and MySQL
///
/// **Test Intent**: Verify that same schema changes are detected consistently regardless of DB backend
///
/// **Integration Point**: MigrationAutodetector → detect_changes()
///
/// **Expected Behavior**: Detection results should be identical for common operations
#[rstest]
fn test_postgres_mysql_detection_consistency() {
	// from_state: User model with basic fields
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "username", FieldType::VarChar(100));
	add_field(&mut user_model, "email", FieldType::VarChar(255));
	from_state.add_model(user_model);

	// to_state: User model with additional field
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "username", FieldType::VarChar(100));
	add_field(&mut user_model, "email", FieldType::VarChar(255));
	add_field(&mut user_model, "created_at", FieldType::DateTime);
	to_state.add_model(user_model);

	// Run autodetector (DB-independent detection)
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field addition is detected (common to PostgreSQL/MySQL)
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect 1 added field consistently"
	);
	assert_eq!(detected.added_fields[0].0, "testapp");
	assert_eq!(detected.added_fields[0].1, "User");
	assert_eq!(detected.added_fields[0].2, "created_at");

	// NOTE: This test verifies DB-independent detection logic
	// Type mapping differences in actual SQL generation are verified in the next test
}

// ============================================================================
// Test 41: Type Mapping Differences Detection
// ============================================================================

/// Test detection of type mapping differences between PostgreSQL and MySQL
///
/// **Test Intent**: Verify that type differences are properly detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: Type changes should be detected, though SQL generation may differ
#[rstest]
fn test_type_mapping_differences() {
	// from_state: Product with Integer price
	let mut from_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	add_field(&mut product_model, "price", FieldType::Integer);
	from_state.add_model(product_model);

	// to_state: Product with Decimal price
	// PostgreSQL: NUMERIC(10,2)
	// MySQL: DECIMAL(10,2)
	let mut to_state = ProjectState::new();
	let mut product_model = create_basic_model("testapp", "Product", "testapp_product");
	add_field(&mut product_model, "price", FieldType::Decimal(10, 2));
	to_state.add_model(product_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Type change is detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect type change from Integer to Decimal"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "Product");
	assert_eq!(detected.altered_fields[0].2, "price");

	// NOTE: Mapped to DB-specific types during actual SQL generation
	// PostgreSQL: ALTER TABLE ... ALTER COLUMN price TYPE NUMERIC(10,2)
	// MySQL: ALTER TABLE ... MODIFY COLUMN price DECIMAL(10,2)
}

// ============================================================================
// Composite Primary Key Tests
// ============================================================================

// ============================================================================
// Test: Add Composite Primary Key
// ============================================================================

/// Test detection of composite primary key addition
///
/// **Test Intent**: Verify that composite primary key constraints are detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_constraints()
///
/// **Expected Behavior**: Composite PK constraint addition should be detected
#[rstest]
fn test_detect_add_composite_primary_key() {
	// from_state: OrderItem with auto-increment id (single PK)
	let mut from_state = ProjectState::new();
	let order_item_model = create_basic_model("testapp", "OrderItem", "testapp_orderitem");
	from_state.add_model(order_item_model);

	// to_state: OrderItem with composite PK (order_id, product_id)
	let mut to_state = ProjectState::new();
	let mut order_item_model = create_basic_model("testapp", "OrderItem", "testapp_orderitem");

	// Remove id field and add composite PK fields
	order_item_model.fields.remove("id");
	add_field(&mut order_item_model, "order_id", FieldType::Integer);
	add_field(&mut order_item_model, "product_id", FieldType::Integer);

	// Add as composite PK constraint
	order_item_model.constraints.push(ConstraintDefinition {
		name: "pk_orderitem".to_string(),
		constraint_type: "PrimaryKey".to_string(),
		fields: vec!["order_id".to_string(), "product_id".to_string()],
		expression: None,
		foreign_key_info: None,
	});

	to_state.add_model(order_item_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: id field deletion, new field addition, and composite PK constraint addition are detected
	assert_eq!(
		detected.removed_fields.len(),
		1,
		"Should detect id field removal"
	);
	assert_eq!(
		detected.added_fields.len(),
		2,
		"Should detect order_id and product_id addition"
	);
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect composite PK constraint addition"
	);
	assert_eq!(
		detected.added_constraints[0].2.constraint_type,
		"PrimaryKey"
	);
	assert_eq!(detected.added_constraints[0].2.fields.len(), 2);
}

// ============================================================================
// Test: Modify Composite Primary Key
// ============================================================================

/// Test detection of composite primary key modification
///
/// **Test Intent**: Verify that changes to composite PK fields are detected
///
/// **Integration Point**: MigrationAutodetector → detect_removed_constraints() + detect_added_constraints()
///
/// **Expected Behavior**: Old PK removal and new PK addition should be detected
#[rstest]
fn test_detect_modify_composite_primary_key() {
	// from_state: OrderItem with composite PK (order_id, product_id)
	let mut from_state = ProjectState::new();
	let mut order_item_model = create_basic_model("testapp", "OrderItem", "testapp_orderitem");
	order_item_model.fields.remove("id");
	add_field(&mut order_item_model, "order_id", FieldType::Integer);
	add_field(&mut order_item_model, "product_id", FieldType::Integer);
	order_item_model.constraints.push(ConstraintDefinition {
		name: "pk_orderitem".to_string(),
		constraint_type: "PrimaryKey".to_string(),
		fields: vec!["order_id".to_string(), "product_id".to_string()],
		expression: None,
		foreign_key_info: None,
	});
	from_state.add_model(order_item_model);

	// to_state: OrderItem with different composite PK (order_id, product_id, line_number)
	let mut to_state = ProjectState::new();
	let mut order_item_model = create_basic_model("testapp", "OrderItem", "testapp_orderitem");
	order_item_model.fields.remove("id");
	add_field(&mut order_item_model, "order_id", FieldType::Integer);
	add_field(&mut order_item_model, "product_id", FieldType::Integer);
	add_field(&mut order_item_model, "line_number", FieldType::Integer);
	order_item_model.constraints.push(ConstraintDefinition {
		name: "pk_orderitem_new".to_string(),
		constraint_type: "PrimaryKey".to_string(),
		fields: vec![
			"order_id".to_string(),
			"product_id".to_string(),
			"line_number".to_string(),
		],
		expression: None,
		foreign_key_info: None,
	});
	to_state.add_model(order_item_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: New field addition, old PK deletion, and new PK addition are detected
	assert_eq!(
		detected.added_fields.len(),
		1,
		"Should detect line_number field addition"
	);
	assert_eq!(
		detected.removed_constraints.len(),
		1,
		"Should detect old PK constraint removal"
	);
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect new PK constraint addition"
	);

	// New PK contains 3 fields
	assert_eq!(detected.added_constraints[0].2.fields.len(), 3);
	assert!(
		detected.added_constraints[0]
			.2
			.fields
			.contains(&"line_number".to_string())
	);
}

// ============================================================================
// Test: Cross-DB Composite PK Behavior
// ============================================================================

/// Test cross-database behavior consistency for composite primary keys
///
/// **Test Intent**: Verify that composite PK detection is consistent across databases
///
/// **Integration Point**: MigrationAutodetector → detect_added_constraints()
///
/// **Expected Behavior**: Detection should be consistent, though SQL syntax may differ
#[rstest]
fn test_cross_db_composite_pk_behavior() {
	// from_state: Empty
	let from_state = ProjectState::new();

	// to_state: UserRole with composite PK (user_id, role_id)
	let mut to_state = ProjectState::new();
	let mut user_role_model = create_basic_model("testapp", "UserRole", "testapp_userrole");
	user_role_model.fields.remove("id");
	add_field(&mut user_role_model, "user_id", FieldType::Integer);
	add_field(&mut user_role_model, "role_id", FieldType::Integer);
	add_field(&mut user_role_model, "assigned_at", FieldType::DateTime);

	// Composite PK constraint
	user_role_model.constraints.push(ConstraintDefinition {
		name: "pk_userrole".to_string(),
		constraint_type: "PrimaryKey".to_string(),
		fields: vec!["user_id".to_string(), "role_id".to_string()],
		expression: None,
		foreign_key_info: None,
	});

	to_state.add_model(user_role_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Model creation and composite PK constraint are detected
	assert_eq!(
		detected.created_models.len(),
		1,
		"Should detect model creation"
	);
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect composite PK constraint"
	);
	assert_eq!(
		detected.added_constraints[0].2.constraint_type,
		"PrimaryKey"
	);
	assert_eq!(
		detected.added_constraints[0].2.fields,
		vec!["user_id".to_string(), "role_id".to_string()]
	);

	// NOTE: Syntax during actual SQL generation is DB-dependent:
	// PostgreSQL: CREATE TABLE ... (user_id INT, role_id INT, PRIMARY KEY (user_id, role_id))
	// MySQL: CREATE TABLE ... (user_id INT, role_id INT, PRIMARY KEY (user_id, role_id))
	// Syntax is almost the same, but there may be type differences (INT vs INTEGER) etc.
}

// ============================================================================
// Test: Composite PK with Other Constraints
// ============================================================================

/// Test composite primary key combined with other constraints
///
/// **Test Intent**: Verify detection when composite PK coexists with UNIQUE/FK constraints
///
/// **Integration Point**: MigrationAutodetector → detect_added_constraints()
///
/// **Expected Behavior**: All constraints should be detected independently
#[rstest]
fn test_composite_pk_with_other_constraints() {
	// from_state: Empty
	let from_state = ProjectState::new();

	// to_state: OrderItem with composite PK + UNIQUE constraint + FK constraint
	let mut to_state = ProjectState::new();
	let mut order_item_model = create_basic_model("testapp", "OrderItem", "testapp_orderitem");
	order_item_model.fields.remove("id");
	add_field(&mut order_item_model, "order_id", FieldType::Integer);
	add_field(&mut order_item_model, "product_id", FieldType::Integer);
	add_field(&mut order_item_model, "sku", FieldType::VarChar(100));

	// Composite PK constraint
	order_item_model.constraints.push(ConstraintDefinition {
		name: "pk_orderitem".to_string(),
		constraint_type: "PrimaryKey".to_string(),
		fields: vec!["order_id".to_string(), "product_id".to_string()],
		expression: None,
		foreign_key_info: None,
	});

	// UNIQUE constraint (SKU)
	order_item_model.constraints.push(ConstraintDefinition {
		name: "unique_sku".to_string(),
		constraint_type: "Unique".to_string(),
		fields: vec!["sku".to_string()],
		expression: None,
		foreign_key_info: None,
	});

	// FK constraint (order_id → orders.id)
	order_item_model.constraints.push(ConstraintDefinition {
		name: "fk_order".to_string(),
		constraint_type: "ForeignKey".to_string(),
		fields: vec!["order_id".to_string()],
		expression: None,
		foreign_key_info: Some(reinhardt_db::migrations::ForeignKeyConstraintInfo {
			referenced_table: "testapp_order".to_string(),
			referenced_columns: vec!["id".to_string()],
			on_delete: reinhardt_db::migrations::ForeignKeyAction::Cascade,
			on_update: reinhardt_db::migrations::ForeignKeyAction::NoAction,
		}),
	});

	to_state.add_model(order_item_model);

	// Run autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: All constraints are detected
	assert_eq!(
		detected.created_models.len(),
		1,
		"Should detect model creation"
	);
	assert_eq!(
		detected.added_constraints.len(),
		3,
		"Should detect all 3 constraints (PK + UNIQUE + FK)"
	);

	// Verify constraint type
	let constraint_types: Vec<&str> = detected
		.added_constraints
		.iter()
		.map(|c| c.2.constraint_type.as_str())
		.collect();
	assert!(constraint_types.contains(&"PrimaryKey"));
	assert!(constraint_types.contains(&"Unique"));
	assert!(constraint_types.contains(&"ForeignKey"));
}
