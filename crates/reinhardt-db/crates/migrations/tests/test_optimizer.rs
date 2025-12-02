//! Tests for migration optimizer
//! Adapted from Django's test_optimizer.py

use reinhardt_migrations::{ColumnDefinition, Migration, Operation, OperationOptimizer};

fn create_column(name: &'static str, type_def: &'static str) -> ColumnDefinition {
	ColumnDefinition {
		name,
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
		max_length: None,
	}
}

#[test]
fn test_create_delete_table_optimization() {
	// CreateTable followed by DropTable should be optimized away
	let operations = vec![
		Operation::CreateTable {
			name: "test_table",
			columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
			constraints: vec![],
		},
		Operation::DropTable { name: "test_table" },
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
			table: "test_table",
			column: create_column("temp", "TEXT"),
		},
		Operation::DropColumn {
			table: "test_table",
			column: "temp",
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
			table: "test_table",
			column: "field",
			new_definition: create_column("field", "INTEGER"),
		},
		Operation::AlterColumn {
			table: "test_table",
			column: "field",
			new_definition: create_column("field", "INTEGER NOT NULL"),
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
		} => {
			assert_eq!(*table, "test_table");
			assert_eq!(*column, "field");
			assert_eq!(new_definition.type_definition, "INTEGER NOT NULL");
		}
		_ => panic!("Expected AlterColumn operation"),
	}
}

#[test]
fn test_rename_table_optimization() {
	// Multiple RenameTable operations should be chained
	let operations = vec![
		Operation::RenameTable {
			old_name: "old",
			new_name: "temp",
		},
		Operation::RenameTable {
			old_name: "temp",
			new_name: "new",
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
			assert_eq!(*old_name, "old");
			assert_eq!(*new_name, "new");
		}
		_ => panic!("Expected RenameTable operation"),
	}
}

#[test]
fn test_create_table_with_add_column() {
	// CreateTable followed by AddColumn (not currently optimized)
	let operations = vec![
		Operation::CreateTable {
			name: "test_table",
			columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
			constraints: vec![],
		},
		Operation::AddColumn {
			table: "test_table",
			column: create_column("name", "TEXT"),
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
			name: "table1",
			columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
			constraints: vec![],
		},
		Operation::CreateTable {
			name: "table2",
			columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
			constraints: vec![],
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
			table: "test_table",
			columns: vec!["field"],
			unique: false,
		},
		Operation::DropIndex {
			table: "test_table",
			columns: vec!["field"],
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
			table: "test_table",
			constraint_sql: "CONSTRAINT check_temp CHECK (value > 0)",
		},
		Operation::DropConstraint {
			table: "test_table",
			constraint_name: "check_temp",
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
	let migration = Migration {
		app_label: "testapp",
		name: "0001_empty",
		operations: vec![],
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: None,
	};

	assert_eq!(migration.operations.len(), 0);
}

#[test]
fn test_migration_optimization_preserves_order() {
	// Optimization should preserve operation order when necessary
	let operations = vec![
		Operation::CreateTable {
			name: "table1",
			columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
			constraints: vec![],
		},
		Operation::CreateTable {
			name: "table2",
			columns: vec![
				create_column("id", "INTEGER PRIMARY KEY"),
				create_column("table1_id", "INTEGER"),
			],
			constraints: vec!["FOREIGN KEY (table1_id) REFERENCES table1(id)"],
		},
	];

	let optimizer = OperationOptimizer::new();
	let optimized = optimizer.optimize(operations.clone());

	// Order must be preserved (table2 depends on table1)
	assert_eq!(optimized.len(), 2);
	match &optimized[0] {
		Operation::CreateTable { name, .. } => assert_eq!(*name, "table1"),
		_ => panic!("Expected CreateTable for table1"),
	}
	match &optimized[1] {
		Operation::CreateTable { name, .. } => assert_eq!(*name, "table2"),
		_ => panic!("Expected CreateTable for table2"),
	}
}
