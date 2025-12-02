//! Tests for MigrationOperation trait implementation
//!
//! This test file verifies that the Operation enum correctly implements
//! the MigrationOperation trait for Django-style migration naming.

use reinhardt_migrations::{ColumnDefinition, MigrationOperation, Operation};

/// Helper function to leak a string to get a 'static lifetime
fn leak_str(s: impl Into<String>) -> &'static str {
	Box::leak(s.into().into_boxed_str())
}

#[test]
fn test_create_table_fragment() {
	let op = Operation::CreateTable {
		name: leak_str("users"),
		columns: vec![],
		constraints: vec![],
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("users".to_string()),
		"CreateTable should generate lowercase table name fragment"
	);
	assert_eq!(
		op.describe(),
		"Create table users",
		"CreateTable should have descriptive message"
	);
}

#[test]
fn test_drop_table_fragment() {
	let op = Operation::DropTable {
		name: leak_str("Users"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("delete_users".to_string()),
		"DropTable should generate 'delete_' prefix with lowercase name"
	);
	assert_eq!(
		op.describe(),
		"Drop table Users",
		"DropTable should have descriptive message"
	);
}

#[test]
fn test_add_column_fragment() {
	let op = Operation::AddColumn {
		table: leak_str("Users"),
		column: ColumnDefinition::new("email", "VARCHAR(255)"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("users_email".to_string()),
		"AddColumn should generate 'table_column' format"
	);
	assert_eq!(
		op.describe(),
		"Add column email to Users",
		"AddColumn should have descriptive message"
	);
}

#[test]
fn test_drop_column_fragment() {
	let op = Operation::DropColumn {
		table: leak_str("Users"),
		column: leak_str("Age"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("remove_users_age".to_string()),
		"DropColumn should generate 'remove_table_column' format"
	);
	assert_eq!(
		op.describe(),
		"Drop column Age from Users",
		"DropColumn should have descriptive message"
	);
}

#[test]
fn test_alter_column_fragment() {
	let op = Operation::AlterColumn {
		table: leak_str("Users"),
		column: leak_str("Email"),
		new_definition: ColumnDefinition::new("email", "TEXT"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("alter_users_email".to_string()),
		"AlterColumn should generate 'alter_table_column' format"
	);
	assert_eq!(
		op.describe(),
		"Alter column Email on Users",
		"AlterColumn should have descriptive message"
	);
}

#[test]
fn test_rename_table_fragment() {
	let op = Operation::RenameTable {
		old_name: leak_str("OldTable"),
		new_name: leak_str("NewTable"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("rename_oldtable_to_newtable".to_string()),
		"RenameTable should generate 'rename_old_to_new' format"
	);
	assert_eq!(
		op.describe(),
		"Rename table OldTable to NewTable",
		"RenameTable should have descriptive message"
	);
}

#[test]
fn test_rename_column_fragment() {
	let op = Operation::RenameColumn {
		table: leak_str("Users"),
		old_name: leak_str("old_field"),
		new_name: leak_str("new_field"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("rename_users_new_field".to_string()),
		"RenameColumn should generate 'rename_table_newname' format"
	);
	assert_eq!(
		op.describe(),
		"Rename column old_field to new_field on Users",
		"RenameColumn should have descriptive message"
	);
}

#[test]
fn test_add_constraint_fragment() {
	let op = Operation::AddConstraint {
		table: leak_str("Users"),
		constraint_sql: leak_str("UNIQUE(email)"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("add_constraint_users".to_string()),
		"AddConstraint should generate 'add_constraint_table' format"
	);
	assert_eq!(
		op.describe(),
		"Add constraint on Users",
		"AddConstraint should have descriptive message"
	);
}

#[test]
fn test_drop_constraint_fragment() {
	let op = Operation::DropConstraint {
		table: leak_str("Users"),
		constraint_name: leak_str("UniqueEmail"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("drop_constraint_uniqueemail".to_string()),
		"DropConstraint should generate 'drop_constraint_name' format"
	);
	assert_eq!(
		op.describe(),
		"Drop constraint UniqueEmail from Users",
		"DropConstraint should have descriptive message"
	);
}

#[test]
fn test_create_index_fragment() {
	let op = Operation::CreateIndex {
		table: leak_str("Users"),
		columns: vec!["email"],
		unique: false,
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("create_index_users".to_string()),
		"CreateIndex should generate 'create_index_table' format"
	);
	assert_eq!(
		op.describe(),
		"Create index on Users",
		"CreateIndex should have descriptive message"
	);
}

#[test]
fn test_create_unique_index_fragment() {
	let op = Operation::CreateIndex {
		table: leak_str("Users"),
		columns: vec!["email"],
		unique: true,
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("create_unique_index_users".to_string()),
		"CreateIndex with unique=true should generate 'create_unique_index_table' format"
	);
	assert_eq!(
		op.describe(),
		"Create unique index on Users",
		"CreateIndex with unique=true should have descriptive message"
	);
}

#[test]
fn test_drop_index_fragment() {
	let op = Operation::DropIndex {
		table: leak_str("Users"),
		columns: vec!["email"],
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("drop_index_users".to_string()),
		"DropIndex should generate 'drop_index_table' format"
	);
	assert_eq!(
		op.describe(),
		"Drop index on Users",
		"DropIndex should have descriptive message"
	);
}

#[test]
fn test_run_sql_no_fragment() {
	let op = Operation::RunSQL {
		sql: leak_str("CREATE TRIGGER ..."),
		reverse_sql: None,
	};

	assert_eq!(
		op.migration_name_fragment(),
		None,
		"RunSQL should return None to trigger auto-naming"
	);
	assert!(
		op.describe().starts_with("RunSQL:"),
		"RunSQL should have descriptive message starting with 'RunSQL:'"
	);
}

#[test]
fn test_run_rust_no_fragment() {
	let op = Operation::RunRust {
		code: leak_str("fn migrate() { ... }"),
		reverse_code: None,
	};

	assert_eq!(
		op.migration_name_fragment(),
		None,
		"RunRust should return None to trigger auto-naming"
	);
	assert!(
		op.describe().starts_with("RunRust:"),
		"RunRust should have descriptive message starting with 'RunRust:'"
	);
}

#[test]
fn test_alter_table_comment_fragment() {
	let op = Operation::AlterTableComment {
		table: leak_str("Users"),
		comment: Some("User account table"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("alter_comment_users".to_string()),
		"AlterTableComment should generate 'alter_comment_table' format"
	);
	assert_eq!(
		op.describe(),
		"Set comment on Users to 'User account table'",
		"AlterTableComment should have descriptive message"
	);
}

#[test]
fn test_alter_unique_together_fragment() {
	let op = Operation::AlterUniqueTogether {
		table: leak_str("OrderItem"),
		unique_together: vec![vec!["order_id", "product_id"]],
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("alter_unique_orderitem".to_string()),
		"AlterUniqueTogether should generate 'alter_unique_table' format"
	);
	assert_eq!(
		op.describe(),
		"Alter unique_together on OrderItem",
		"AlterUniqueTogether should have descriptive message"
	);
}

#[test]
fn test_alter_model_options_fragment() {
	use std::collections::HashMap;

	let mut options = HashMap::new();
	options.insert("ordering", "username");

	let op = Operation::AlterModelOptions {
		table: leak_str("User"),
		options,
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("alter_options_user".to_string()),
		"AlterModelOptions should generate 'alter_options_table' format"
	);
	assert_eq!(
		op.describe(),
		"Alter model options on User",
		"AlterModelOptions should have descriptive message"
	);
}

#[test]
fn test_create_inherited_table_fragment() {
	let op = Operation::CreateInheritedTable {
		name: leak_str("AdminUser"),
		columns: vec![],
		base_table: leak_str("User"),
		join_column: leak_str("user_id"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("create_inherited_adminuser".to_string()),
		"CreateInheritedTable should generate 'create_inherited_name' format"
	);
	assert_eq!(
		op.describe(),
		"Create inherited table AdminUser from User",
		"CreateInheritedTable should have descriptive message"
	);
}

#[test]
fn test_add_discriminator_column_fragment() {
	let op = Operation::AddDiscriminatorColumn {
		table: leak_str("Animal"),
		column_name: leak_str("animal_type"),
		default_value: leak_str("animal"),
	};

	assert_eq!(
		op.migration_name_fragment(),
		Some("add_discriminator_animal".to_string()),
		"AddDiscriminatorColumn should generate 'add_discriminator_table' format"
	);
	assert_eq!(
		op.describe(),
		"Add discriminator column animal_type to Animal",
		"AddDiscriminatorColumn should have descriptive message"
	);
}

#[test]
fn test_case_insensitive_naming() {
	// Test that all fragments are lowercase
	let ops = vec![
		Operation::CreateTable {
			name: leak_str("MyTable"),
			columns: vec![],
			constraints: vec![],
		},
		Operation::AddColumn {
			table: leak_str("MyTable"),
			column: ColumnDefinition::new("MyField", "TEXT"),
		},
		Operation::DropColumn {
			table: leak_str("MyTable"),
			column: leak_str("MyField"),
		},
	];

	for op in ops {
		if let Some(fragment) = op.migration_name_fragment() {
			assert_eq!(
				fragment,
				fragment.to_lowercase(),
				"Fragment should be lowercase: {}",
				fragment
			);
		}
	}
}
