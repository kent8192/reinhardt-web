use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "users".to_string(),
		name: "0002_users_is_superuser_alter_users_is_active_ad_and_more".to_string(),
		operations: vec![
			Operation::AddColumn {
				table: "users".to_string(),
				column: ColumnDefinition {
					name: "is_superuser".to_string(),
					type_definition: FieldType::Boolean,
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: Some("false".to_string()),
				},
				mysql_options: None,
			},
			Operation::AlterColumn {
				table: "users".to_string(),
				column: "is_active".to_string(),
				old_definition: None,
				new_definition: ColumnDefinition {
					name: "is_active".to_string(),
					type_definition: FieldType::Boolean,
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: Some("true".to_string()),
				},
				mysql_options: None,
			},
		],
		dependencies: vec![("users".to_string(), "0001_initial".to_string())],
		atomic: true,
		replaces: vec![],
		initial: None,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
