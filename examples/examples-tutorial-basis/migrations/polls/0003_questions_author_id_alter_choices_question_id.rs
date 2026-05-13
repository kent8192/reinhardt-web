use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "polls".to_string(),
		name: "0003_questions_author_id_alter_choices_question_id".to_string(),
		operations: vec![
			Operation::AddColumn {
				table: "questions".to_string(),
				column: ColumnDefinition {
					name: "author_id".to_string(),
					type_definition: FieldType::Uuid,
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				mysql_options: None,
			},
			Operation::AlterColumn {
				table: "choices".to_string(),
				column: "question_id".to_string(),
				old_definition: None,
				new_definition: ColumnDefinition {
					name: "question_id".to_string(),
					type_definition: FieldType::Uuid,
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				mysql_options: None,
			},
		],
		dependencies: vec![(
			"polls".to_string(),
			"0002_alter_choices_question_id".to_string(),
		)],
		atomic: true,
		replaces: vec![],
		initial: None,
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
