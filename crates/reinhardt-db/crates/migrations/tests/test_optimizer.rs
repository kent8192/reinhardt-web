//! Tests for migration optimizer
//! Adapted from Django's test_optimizer.py

use reinhardt_migrations::{
	ColumnDefinition, Constraint, FieldType, ForeignKeyAction, Migration, Operation,
	OperationOptimizer,
};

fn create_column(name: &str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
	}
}

#[test]
fn test_create_delete_table_optimization() {
	// CreateTable followed by DropTable should be optimized away
	let operations = vec![
		Operation::CreateTable {
			name: "test_table".to_string(),
			columns: vec![create_column(
				"id",
				FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		},
		Operation::DropTable {
			name: "test_table".to_string(),
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// Both operations should be removed (they cancel each other)
	assert_eq!(optimized.len(), 0);
}

#[test]
fn test_add_remove_field_optimization() {
	// AddColumn followed by DropColumn should be optimized away
	let operations = vec![
		Operation::AddColumn {
			table: "test_table".to_string(),
			column: create_column("temp", FieldType::Text),
			mysql_options: None,
		},
		Operation::DropColumn {
			table: "test_table".to_string(),
			column: "temp".to_string(),
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// Both operations should be removed (they cancel each other)
	assert_eq!(optimized.len(), 0);
}

#[test]
fn test_consecutive_alter_optimization() {
	// Multiple AlterColumn on same field should be merged
	let operations = vec![
		Operation::AlterColumn {
			table: "test_table".to_string(),
			column: "field".to_string(),
			old_definition: None,
			new_definition: create_column("field", FieldType::Integer),
			mysql_options: None,
		},
		Operation::AlterColumn {
			table: "test_table".to_string(),
			column: "field".to_string(),
			old_definition: None,
			new_definition: create_column(
				"field",
				FieldType::Custom("INTEGER NOT NULL".to_string()),
			),
			mysql_options: None,
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// Should be merged into a single AlterColumn (last one wins)
	assert_eq!(optimized.len(), 1);
	match &optimized[0] {
		Operation::AlterColumn {
			table,
			column,
			new_definition,
			..
		} => {
			assert_eq!(table, "test_table");
			assert_eq!(column, "field");
			assert_eq!(
				new_definition.type_definition.to_sql_string(),
				FieldType::Custom("INTEGER NOT NULL".to_string()).to_sql_string()
			);
		}
		_ => panic!("Expected AlterColumn operation"),
	}
}

#[test]
fn test_rename_table_optimization() {
	// Multiple RenameTable operations should be chained
	let operations = vec![
		Operation::RenameTable {
			old_name: "old".to_string(),
			new_name: "temp".to_string(),
		},
		Operation::RenameTable {
			old_name: "temp".to_string(),
			new_name: "new".to_string(),
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// Debug: Print optimized operations
	eprintln!("Optimized operations: {:?}", optimized);

	// Should be chained into a single RenameTable (old -> new)
	assert_eq!(optimized.len(), 1);
	match &optimized[0] {
		Operation::RenameTable { old_name, new_name } => {
			assert_eq!(old_name, "old");
			assert_eq!(new_name, "new");
		}
		_ => panic!("Expected RenameTable operation"),
	}
}

#[test]
fn test_create_table_with_add_column() {
	// CreateTable followed by AddColumn (not currently optimized)
	let operations = vec![
		Operation::CreateTable {
			name: "test_table".to_string(),
			columns: vec![create_column(
				"id",
				FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		},
		Operation::AddColumn {
			table: "test_table".to_string(),
			column: create_column("name", FieldType::Text),
			mysql_options: None,
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// NOTE: Current optimizer does not merge AddColumn into CreateTable
	// This optimization requires modifying CreateTable.columns, which is complex
	// Future enhancement: Merge AddColumn into CreateTable.columns
	assert_eq!(optimized.len(), 2);
}

#[test]
fn test_no_optimization_needed() {
	// Different tables - no optimization needed
	let operations = vec![
		Operation::CreateTable {
			name: "table1".to_string(),
			columns: vec![create_column(
				"id",
				FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		},
		Operation::CreateTable {
			name: "table2".to_string(),
			columns: vec![create_column(
				"id",
				FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// No optimization needed (different tables)
	assert_eq!(optimized.len(), 2);
}

#[test]
fn test_index_optimization() {
	// CreateIndex followed by DropIndex should be optimized away
	let operations = vec![
		Operation::CreateIndex {
			table: "test_table".to_string(),
			columns: vec!["field".to_string()],
			unique: false,
			index_type: None,
			where_clause: None,
			concurrently: false,
			expressions: None,
			mysql_options: None,
			operator_class: None,
		},
		Operation::DropIndex {
			table: "test_table".to_string(),
			columns: vec!["field".to_string()],
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// Both operations should be removed (they cancel each other)
	assert_eq!(optimized.len(), 0);
}

#[test]
fn test_constraint_optimization() {
	// AddConstraint followed by DropConstraint should be optimized away
	let operations = vec![
		Operation::AddConstraint {
			table: "test_table".to_string(),
			constraint_sql: "CONSTRAINT check_temp CHECK (value > 0)".to_string(),
		},
		Operation::DropConstraint {
			table: "test_table".to_string(),
			constraint_name: "check_temp".to_string(),
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations);

	// Both operations should be removed (approximate matching by table)
	// NOTE: Current implementation uses approximate matching (by table only)
	// Perfect matching would require parsing constraint_sql to extract name
	assert_eq!(optimized.len(), 0);
}

#[test]
fn test_migration_with_no_operations() {
	// Migration with no operations
	let migration = Migration::new("0001_empty", "testapp");

	assert_eq!(migration.operations.len(), 0);
}

#[test]
fn test_migration_optimization_preserves_order() {
	// Optimization should preserve operation order when necessary
	let operations = vec![
		Operation::CreateTable {
			name: "table1".to_string(),
			columns: vec![create_column(
				"id",
				FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		},
		Operation::CreateTable {
			name: "table2".to_string(),
			columns: vec![
				create_column("id", FieldType::Custom("INTEGER PRIMARY KEY".to_string())),
				create_column("table1_id", FieldType::Integer),
			],
			constraints: vec![Constraint::ForeignKey {
				name: "fk_table2_table1".to_string(),
				columns: vec!["table1_id".to_string()],
				referenced_table: "table1".to_string(),
				referenced_columns: vec!["id".to_string()],
				on_delete: ForeignKeyAction::Cascade,
				on_update: ForeignKeyAction::NoAction,
				deferrable: None,
			}],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations.clone());

	// Order must be preserved (table2 depends on table1)
	assert_eq!(optimized.len(), 2);
	match &optimized[0] {
		Operation::CreateTable { name, .. } => assert_eq!(name, "table1"),
		_ => panic!("Expected CreateTable for table1"),
	}
	match &optimized[1] {
		Operation::CreateTable { name, .. } => assert_eq!(name, "table2"),
		_ => panic!("Expected CreateTable for table2"),
	}
}
