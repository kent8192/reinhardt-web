use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "auth".to_string(),
		name: "0002_alter_auth_permission_id_alter_auth_group_i_and_more".to_string(),
		operations: vec![
			Operation::AlterColumn {
				table: "auth_permission".to_string(),
				column: "id".to_string(),
				old_definition: None,
				new_definition: ColumnDefinition {
					name: "id".to_string(),
					type_definition: FieldType::Uuid,
					not_null: true,
					unique: false,
					primary_key: true,
					auto_increment: false,
					default: None,
				},
				mysql_options: None,
			},
			Operation::AlterColumn {
				table: "auth_group".to_string(),
				column: "id".to_string(),
				old_definition: None,
				new_definition: ColumnDefinition {
					name: "id".to_string(),
					type_definition: FieldType::Uuid,
					not_null: true,
					unique: false,
					primary_key: true,
					auto_increment: false,
					default: None,
				},
				mysql_options: None,
			},
			Operation::AddConstraint {
				table: "auth_group".to_string(),
				constraint_sql: "CONSTRAINT auth_group_name_uniq UNIQUE (name)".to_string(),
			},
		],
		dependencies: vec![("auth".to_string(), "0001_initial".to_string())],
		atomic: true,
		replaces: vec![],
		initial: None,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
