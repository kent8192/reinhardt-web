//! Tests for MigrationOperation trait implementation
//!
//! This test file verifies that the Operation enum correctly implements
//! the MigrationOperation trait for Django-style migration naming.

use reinhardt_db::migrations::{ColumnDefinition, FieldType, MigrationOperation, Operation};

#[test]
fn test_create_table_fragment() {
	let op = Operation::CreateTable {
		name: "users".to_string(),
		columns: vec![],
		constraints: vec![],
		without_rowid: None,
		partition: None,
		interleave_in_parent: None,
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
		name: "Users".to_string(),
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
		table: "Users".to_string(),
		column: ColumnDefinition::new("email", FieldType::Custom("VARCHAR(255)".to_string())),
		mysql_options: None,
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
		table: "Users".to_string(),
		column: "Age".to_string(),
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
		table: "Users".to_string(),
		column: "Email".to_string(),
		old_definition: None,
		new_definition: ColumnDefinition::new("email", FieldType::Custom("TEXT".to_string())),
		mysql_options: None,
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
		old_name: "OldTable".to_string(),
		new_name: "NewTable".to_string(),
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
		table: "Users".to_string(),
		old_name: "old_field".to_string(),
		new_name: "new_field".to_string(),
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
		table: "Users".to_string(),
		constraint_sql: "UNIQUE(email)".to_string(),
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
		table: "Users".to_string(),
		constraint_name: "UniqueEmail".to_string(),
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
		table: "Users".to_string(),
		columns: vec!["email".to_string()],
		unique: false,
		index_type: None,
		where_clause: None,
		concurrently: false,
		expressions: None,
		mysql_options: None,
		operator_class: None,
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
		table: "Users".to_string(),
		columns: vec!["email".to_string()],
		unique: true,
		index_type: None,
		where_clause: None,
		concurrently: false,
		expressions: None,
		mysql_options: None,
		operator_class: None,
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
		table: "Users".to_string(),
		columns: vec!["email".to_string()],
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
		sql: "CREATE TRIGGER ...".to_string(),
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
		code: "fn migrate() { ... }".to_string(),
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
		table: "Users".to_string(),
		comment: Some("User account table".to_string()),
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
		table: "OrderItem".to_string(),
		unique_together: vec![vec!["order_id".to_string(), "product_id".to_string()]],
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
	options.insert("ordering".to_string(), "username".to_string());

	let op = Operation::AlterModelOptions {
		table: "User".to_string(),
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
		name: "AdminUser".to_string(),
		columns: vec![],
		base_table: "User".to_string(),
		join_column: "user_id".to_string(),
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
		table: "Animal".to_string(),
		column_name: "animal_type".to_string(),
		default_value: "animal".to_string(),
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
			name: "MyTable".to_string(),
			columns: vec![],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		},
		Operation::AddColumn {
			table: "MyTable".to_string(),
			column: ColumnDefinition::new("MyField", FieldType::Custom("TEXT".to_string())),
			mysql_options: None,
		},
		Operation::DropColumn {
			table: "MyTable".to_string(),
			column: "MyField".to_string(),
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
