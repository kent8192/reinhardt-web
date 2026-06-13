use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;

pub(super) fn migration() -> Migration {
	Migration {
		app_label: "polls".to_string(),
		name: "0002_question_author".to_string(),
		operations: vec![Operation::AddColumn {
			table: "questions".to_string(),
			column: ColumnDefinition {
				name: "author_id".to_string(),
				type_definition: FieldType::BigInteger,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
			mysql_options: None,
		}],
		dependencies: vec![("polls".to_string(), "0001_initial".to_string())],
		atomic: true,
		replaces: vec![],
		initial: None,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
