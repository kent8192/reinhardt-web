//! SQLite Table Recreation Tests
//!
//! Tests for SQLite-specific table recreation functionality.
//! SQLite has limited ALTER TABLE support, so operations like
//! DROP COLUMN, ALTER COLUMN, and constraint modifications require
//! a 4-step table recreation process.

use reinhardt_db::migrations::{
	FieldType, ProjectState,
	operations::{ColumnDefinition, Constraint, Operation, SqlDialect, SqliteTableRecreation},
};
use rstest::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Fixture for basic column definitions
#[fixture]
fn basic_columns() -> Vec<ColumnDefinition> {
	vec![
		ColumnDefinition {
			name: "id".to_string(),
			type_definition: FieldType::Integer,
			not_null: true,
			primary_key: true,
			unique: false,
			auto_increment: true,
			default: None,
		},
		ColumnDefinition {
			name: "name".to_string(),
			type_definition: FieldType::Text,
			not_null: true,
			primary_key: false,
			unique: false,
			auto_increment: false,
			default: None,
		},
		ColumnDefinition {
			name: "email".to_string(),
			type_definition: FieldType::Text,
			not_null: false,
			primary_key: false,
			unique: true,
			auto_increment: false,
			default: None,
		},
	]
}

/// Fixture for constraints
#[fixture]
fn basic_constraints() -> Vec<Constraint> {
	vec![Constraint::Unique {
		name: "uq_email".to_string(),
		columns: vec!["email".to_string()],
	}]
}

// ============================================================================
// SqliteTableRecreation Tests
// ============================================================================

/// Test: for_drop_column generates correct recreation SQL
///
/// Category: Unit Test
/// Verifies that dropping a column generates the correct 4-step SQL.
#[rstest]
fn test_sqlite_recreation_drop_column(basic_columns: Vec<ColumnDefinition>) {
	let constraints: Vec<Constraint> = vec![];
	let recreation =
		SqliteTableRecreation::for_drop_column("users", basic_columns, "email", constraints);

	let statements = recreation.to_sql_statements();

	// Should generate 4 statements: CREATE, INSERT, DROP, RENAME
	assert_eq!(statements.len(), 4, "Should generate 4 SQL statements");

	// Verify CREATE statement
	assert!(
		statements[0].contains("CREATE TABLE"),
		"First statement should be CREATE TABLE"
	);
	assert!(
		statements[0].contains("_new"),
		"Should create temporary table with _new suffix"
	);
	assert!(
		!statements[0].contains("email"),
		"New table should not contain dropped column 'email'"
	);
	assert!(
		statements[0].contains("id"),
		"New table should contain 'id' column"
	);
	assert!(
		statements[0].contains("name"),
		"New table should contain 'name' column"
	);

	// Verify INSERT statement
	assert!(
		statements[1].contains("INSERT INTO"),
		"Second statement should be INSERT"
	);
	assert!(
		statements[1].contains("SELECT"),
		"Should copy data with SELECT"
	);

	// Verify DROP statement
	assert!(
		statements[2].contains("DROP TABLE"),
		"Third statement should be DROP TABLE"
	);

	// Verify RENAME statement
	assert!(
		statements[3].contains("ALTER TABLE"),
		"Fourth statement should be ALTER TABLE RENAME"
	);
	assert!(
		statements[3].contains("RENAME TO"),
		"Should rename temporary table"
	);
}

/// Test: for_alter_column generates correct recreation SQL
///
/// Category: Unit Test
/// Verifies that altering a column generates the correct 4-step SQL.
#[rstest]
fn test_sqlite_recreation_alter_column(basic_columns: Vec<ColumnDefinition>) {
	let constraints: Vec<Constraint> = vec![];
	let new_definition = ColumnDefinition {
		name: "name".to_string(),
		type_definition: FieldType::Text,
		not_null: false, // Changed from true to false
		primary_key: false,
		unique: false,
		auto_increment: false,
		default: Some("'Unknown'".to_string()),
	};

	let recreation = SqliteTableRecreation::for_alter_column(
		"users",
		basic_columns,
		"name",
		new_definition,
		constraints,
	);

	let statements = recreation.to_sql_statements();

	assert_eq!(statements.len(), 4, "Should generate 4 SQL statements");

	// Verify the new column definition is used
	assert!(
		!statements[0].contains("NOT NULL") || statements[0].contains("DEFAULT"),
		"Altered column should have new definition"
	);
}

/// Test: for_add_constraint generates correct recreation SQL
///
/// Category: Unit Test
/// Verifies that adding a constraint generates the correct 4-step SQL.
#[rstest]
fn test_sqlite_recreation_add_constraint(
	basic_columns: Vec<ColumnDefinition>,
	basic_constraints: Vec<Constraint>,
) {
	let new_constraint = "CONSTRAINT chk_name CHECK (length(name) > 0)".to_string();

	let recreation = SqliteTableRecreation::for_add_constraint(
		"users",
		basic_columns,
		basic_constraints,
		new_constraint.clone(),
	);

	let statements = recreation.to_sql_statements();

	assert_eq!(statements.len(), 4, "Should generate 4 SQL statements");

	// Verify the new constraint is included
	assert!(
		statements[0].contains("CHECK"),
		"New table should include the new CHECK constraint"
	);
}

/// Test: for_drop_constraint generates correct recreation SQL
///
/// Category: Unit Test
/// Verifies that dropping a constraint generates the correct 4-step SQL.
#[rstest]
fn test_sqlite_recreation_drop_constraint(basic_columns: Vec<ColumnDefinition>) {
	let constraints = vec![
		Constraint::Unique {
			name: "uq_email".to_string(),
			columns: vec!["email".to_string()],
		},
		Constraint::Unique {
			name: "uq_name".to_string(),
			columns: vec!["name".to_string()],
		},
	];

	let recreation =
		SqliteTableRecreation::for_drop_constraint("users", basic_columns, constraints, "uq_email");

	let statements = recreation.to_sql_statements();

	assert_eq!(statements.len(), 4, "Should generate 4 SQL statements");

	// The dropped constraint should not be in the new table
	// Note: The remaining constraint (uq_name) should still be there
}

// ============================================================================
// requires_sqlite_recreation Tests
// ============================================================================

/// Test: requires_sqlite_recreation returns true for incompatible operations
///
/// Category: Unit Test
/// Verifies that operations requiring recreation are correctly identified.
#[rstest]
#[case(Operation::DropColumn { table: "t".to_string(), column: "c".to_string() }, true)]
#[case(Operation::AddConstraint { table: "t".to_string(), constraint_sql: "...".to_string() }, true)]
#[case(Operation::DropConstraint { table: "t".to_string(), constraint_name: "c".to_string() }, true)]
#[case(Operation::CreateTable { name: "t".to_string(), columns: vec![], constraints: vec![], without_rowid: None, interleave_in_parent: None, partition: None }, false)]
#[case(Operation::DropTable { name: "t".to_string() }, false)]
#[case(Operation::RenameTable { old_name: "a".to_string(), new_name: "b".to_string() }, false)]
#[case(Operation::RenameColumn { table: "t".to_string(), old_name: "a".to_string(), new_name: "b".to_string() }, false)]
fn test_requires_sqlite_recreation(#[case] operation: Operation, #[case] expected: bool) {
	assert_eq!(
		operation.requires_sqlite_recreation(),
		expected,
		"Operation {:?} should {} require SQLite recreation",
		operation,
		if expected { "" } else { "not" }
	);
}

/// Test: AlterColumn requires SQLite recreation
///
/// Category: Unit Test
/// Separate test for AlterColumn since it requires a ColumnDefinition.
#[rstest]
fn test_alter_column_requires_sqlite_recreation() {
	let operation = Operation::AlterColumn {
		table: "t".to_string(),
		column: "c".to_string(),
		old_definition: None,
		new_definition: ColumnDefinition {
			name: "c".to_string(),
			type_definition: FieldType::Text,
			not_null: false,
			primary_key: false,
			unique: false,
			auto_increment: false,
			default: None,
		},
		mysql_options: None,
	};

	assert!(
		operation.requires_sqlite_recreation(),
		"AlterColumn should require SQLite recreation"
	);
}

// ============================================================================
// reverse_requires_sqlite_recreation Tests
// ============================================================================

/// Test: reverse_requires_sqlite_recreation identifies reverse operations
///
/// Category: Unit Test
/// Verifies that operations whose reverse requires recreation are identified.
#[rstest]
#[case(Operation::AddConstraint { table: "t".to_string(), constraint_sql: "...".to_string() }, true)]
#[case(Operation::DropConstraint { table: "t".to_string(), constraint_name: "c".to_string() }, true)]
#[case(Operation::CreateTable { name: "t".to_string(), columns: vec![], constraints: vec![], without_rowid: None, interleave_in_parent: None, partition: None }, false)]
#[case(Operation::DropTable { name: "t".to_string() }, false)]
fn test_reverse_requires_sqlite_recreation(#[case] operation: Operation, #[case] expected: bool) {
	assert_eq!(
		operation.reverse_requires_sqlite_recreation(),
		expected,
		"Reverse of {:?} should {} require SQLite recreation",
		operation,
		if expected { "" } else { "not" }
	);
}

/// Test: AddColumn reverse requires SQLite recreation
///
/// Category: Unit Test
/// The reverse of AddColumn is DropColumn, which requires recreation.
#[rstest]
fn test_add_column_reverse_requires_sqlite_recreation() {
	let operation = Operation::AddColumn {
		table: "t".to_string(),
		column: ColumnDefinition {
			name: "c".to_string(),
			type_definition: FieldType::Text,
			not_null: false,
			primary_key: false,
			unique: false,
			auto_increment: false,
			default: None,
		},
		mysql_options: None,
	};

	assert!(
		operation.reverse_requires_sqlite_recreation(),
		"Reverse of AddColumn (DropColumn) should require SQLite recreation"
	);
}

/// Test: AlterColumn reverse requires SQLite recreation
///
/// Category: Unit Test
/// The reverse of AlterColumn is another AlterColumn, which requires recreation.
#[rstest]
fn test_alter_column_reverse_requires_sqlite_recreation() {
	let operation = Operation::AlterColumn {
		table: "t".to_string(),
		column: "c".to_string(),
		old_definition: None,
		new_definition: ColumnDefinition {
			name: "c".to_string(),
			type_definition: FieldType::Text,
			not_null: false,
			primary_key: false,
			unique: false,
			auto_increment: false,
			default: None,
		},
		mysql_options: None,
	};

	assert!(
		operation.reverse_requires_sqlite_recreation(),
		"Reverse of AlterColumn should require SQLite recreation"
	);
}

// ============================================================================
// to_reverse_operation Tests
// ============================================================================

/// Test: to_reverse_operation for CreateTable
///
/// Category: Unit Test
/// Reverse of CreateTable is DropTable.
#[rstest]
fn test_to_reverse_operation_create_table() {
	let operation = Operation::CreateTable {
		name: "users".to_string(),
		columns: vec![],
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_some(),
		"CreateTable should have a reverse operation"
	);
	if let Some(Operation::DropTable { name }) = reverse {
		assert_eq!(name, "users");
	} else {
		panic!("Expected DropTable, got {:?}", reverse);
	}
}

/// Test: to_reverse_operation for AddColumn
///
/// Category: Unit Test
/// Reverse of AddColumn is DropColumn.
#[rstest]
fn test_to_reverse_operation_add_column() {
	let operation = Operation::AddColumn {
		table: "users".to_string(),
		column: ColumnDefinition {
			name: "email".to_string(),
			type_definition: FieldType::Text,
			not_null: false,
			primary_key: false,
			unique: false,
			auto_increment: false,
			default: None,
		},
		mysql_options: None,
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_some(),
		"AddColumn should have a reverse operation"
	);
	if let Some(Operation::DropColumn { table, column }) = reverse {
		assert_eq!(table, "users");
		assert_eq!(column, "email");
	} else {
		panic!("Expected DropColumn, got {:?}", reverse);
	}
}

/// Test: to_reverse_operation for RenameTable
///
/// Category: Unit Test
/// Reverse of RenameTable is RenameTable with swapped names.
#[rstest]
fn test_to_reverse_operation_rename_table() {
	let operation = Operation::RenameTable {
		old_name: "old_users".to_string(),
		new_name: "new_users".to_string(),
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_some(),
		"RenameTable should have a reverse operation"
	);
	if let Some(Operation::RenameTable { old_name, new_name }) = reverse {
		assert_eq!(
			old_name, "new_users",
			"Old name should be the new name from original"
		);
		assert_eq!(
			new_name, "old_users",
			"New name should be the old name from original"
		);
	} else {
		panic!("Expected RenameTable, got {:?}", reverse);
	}
}

/// Test: to_reverse_operation for RenameColumn
///
/// Category: Unit Test
/// Reverse of RenameColumn is RenameColumn with swapped names.
#[rstest]
fn test_to_reverse_operation_rename_column() {
	let operation = Operation::RenameColumn {
		table: "users".to_string(),
		old_name: "old_name".to_string(),
		new_name: "new_name".to_string(),
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_some(),
		"RenameColumn should have a reverse operation"
	);
	if let Some(Operation::RenameColumn {
		table,
		old_name,
		new_name,
	}) = reverse
	{
		assert_eq!(table, "users");
		assert_eq!(
			old_name, "new_name",
			"Old name should be the new name from original"
		);
		assert_eq!(
			new_name, "old_name",
			"New name should be the old name from original"
		);
	} else {
		panic!("Expected RenameColumn, got {:?}", reverse);
	}
}

/// Test: to_reverse_operation for CreateIndex
///
/// Category: Unit Test
/// Reverse of CreateIndex is DropIndex.
#[rstest]
fn test_to_reverse_operation_create_index() {
	let operation = Operation::CreateIndex {
		table: "users".to_string(),
		columns: vec!["email".to_string()],
		unique: true,
		index_type: None,
		where_clause: None,
		concurrently: false,
		expressions: None,
		mysql_options: None,
		operator_class: None,
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_some(),
		"CreateIndex should have a reverse operation"
	);
	if let Some(Operation::DropIndex { table, columns }) = reverse {
		assert_eq!(table, "users");
		assert_eq!(columns, vec!["email".to_string()]);
	} else {
		panic!("Expected DropIndex, got {:?}", reverse);
	}
}

/// Test: to_reverse_operation for DropIndex
///
/// Category: Unit Test
/// Reverse of DropIndex is CreateIndex (basic, without advanced properties).
#[rstest]
fn test_to_reverse_operation_drop_index() {
	let operation = Operation::DropIndex {
		table: "users".to_string(),
		columns: vec!["email".to_string()],
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_some(),
		"DropIndex should have a reverse operation"
	);
	if let Some(Operation::CreateIndex {
		table,
		columns,
		unique,
		..
	}) = reverse
	{
		assert_eq!(table, "users");
		assert_eq!(columns, vec!["email".to_string()]);
		assert!(
			!unique,
			"Cannot determine uniqueness from DropIndex, defaults to false"
		);
	} else {
		panic!("Expected CreateIndex, got {:?}", reverse);
	}
}

/// Test: to_reverse_operation for RunSQL returns None
///
/// Category: Unit Test
/// RunSQL cannot be reversed as an Operation (uses reverse_sql string instead).
#[rstest]
fn test_to_reverse_operation_run_sql() {
	let operation = Operation::RunSQL {
		sql: "INSERT INTO users VALUES (1, 'test')".to_string(),
		reverse_sql: Some("DELETE FROM users WHERE id = 1".to_string()),
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_none(),
		"RunSQL should not have a reverse Operation (uses reverse_sql string)"
	);
}

/// Test: to_reverse_operation for AddConstraint
///
/// Category: Unit Test
/// Reverse of AddConstraint is DropConstraint.
#[rstest]
fn test_to_reverse_operation_add_constraint() {
	let operation = Operation::AddConstraint {
		table: "users".to_string(),
		constraint_sql: "CONSTRAINT uq_email UNIQUE (email)".to_string(),
	};

	let project_state = ProjectState::default();
	let reverse = operation.to_reverse_operation(&project_state).unwrap();

	assert!(
		reverse.is_some(),
		"AddConstraint should have a reverse operation"
	);
	if let Some(Operation::DropConstraint {
		table,
		constraint_name,
	}) = reverse
	{
		assert_eq!(table, "users");
		assert_eq!(constraint_name, "uq_email");
	} else {
		panic!("Expected DropConstraint, got {:?}", reverse);
	}
}

// ============================================================================
// SQL Generation Dialect Tests
// ============================================================================

/// Test: to_sql for DropColumn on different dialects
///
/// Category: Unit Test
/// Verifies that DropColumn generates appropriate SQL for each dialect.
#[rstest]
fn test_drop_column_sql_dialect() {
	let operation = Operation::DropColumn {
		table: "users".to_string(),
		column: "old_field".to_string(),
	};

	// PostgreSQL and MySQL support direct DROP COLUMN
	let pg_sql = operation.to_sql(&SqlDialect::Postgres);
	assert!(
		pg_sql.contains("ALTER TABLE") && pg_sql.contains("DROP COLUMN"),
		"PostgreSQL should use ALTER TABLE DROP COLUMN"
	);

	let mysql_sql = operation.to_sql(&SqlDialect::Mysql);
	assert!(
		mysql_sql.contains("ALTER TABLE") && mysql_sql.contains("DROP COLUMN"),
		"MySQL should use ALTER TABLE DROP COLUMN"
	);

	// SQLite also generates ALTER TABLE SQL, but executor uses recreation
	let sqlite_sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sqlite_sql.contains("ALTER TABLE") && sqlite_sql.contains("DROP COLUMN"),
		"SQLite to_sql still generates ALTER TABLE (executor handles recreation)"
	);
}
