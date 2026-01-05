//! Integration tests for relationship detection in autodetector
//!
//! Tests relationship-related change detection:
//! - Foreign Key relationships
//! - One-to-One relationships
//! - Many-to-Many relationships (auto and custom through tables)
//! - Self-referencing relationships
//! - Inheritance patterns
//!
//! **Test Coverage:**
//! - FK addition and ON DELETE/UPDATE changes
//! - M2M intermediate table generation
//! - Self-referential FK handling
//! - Inheritance type detection
//!
//! **Fixtures Used:**
//! - None (pure ProjectState manipulation)

use reinhardt_migrations::{
	ConstraintDefinition, FieldState, FieldType, ForeignKeyAction, ForeignKeyConstraintInfo,
	ManyToManyMetadata, MigrationAutodetector, ModelState, ProjectState,
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

/// Add a foreign key constraint to a model
fn add_fk_constraint(
	model: &mut ModelState,
	field_name: &str,
	referenced_table: &str,
	referenced_column: &str,
	on_delete: ForeignKeyAction,
	on_update: ForeignKeyAction,
) {
	model.constraints.push(ConstraintDefinition {
		name: format!("fk_{}_{}", model.table_name, field_name),
		constraint_type: "ForeignKey".to_string(),
		fields: vec![field_name.to_string()],
		expression: None,
		foreign_key_info: Some(ForeignKeyConstraintInfo {
			referenced_table: referenced_table.to_string(),
			referenced_columns: vec![referenced_column.to_string()],
			on_delete,
			on_update,
		}),
	});
}

// ============================================================================
// Test 16: Add Foreign Key Detection
// ============================================================================

/// Test detection of foreign key addition
///
/// **Test Intent**: Verify that adding a FK field is detected correctly
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields() + detect_added_constraints()
///
/// **Expected Behavior**: FK field and constraint addition detected
#[rstest]
#[test]
fn test_detect_add_foreign_key() {
	// from_state: Post model without author FK
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	from_state.add_model(create_basic_model("testapp", "Post", "testapp_post"));

	// to_state: Post model with author FK to User
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	let mut post_model = create_basic_model("testapp", "Post", "testapp_post");
	add_field(&mut post_model, "author_id", FieldType::Integer, true);
	add_fk_constraint(
		&mut post_model,
		"author_id",
		"testapp_user",
		"id",
		ForeignKeyAction::Cascade,
		ForeignKeyAction::NoAction,
	);
	to_state.add_model(post_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert!(
		detected.added_fields.len() > 0 || detected.added_constraints.len() > 0,
		"Should detect FK field or constraint addition"
	);
}

// ============================================================================
// Test 17: Change ON DELETE Action Detection
// ============================================================================

/// Test detection of ON DELETE action change
///
/// **Test Intent**: Verify that changing ON DELETE behavior is detected
///
/// **Integration Point**: MigrationAutodetector → detect_altered_constraints()
///
/// **Expected Behavior**: Constraint alteration detected
#[rstest]
#[test]
fn test_detect_change_on_delete_action() {
	// from_state: Post with FK (ON DELETE CASCADE)
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	let mut post_model = create_basic_model("testapp", "Post", "testapp_post");
	add_field(&mut post_model, "author_id", FieldType::Integer, true);
	add_fk_constraint(
		&mut post_model,
		"author_id",
		"testapp_user",
		"id",
		ForeignKeyAction::Cascade,
		ForeignKeyAction::NoAction,
	);
	from_state.add_model(post_model);

	// to_state: Post with FK (ON DELETE SET NULL)
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	let mut post_model = create_basic_model("testapp", "Post", "testapp_post");
	add_field(&mut post_model, "author_id", FieldType::Integer, true);
	add_fk_constraint(
		&mut post_model,
		"author_id",
		"testapp_user",
		"id",
		ForeignKeyAction::SetNull,
		ForeignKeyAction::NoAction,
	);
	to_state.add_model(post_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Constraint may be detected as remove→re-create
	assert!(
		detected.removed_constraints.len() > 0 || detected.added_constraints.len() > 0,
		"Should detect constraint change (remove old + add new)"
	);
}

// ============================================================================
// Test 18: Add One-to-One Field Detection
// ============================================================================

/// Test detection of one-to-one relationship addition
///
/// **Test Intent**: Verify that adding a 1:1 relationship is detected
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields() + detect_added_constraints()
///
/// **Expected Behavior**: FK field with unique constraint detected
#[rstest]
#[test]
fn test_detect_add_one_to_one_field() {
	// from_state: User without profile link
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	from_state.add_model(create_basic_model("testapp", "Profile", "testapp_profile"));

	// to_state: Profile with 1:1 FK to User (unique)
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "User", "testapp_user"));
	let mut profile_model = create_basic_model("testapp", "Profile", "testapp_profile");
	add_field(&mut profile_model, "user_id", FieldType::Integer, false);
	add_fk_constraint(
		&mut profile_model,
		"user_id",
		"testapp_user",
		"id",
		ForeignKeyAction::Cascade,
		ForeignKeyAction::NoAction,
	);
	// UNIQUE constraint (to ensure 1:1)
	profile_model.constraints.push(ConstraintDefinition {
		name: "unique_user_id".to_string(),
		constraint_type: "Unique".to_string(),
		fields: vec!["user_id".to_string()],
		expression: None,
		foreign_key_info: None,
	});
	to_state.add_model(profile_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Field and UNIQUE constraint are detected
	assert!(
		detected.added_fields.len() > 0,
		"Should detect user_id field"
	);
	assert!(
		detected.added_constraints.len() > 0,
		"Should detect UNIQUE constraint for 1:1"
	);
}

// ============================================================================
// Test 19: Many-to-Many Auto Through Table Detection
// ============================================================================

/// Test detection of M2M relationship with auto-generated through table
///
/// **Test Intent**: Verify that M2M field addition generates intermediate table
///
/// **Integration Point**: MigrationAutodetector → detect_created_many_to_many()
///
/// **Expected Behavior**: Intermediate table creation detected
#[rstest]
#[test]
fn test_detect_many_to_many_auto_through() {
	// from_state: Book and Author without M2M
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "Book", "testapp_book"));
	from_state.add_model(create_basic_model("testapp", "Author", "testapp_author"));

	// to_state: Book with M2M to Author (auto through table)
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "Author", "testapp_author"));
	let mut book_model = create_basic_model("testapp", "Book", "testapp_book");
	book_model.many_to_many_fields.push(ManyToManyMetadata {
		field_name: "authors".to_string(),
		target_model: "Author".to_string(),
		through_model: None, // Auto-generated
		source_field_name: None,
		target_field_name: None,
	});
	to_state.add_model(book_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: M2M intermediate table creation detected
	assert!(
		detected.created_many_to_many.len() > 0 || detected.created_models.len() > 0,
		"Should detect M2M intermediate table creation"
	);
}

// ============================================================================
// Test 20: Many-to-Many Custom Through Table Detection
// ============================================================================

/// Test detection of M2M relationship with custom through table
///
/// **Test Intent**: Verify that M2M with custom through model is detected
///
/// **Integration Point**: MigrationAutodetector → detect_created_many_to_many()
///
/// **Expected Behavior**: Custom through model detected with extra fields
#[rstest]
#[test]
fn test_detect_many_to_many_custom_through() {
	// from_state: Book and Author without M2M
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model("testapp", "Book", "testapp_book"));
	from_state.add_model(create_basic_model("testapp", "Author", "testapp_author"));

	// to_state: Book with M2M to Author through custom 'BookAuthor' model
	let mut to_state = ProjectState::new();
	to_state.add_model(create_basic_model("testapp", "Author", "testapp_author"));

	// Custom intermediate model
	let mut book_author_model = create_basic_model("testapp", "BookAuthor", "testapp_bookauthor");
	add_field(&mut book_author_model, "book_id", FieldType::Integer, false);
	add_field(
		&mut book_author_model,
		"author_id",
		FieldType::Integer,
		false,
	);
	add_field(&mut book_author_model, "role", FieldType::VarChar(50), true); // Additional field
	to_state.add_model(book_author_model);

	let mut book_model = create_basic_model("testapp", "Book", "testapp_book");
	book_model.many_to_many_fields.push(ManyToManyMetadata {
		field_name: "authors".to_string(),
		target_model: "Author".to_string(),
		through_model: Some("BookAuthor".to_string()),
		source_field_name: Some("book_id".to_string()),
		target_field_name: Some("author_id".to_string()),
	});
	to_state.add_model(book_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Custom intermediate model creation detected
	assert!(
		detected.created_models.len() > 0,
		"Should detect custom through model creation"
	);
}

// ============================================================================
// Test 21: Self-Referencing FK Detection
// ============================================================================

/// Test detection of self-referencing foreign key
///
/// **Test Intent**: Verify that self-referential FK is detected correctly
///
/// **Integration Point**: MigrationAutodetector → detect_added_fields()
///
/// **Expected Behavior**: Self-referential FK field detected
#[rstest]
#[test]
fn test_detect_self_referencing_fk() {
	// from_state: Category without parent link
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model(
		"testapp",
		"Category",
		"testapp_category",
	));

	// to_state: Category with parent_id (self-referencing FK)
	let mut to_state = ProjectState::new();
	let mut category_model = create_basic_model("testapp", "Category", "testapp_category");
	add_field(&mut category_model, "parent_id", FieldType::Integer, true);
	add_fk_constraint(
		&mut category_model,
		"parent_id",
		"testapp_category", // Self-reference
		"id",
		ForeignKeyAction::SetNull,
		ForeignKeyAction::NoAction,
	);
	to_state.add_model(category_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify
	assert!(
		detected.added_fields.len() > 0,
		"Should detect self-referencing FK field"
	);
}

// ============================================================================
// Test 22: Single Table Inheritance Detection
// ============================================================================

/// Test detection of single table inheritance pattern
///
/// **Test Intent**: Verify that STI pattern is detected
///
/// **Integration Point**: MigrationAutodetector → inheritance_type field
///
/// **Expected Behavior**: Base model with discriminator column detected
#[rstest]
#[test]
fn test_detect_single_table_inheritance() {
	// from_state: Simple Employee model
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model(
		"testapp",
		"Employee",
		"testapp_employee",
	));

	// to_state: Employee with STI (discriminator column)
	let mut to_state = ProjectState::new();
	let mut employee_model = create_basic_model("testapp", "Employee", "testapp_employee");
	employee_model.inheritance_type = Some("SingleTable".to_string());
	employee_model.discriminator_column = Some("employee_type".to_string());
	add_field(
		&mut employee_model,
		"employee_type",
		FieldType::VarChar(50),
		false,
	);
	to_state.add_model(employee_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: Discriminator column addition detected
	assert!(
		detected.added_fields.len() > 0,
		"Should detect discriminator column addition"
	);
}

// ============================================================================
// Test 23: Joined Table Inheritance Detection
// ============================================================================

/// Test detection of joined table inheritance pattern
///
/// **Test Intent**: Verify that JTI pattern is detected
///
/// **Integration Point**: MigrationAutodetector → inheritance_type field
///
/// **Expected Behavior**: Child table with FK to parent detected
#[rstest]
#[test]
fn test_detect_joined_table_inheritance() {
	// from_state: Employee model only
	let mut from_state = ProjectState::new();
	from_state.add_model(create_basic_model(
		"testapp",
		"Employee",
		"testapp_employee",
	));

	// to_state: Employee + Manager (JTI)
	let mut to_state = ProjectState::new();
	let mut employee_model = create_basic_model("testapp", "Employee", "testapp_employee");
	employee_model.inheritance_type = Some("JoinedTable".to_string());
	to_state.add_model(employee_model);

	// Manager inheritance table
	let mut manager_model = create_basic_model("testapp", "Manager", "testapp_manager");
	manager_model.base_model = Some("Employee".to_string());
	manager_model.inheritance_type = Some("JoinedTable".to_string());
	add_field(
		&mut manager_model,
		"employee_ptr_id",
		FieldType::Integer,
		false,
	);
	add_fk_constraint(
		&mut manager_model,
		"employee_ptr_id",
		"testapp_employee",
		"id",
		ForeignKeyAction::Cascade,
		ForeignKeyAction::NoAction,
	);
	add_field(
		&mut manager_model,
		"department",
		FieldType::VarChar(100),
		true,
	);
	to_state.add_model(manager_model);

	// Execute Autodetector
	let autodetector = MigrationAutodetector::new(from_state, to_state);
	let detected = autodetector.detect_changes();

	// Verify: New Manager model creation detected
	assert!(
		detected.created_models.len() > 0,
		"Should detect child table creation in JTI"
	);
}
