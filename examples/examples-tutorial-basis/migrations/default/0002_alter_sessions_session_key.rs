use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "default".to_string(),
		name: "0002_alter_sessions_session_key".to_string(),
		operations: vec![Operation::AlterColumn {
			table: "sessions".to_string(),
			column: "session_key".to_string(),
			old_definition: None,
			new_definition: ColumnDefinition {
				name: "session_key".to_string(),
				type_definition: FieldType::VarChar(255u32),
				not_null: true,
				unique: false,
				primary_key: true,
				auto_increment: false,
				default: None,
			},
			mysql_options: None,
		}],
		dependencies: vec![("default".to_string(), "0001_initial".to_string())],
		atomic: true,
		replaces: vec![],
		initial: None,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
