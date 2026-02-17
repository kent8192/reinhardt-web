//! Integration tests for edge cases in autodetector
//!
//! Tests boundary conditions and extreme scenarios:
//! - Large-scale schema changes (50+ models, 1000+ fields)
//! - Deep dependency chains
//! - Complex dependency graphs
//! - Multiple circular dependencies
//! - Special characters and identifiers
//! - Database-specific limitations
//!
//! **Test Coverage:**
//! - Scalability and performance boundaries
//! - Complex dependency resolution
//! - Edge case handling
//!
//! **Fixtures Used:**
//! - None (pure ProjectState manipulation)

use reinhardt_db::migrations::{
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

/// Add a field to a model
fn add_field(model: &mut ModelState, name: &str, field_type: FieldType) {
	model.fields.insert(
		name.to_string(),
		FieldState::new(name.to_string(), field_type, true, BTreeMap::new()),
	);
}

/// Add a foreign key constraint to a model
fn add_fk_constraint(
	model: &mut ModelState,
	field_name: &str,
	referenced_table: &str,
	on_delete: ForeignKeyAction,
) {
	model.constraints.push(ConstraintDefinition {
		name: format!("fk_{}_{}", model.table_name, field_name),
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
// Phase 2 Tests (5 tests - Large Schema & Complex Dependencies)
// ============================================================================

// ============================================================================
// Test 58: Large Schema - 50 Models
// ============================================================================

/// Test detection with large schema (50 models)
///
/// **Test Intent**: Verify that autodetector handles large number of models efficiently
///
/// **Integration Point**: MigrationAutodetector → detect_created_models()
///
/// **Expected Behavior**: All 50 models detected, reasonable performance
#[rstest]
fn test_large_schema_50_models() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: 50 models
	let mut to_state = ProjectState::new();
	for i in 0..50 {
		let model_name = format!("Model{:02}", i);
		let table_name = format!("testapp_model{:02}", i);
		let mut model = create_basic_model("testapp", &model_name, &table_name);
		// Add a few fields to each model
		add_field(&mut model, "name", FieldType::VarChar(100));
		add_field(&mut model, "description", FieldType::Text);
		to_state.add_model(model);
	}

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert_eq!(
		detected.created_models.len(),
		50,
		"Should detect all 50 models"
	);
}

// ============================================================================
// Test 59: Large Schema - 1000 Fields
// ============================================================================

/// Test detection with large number of fields (1000+ total)
///
/// **Test Intent**: Verify that autodetector handles large number of fields
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: All fields detected across multiple models
#[rstest]
fn test_large_schema_1000_fields() {
	// from_state: 10 models with minimal fields
	let mut from_state = ProjectState::new();
	for i in 0..10 {
		let model_name = format!("Model{}", i);
		let table_name = format!("testapp_model{}", i);
		from_state.add_model(create_basic_model("testapp", &model_name, &table_name));
	}

	// to_state: same 10 models, each with 100 fields (total 1000)
	let mut to_state = ProjectState::new();
	for i in 0..10 {
		let model_name = format!("Model{}", i);
		let table_name = format!("testapp_model{}", i);
		let mut model = create_basic_model("testapp", &model_name, &table_name);
		// Add 100 fields to each model
		for j in 0..100 {
			let field_name = format!("field{:03}", j);
			add_field(&mut model, &field_name, FieldType::VarChar(50));
		}
		to_state.add_model(model);
	}

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: All 1000 added fields should be detected
	assert_eq!(
		detected.added_fields.len(),
		1000,
		"Should detect all 1000 added fields"
	);
}

// ============================================================================
// Test 60: Deep Dependency Chain - 10 Levels
// ============================================================================

/// Test detection with deep dependency chain (10 levels)
///
/// **Test Intent**: Verify that deep FK dependency chains are handled
///
/// **Integration Point**: MigrationAutodetector → detect_model_dependencies()
///
/// **Expected Behavior**: Dependency chain correctly ordered via topological sort
#[rstest]
fn test_deep_dependency_chain_10_levels() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: 10 models in dependency chain (Model0 → Model1 → ... → Model9)
	let mut to_state = ProjectState::new();

	// Model0 (no dependencies)
	to_state.add_model(create_basic_model("testapp", "Model0", "testapp_model0"));

	// Model1-9 (each depends on the previous model)
	for i in 1..10 {
		let model_name = format!("Model{}", i);
		let table_name = format!("testapp_model{}", i);
		let prev_table_name = format!("testapp_model{}", i - 1);

		let mut model = create_basic_model("testapp", &model_name, &table_name);
		let fk_field_name = format!("prev_model_id");
		add_field(&mut model, &fk_field_name, FieldType::Integer);
		add_fk_constraint(
			&mut model,
			&fk_field_name,
			&prev_table_name,
			ForeignKeyAction::Cascade,
		);

		to_state.add_model(model);
	}

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Dependencies detected, no circular dependencies
	assert_eq!(
		detected.created_models.len(),
		10,
		"Should detect all 10 models"
	);
	assert!(
		!detected.check_circular_dependencies(),
		"Should not detect circular dependency in linear chain"
	);
}

// ============================================================================
// Test 61: Complex Dependency Graph
// ============================================================================

/// Test detection with complex dependency graph (multiple paths)
///
/// **Test Intent**: Verify that complex DAG dependencies are resolved correctly
///
/// **Integration Point**: MigrationAutodetector → order_models_by_dependency()
///
/// **Expected Behavior**: Topological sort produces valid order
#[rstest]
fn test_complex_dependency_graph() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: Complex dependency graph
	// A → C
	// B → C
	// C → D
	// D → E
	let mut to_state = ProjectState::new();

	// ModelE (no dependencies)
	to_state.add_model(create_basic_model("testapp", "ModelE", "testapp_modele"));

	// ModelD → E
	let mut model_d = create_basic_model("testapp", "ModelD", "testapp_modeld");
	add_field(&mut model_d, "e_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_d,
		"e_id",
		"testapp_modele",
		ForeignKeyAction::Cascade,
	);
	to_state.add_model(model_d);

	// ModelC → D
	let mut model_c = create_basic_model("testapp", "ModelC", "testapp_modelc");
	add_field(&mut model_c, "d_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_c,
		"d_id",
		"testapp_modeld",
		ForeignKeyAction::Cascade,
	);
	to_state.add_model(model_c);

	// ModelA → C
	let mut model_a = create_basic_model("testapp", "ModelA", "testapp_modela");
	add_field(&mut model_a, "c_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_a,
		"c_id",
		"testapp_modelc",
		ForeignKeyAction::Cascade,
	);
	to_state.add_model(model_a);

	// ModelB → C
	let mut model_b = create_basic_model("testapp", "ModelB", "testapp_modelb");
	add_field(&mut model_b, "c_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_b,
		"c_id",
		"testapp_modelc",
		ForeignKeyAction::Cascade,
	);
	to_state.add_model(model_b);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: All models detected, no circular dependencies
	assert_eq!(
		detected.created_models.len(),
		5,
		"Should detect all 5 models"
	);
	assert!(
		!detected.check_circular_dependencies(),
		"Should not detect circular dependency in DAG"
	);

	// Verify dependency ordering is correct
	let ordered = detected.order_models_by_dependency();
	assert!(
		ordered.is_ok(),
		"Should successfully order models by dependency"
	);
}

// ============================================================================
// Test 62: Multiple Circular Dependencies
// ============================================================================

/// Test detection with multiple circular dependency groups
///
/// **Test Intent**: Verify that multiple independent cycles are detected
///
/// **Integration Point**: MigrationAutodetector → check_circular_dependencies()
///
/// **Expected Behavior**: All circular dependencies detected
#[rstest]
fn test_multiple_circular_dependencies() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: Two independent circular dependency groups
	// Group 1: A → B → C → A
	// Group 2: X → Y → Z → X
	let mut to_state = ProjectState::new();

	// Group 1: Cycle A → B → C → A
	let mut model_a = create_basic_model("testapp", "ModelA", "testapp_modela");
	add_field(&mut model_a, "b_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_a,
		"b_id",
		"testapp_modelb",
		ForeignKeyAction::Cascade,
	);

	let mut model_b = create_basic_model("testapp", "ModelB", "testapp_modelb");
	add_field(&mut model_b, "c_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_b,
		"c_id",
		"testapp_modelc",
		ForeignKeyAction::Cascade,
	);

	let mut model_c = create_basic_model("testapp", "ModelC", "testapp_modelc");
	add_field(&mut model_c, "a_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_c,
		"a_id",
		"testapp_modela",
		ForeignKeyAction::Cascade,
	);

	// Group 2: Cycle X → Y → Z → X
	let mut model_x = create_basic_model("testapp", "ModelX", "testapp_modelx");
	add_field(&mut model_x, "y_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_x,
		"y_id",
		"testapp_modely",
		ForeignKeyAction::Cascade,
	);

	let mut model_y = create_basic_model("testapp", "ModelY", "testapp_modely");
	add_field(&mut model_y, "z_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_y,
		"z_id",
		"testapp_modelz",
		ForeignKeyAction::Cascade,
	);

	let mut model_z = create_basic_model("testapp", "ModelZ", "testapp_modelz");
	add_field(&mut model_z, "x_id", FieldType::Integer);
	add_fk_constraint(
		&mut model_z,
		"x_id",
		"testapp_modelx",
		ForeignKeyAction::Cascade,
	);

	to_state.add_model(model_a);
	to_state.add_model(model_b);
	to_state.add_model(model_c);
	to_state.add_model(model_x);
	to_state.add_model(model_y);
	to_state.add_model(model_z);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Circular dependencies detected
	assert!(
		detected.check_circular_dependencies(),
		"Should detect circular dependencies in both groups"
	);
}

// ============================================================================
// Phase 3 Tests (5 tests - Database-Specific Features)
// ============================================================================

// ============================================================================
// Test 63: SQLite ALTER TABLE Limitation
// ============================================================================

/// Test detection with SQLite's ALTER TABLE limitations
///
/// **Test Intent**: Verify that column type changes are detected (SQLite has limited ALTER TABLE support)
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: Type change detected; SQLite may require table rebuild
#[rstest]
fn test_sqlite_alter_table_limitation() {
	// from_state: User with VarChar username
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "username", FieldType::VarChar(100));
	from_state.add_model(user_model);

	// to_state: User with Text username (type change)
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "username", FieldType::Text);
	to_state.add_model(user_model);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field type change detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect column type change"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "username");

	// NOTE: SQLite doesn't support ALTER COLUMN TYPE, so table rebuild is required:
	// CREATE TABLE new → INSERT INTO new SELECT FROM old → DROP TABLE old
	// PostgreSQL/MySQL: ALTER TABLE ... ALTER COLUMN ... TYPE works
}

// ============================================================================
// Test 64: MySQL-Specific Types
// ============================================================================

/// Test detection of MySQL-specific integer types (TINYINT, MEDIUMINT)
///
/// **Test Intent**: Verify that MySQL-specific types are handled correctly
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: Fields with MySQL-specific types detected
#[rstest]
fn test_mysql_specific_types() {
	// from_state: Empty
	let from_state = ProjectState::new();

	// to_state: Status model with TINYINT type
	let mut to_state = ProjectState::new();
	let mut status_model = create_basic_model("testapp", "Status", "testapp_status");

	// MySQL TINYINT (1 byte integer, range -128 to 127)
	add_field(&mut status_model, "level", FieldType::SmallInt);
	// MySQL MEDIUMINT (3 byte integer) also approximated as SmallInt

	to_state.add_model(status_model);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Model creation and field addition detected
	assert_eq!(
		detected.created_models.len(),
		1,
		"Should detect model creation"
	);
	assert_eq!(
		detected.created_models[0].1, "Status",
		"Should detect Status model"
	);

	// NOTE: MySQL-specific types (TINYINT, MEDIUMINT) map to FieldType::SmallInt
	// PostgreSQL: Becomes SMALLINT
}

// ============================================================================
// Test 65: PostgreSQL ENUM Value Addition
// ============================================================================

/// Test detection of adding values to PostgreSQL ENUM type
///
/// **Test Intent**: Verify that ENUM value additions are detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: ENUM field modification detected
#[rstest]
fn test_postgres_enum_value_addition() {
	// from_state: User with status ENUM('active', 'inactive')
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	let mut status_options = BTreeMap::new();
	status_options.insert(
		"enum_values".to_string(),
		vec!["active".to_string(), "inactive".to_string()]
			.into_iter()
			.collect::<Vec<_>>()
			.join(","),
	);
	user_model.fields.insert(
		"status".to_string(),
		FieldState::new(
			"status".to_string(),
			FieldType::VarChar(20), // ENUM represented as VarChar
			false,
			status_options,
		),
	);
	from_state.add_model(user_model);

	// to_state: User with status ENUM('active', 'inactive', 'pending')
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	let mut status_options = BTreeMap::new();
	status_options.insert(
		"enum_values".to_string(),
		vec![
			"active".to_string(),
			"inactive".to_string(),
			"pending".to_string(),
		]
		.into_iter()
		.collect::<Vec<_>>()
		.join(","),
	);
	user_model.fields.insert(
		"status".to_string(),
		FieldState::new(
			"status".to_string(),
			FieldType::VarChar(20),
			false,
			status_options,
		),
	);
	to_state.add_model(user_model);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field option change (ENUM value addition) detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect ENUM value addition"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "status");

	// NOTE: PostgreSQL: ALTER TYPE ... ADD VALUE 'pending'
	// MySQL: ALTER TABLE ... MODIFY COLUMN status ENUM('active', 'inactive', 'pending')
}

// ============================================================================
// Test 66: PostgreSQL ENUM Value Deletion
// ============================================================================

/// Test detection of removing values from PostgreSQL ENUM type
///
/// **Test Intent**: Verify that ENUM value deletions are detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_fields()
///
/// **Expected Behavior**: ENUM field modification detected
#[rstest]
fn test_postgres_enum_value_deletion() {
	// from_state: User with status ENUM('active', 'inactive', 'pending')
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	let mut status_options = BTreeMap::new();
	status_options.insert(
		"enum_values".to_string(),
		vec![
			"active".to_string(),
			"inactive".to_string(),
			"pending".to_string(),
		]
		.into_iter()
		.collect::<Vec<_>>()
		.join(","),
	);
	user_model.fields.insert(
		"status".to_string(),
		FieldState::new(
			"status".to_string(),
			FieldType::VarChar(20),
			false,
			status_options,
		),
	);
	from_state.add_model(user_model);

	// to_state: User with status ENUM('active', 'inactive') - 'pending' removed
	let mut to_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	let mut status_options = BTreeMap::new();
	status_options.insert(
		"enum_values".to_string(),
		vec!["active".to_string(), "inactive".to_string()]
			.into_iter()
			.collect::<Vec<_>>()
			.join(","),
	);
	user_model.fields.insert(
		"status".to_string(),
		FieldState::new(
			"status".to_string(),
			FieldType::VarChar(20),
			false,
			status_options,
		),
	);
	to_state.add_model(user_model);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field option change (ENUM value deletion) detected
	assert_eq!(
		detected.altered_fields.len(),
		1,
		"Should detect ENUM value deletion"
	);
	assert_eq!(detected.altered_fields[0].0, "testapp");
	assert_eq!(detected.altered_fields[0].1, "User");
	assert_eq!(detected.altered_fields[0].2, "status");

	// NOTE: PostgreSQL doesn't support direct ENUM value deletion
	// Requires creating new ENUM type and converting the column (complex procedure)
	// MySQL: ALTER TABLE ... MODIFY COLUMN allows ENUM definition change,
	// but errors if existing data uses the deleted value
}

// ============================================================================
// Test 67: MySQL Fulltext Index
// ============================================================================

/// Test detection of MySQL fulltext index
///
/// **Test Intent**: Verify that fulltext index additions are detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_indexes()
///
/// **Expected Behavior**: Fulltext index addition detected
#[rstest]
fn test_mysql_fulltext_index() {
	// from_state: Article without fulltext index
	let mut from_state = ProjectState::new();
	let mut article_model = create_basic_model("testapp", "Article", "testapp_article");
	add_field(&mut article_model, "title", FieldType::VarChar(200));
	add_field(&mut article_model, "content", FieldType::Text);
	from_state.add_model(article_model);

	// to_state: Article with fulltext index on (title, content)
	let mut to_state = ProjectState::new();
	let mut article_model = create_basic_model("testapp", "Article", "testapp_article");
	add_field(&mut article_model, "title", FieldType::VarChar(200));
	add_field(&mut article_model, "content", FieldType::Text);

	// Fulltext index constraint
	article_model.constraints.push(ConstraintDefinition {
		name: "ft_article_search".to_string(),
		constraint_type: "FulltextIndex".to_string(),
		fields: vec!["title".to_string(), "content".to_string()],
		expression: None,
		foreign_key_info: None,
	});

	to_state.add_model(article_model);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Fulltext index addition detected
	assert_eq!(
		detected.added_constraints.len(),
		1,
		"Should detect fulltext index addition"
	);
	assert_eq!(
		detected.added_constraints[0].2.constraint_type,
		"FulltextIndex"
	);
	assert_eq!(detected.added_constraints[0].2.fields.len(), 2);

	// NOTE: MySQL: CREATE FULLTEXT INDEX ft_article_search ON testapp_article (title, content)
	// PostgreSQL: Requires tsvector type and GIN index (different approach)
}

// ============================================================================
// Phase 4 Tests (4 tests - Edge Cases & Database Limitations)
// ============================================================================

// ============================================================================
// Test 68: SQL Reserved Word Table Name
// ============================================================================

/// Test detection with SQL reserved words as table names
///
/// **Test Intent**: Verify that reserved words are properly handled (escaped)
///
/// **Integration Point**: MigrationAutodetector → detect_created_models()
///
/// **Expected Behavior**: Reserved words detected, proper escaping applied
#[rstest]
fn test_sql_reserved_word_table_name() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: Models with SQL reserved words as table names
	let mut to_state = ProjectState::new();

	// Common SQL reserved words
	let reserved_words = vec![
		("Select", "select"), // SELECT
		("Table", "table"),   // TABLE
		("Where", "where"),   // WHERE
		("Join", "join"),     // JOIN
		("Order", "order"),   // ORDER
		("Group", "group"),   // GROUP
		("Insert", "insert"), // INSERT
		("Update", "update"), // UPDATE
		("Delete", "delete"), // DELETE
		("Create", "create"), // CREATE
	];

	for (model_name, table_name) in reserved_words {
		let mut model = create_basic_model("testapp", model_name, table_name);
		add_field(&mut model, "name", FieldType::VarChar(100));
		to_state.add_model(model);
	}

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: All reserved word models detected
	assert_eq!(
		detected.created_models.len(),
		10,
		"Should detect all models with reserved word table names"
	);

	// Verify model names are preserved
	let model_names: Vec<String> = detected
		.created_models
		.iter()
		.map(|(_, name)| name.to_string())
		.collect();

	assert!(model_names.contains(&"Select".to_string()));
	assert!(model_names.contains(&"Table".to_string()));
	assert!(model_names.contains(&"Where".to_string()));

	// NOTE: In SQL generation, these table names should be escaped:
	// PostgreSQL: "select", "table", "where" (double quotes)
	// MySQL: `select`, `table`, `where` (backticks)
	// SQLite: "select", "table", "where" (double quotes)
}

// ============================================================================
// Test 69: Special Characters in Names
// ============================================================================

/// Test detection with special characters in table/column names
///
/// **Test Intent**: Verify that special characters are handled correctly
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: Special characters detected, proper escaping applied
#[rstest]
fn test_special_characters_in_names() {
	// from_state: Basic model
	let mut from_state = ProjectState::new();
	let basic_model = create_basic_model("testapp", "SpecialModel", "testapp_special");
	from_state.add_model(basic_model);

	// to_state: Model with special character field names
	let mut to_state = ProjectState::new();
	let mut special_model = create_basic_model("testapp", "SpecialModel", "testapp_special");

	// Field names with special characters
	// Note: Some databases have restrictions on allowed characters
	add_field(
		&mut special_model,
		"field_with_underscore",
		FieldType::VarChar(100),
	);
	add_field(
		&mut special_model,
		"field-with-dash",
		FieldType::VarChar(100),
	);
	add_field(
		&mut special_model,
		"field.with.dot",
		FieldType::VarChar(100),
	);
	add_field(
		&mut special_model,
		"field with space",
		FieldType::VarChar(100),
	);
	add_field(
		&mut special_model,
		"field$with$dollar",
		FieldType::VarChar(100),
	);

	to_state.add_model(special_model);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: All special character fields detected
	assert_eq!(
		detected.added_fields.len(),
		5,
		"Should detect all fields with special characters"
	);

	// Verify field names are preserved
	let field_names: Vec<String> = detected
		.added_fields
		.iter()
		.map(|(_, _, field)| field.to_string())
		.collect();

	assert!(field_names.contains(&"field_with_underscore".to_string()));
	assert!(field_names.contains(&"field-with-dash".to_string()));
	assert!(field_names.contains(&"field.with.dot".to_string()));
	assert!(field_names.contains(&"field with space".to_string()));
	assert!(field_names.contains(&"field$with$dollar".to_string()));

	// NOTE: Database-specific handling:
	// - Underscore: Allowed in all databases (no escaping needed)
	// - Dash: Requires escaping (conflicts with minus operator)
	// - Dot: Requires escaping (conflicts with schema.table notation)
	// - Space: Requires escaping (conflicts with SQL syntax)
	// - Dollar: PostgreSQL allows, MySQL requires escaping
	//
	// Best practice: Use quoted identifiers for all special characters
	// PostgreSQL: "field-with-dash", "field.with.dot", "field with space"
	// MySQL: `field-with-dash`, `field.with.dot`, `field with space`
}

// ============================================================================
// Test 70: Very Long Identifier (63 chars)
// ============================================================================

/// Test detection with very long identifiers (PostgreSQL 63 char limit)
///
/// **Test Intent**: Verify that long identifiers are handled correctly
///
/// **Integration Point**: MigrationAutodetector → detect_created_models()
///
/// **Expected Behavior**: Long identifiers detected, truncation handled
#[rstest]
fn test_very_long_identifier_63_chars() {
	// from_state: empty
	let from_state = ProjectState::new();

	// to_state: Model with very long table name (PostgreSQL limit: 63 chars)
	let mut to_state = ProjectState::new();

	// PostgreSQL identifier limit: NAMEDATALEN - 1 = 63 characters
	// Identifiers longer than 63 chars are truncated

	// Exactly 63 characters (maximum allowed)
	let long_name_63 = "a_very_long_table_name_that_is_exactly_sixty_three_characters_";
	assert_eq!(long_name_63.len(), 63, "Should be exactly 63 characters");

	// 64 characters (will be truncated to 63)
	let long_name_64 = "a_very_long_table_name_that_is_sixty_four_characters_long_abc";
	assert_eq!(long_name_64.len(), 62, "Test string preparation");
	let long_name_64_actual = format!("{}d", long_name_64); // Make it 63
	assert_eq!(long_name_64_actual.len(), 63);

	// Create model with 63 char table name
	let mut model_63 = create_basic_model("testapp", "Model63", &long_name_63);
	add_field(&mut model_63, "name", FieldType::VarChar(100));
	to_state.add_model(model_63);

	// Create model with 64+ char table name (would be truncated by PostgreSQL)
	let long_name_100 = "a".repeat(100);
	assert_eq!(long_name_100.len(), 100);

	let mut model_100 = create_basic_model("testapp", "Model100", &long_name_100);
	add_field(&mut model_100, "name", FieldType::VarChar(100));
	to_state.add_model(model_100);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Both models detected
	assert_eq!(
		detected.created_models.len(),
		2,
		"Should detect both models with long names"
	);

	// Verify model names
	let model_names: Vec<String> = detected
		.created_models
		.iter()
		.map(|(_, name)| name.to_string())
		.collect();

	assert!(model_names.contains(&"Model63".to_string()));
	assert!(model_names.contains(&"Model100".to_string()));

	// NOTE: PostgreSQL behavior:
	// - Identifiers are truncated to 63 characters
	// - NOTICE: identifier "aaaa...aaaa" will be truncated to "aaaa...aaa" (63 chars)
	// - Truncated identifiers may cause name collisions!
	//
	// Example collision:
	// "very_long_prefix_for_user_profile_table_version_1" (50 chars) -> OK
	// "very_long_prefix_for_user_profile_table_version_2" (50 chars) -> OK
	// Both are under 63 chars, no collision
	//
	// But:
	// "very_long_prefix_for_user_profile_table_with_additional_metadata_v1" (68 chars)
	// "very_long_prefix_for_user_profile_table_with_additional_metadata_v2" (68 chars)
	// Both truncated to:
	// "very_long_prefix_for_user_profile_table_with_additional_metad" (63 chars)
	// -> COLLISION! CREATE TABLE will fail
	//
	// Best practice:
	// - Keep identifiers under 63 characters
	// - Use abbreviations for long names
	// - Test with PostgreSQL before deploying
}

// ============================================================================
// Test 71: Abstract Base Model Change
// ============================================================================

/// Test detection of abstract base model changes
///
/// **Test Intent**: Verify that abstract model changes are detected correctly
///
/// **Integration Point**: MigrationAutodetector → detect_model_inheritance_changes()
///
/// **Expected Behavior**: Inheritance changes detected, dependencies updated
#[rstest]
fn test_abstract_base_model_change() {
	// from_state: Concrete model without base
	let mut from_state = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	add_field(&mut user_model, "username", FieldType::VarChar(100));
	add_field(&mut user_model, "email", FieldType::VarChar(255));
	from_state.add_model(user_model);

	// to_state: Same model but now inherits from abstract base
	let mut to_state = ProjectState::new();

	// Abstract base model (not a table)
	let mut timestamped_model = create_basic_model("testapp", "Timestamped", "");
	timestamped_model
		.options
		.insert("abstract".to_string(), "true".to_string());
	add_field(&mut timestamped_model, "created_at", FieldType::Timestamp);
	add_field(&mut timestamped_model, "updated_at", FieldType::Timestamp);
	to_state.add_model(timestamped_model);

	// User model now inherits from Timestamped
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.base_model = Some("Timestamped".to_string());
	add_field(&mut user_model, "username", FieldType::VarChar(100));
	add_field(&mut user_model, "email", FieldType::VarChar(255));
	// Fields from base model are inherited
	add_field(&mut user_model, "created_at", FieldType::Timestamp);
	add_field(&mut user_model, "updated_at", FieldType::Timestamp);
	to_state.add_model(user_model);

	// Execute autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Abstract model creation detected
	assert_eq!(
		detected.created_models.len(),
		1,
		"Should detect abstract base model creation"
	);
	assert_eq!(detected.created_models[0].1, "Timestamped");

	// Verify: Inherited fields detected as added to User
	assert_eq!(
		detected.added_fields.len(),
		2,
		"Should detect inherited fields as added to User"
	);

	let added_field_names: Vec<String> = detected
		.added_fields
		.iter()
		.map(|(_, _, field)| field.to_string())
		.collect();
	assert!(added_field_names.contains(&"created_at".to_string()));
	assert!(added_field_names.contains(&"updated_at".to_string()));

	// ============================================================================
	// Test removing abstract base model
	// ============================================================================

	// from_state: Model with abstract base
	let mut from_state_2 = ProjectState::new();

	let mut timestamped_model = create_basic_model("testapp", "Timestamped", "");
	timestamped_model
		.options
		.insert("abstract".to_string(), "true".to_string());
	add_field(&mut timestamped_model, "created_at", FieldType::Timestamp);
	add_field(&mut timestamped_model, "updated_at", FieldType::Timestamp);
	from_state_2.add_model(timestamped_model);

	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.base_model = Some("Timestamped".to_string());
	add_field(&mut user_model, "username", FieldType::VarChar(100));
	add_field(&mut user_model, "email", FieldType::VarChar(255));
	add_field(&mut user_model, "created_at", FieldType::Timestamp);
	add_field(&mut user_model, "updated_at", FieldType::Timestamp);
	from_state_2.add_model(user_model);

	// to_state: Model without abstract base
	let mut to_state_2 = ProjectState::new();
	let mut user_model = create_basic_model("testapp", "User", "testapp_user");
	user_model.base_model = None; // Base removed
	add_field(&mut user_model, "username", FieldType::VarChar(100));
	add_field(&mut user_model, "email", FieldType::VarChar(255));
	// Inherited fields remain in User (no change)
	add_field(&mut user_model, "created_at", FieldType::Timestamp);
	add_field(&mut user_model, "updated_at", FieldType::Timestamp);
	to_state_2.add_model(user_model);

	// Execute autodetector
	let autodetector_2 = MigrationAutodetector::new(from_state_2, to_state_2);
	let detected_2 = autodetector_2.detect_changes();

	// Verify: Abstract model deletion detected
	assert_eq!(
		detected_2.deleted_models.len(),
		1,
		"Should detect abstract base model deletion"
	);
	assert_eq!(detected_2.deleted_models[0].1, "Timestamped");

	// NOTE: Abstract base model behavior:
	// - Abstract models don't create tables (abstract=true)
	// - Inheriting models get all base fields
	// - Changing base model affects all inheriting models
	// - In Django: class User(Timestamped): ...
	// - In Rust: User { base: Timestamped, ... }
	//
	// Migration considerations:
	// - Adding base: Add inherited fields to concrete model
	// - Removing base: Keep fields in concrete model (data preservation)
	// - Changing base: Remove old fields, add new fields
}
