use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "polls".to_string(),
		name: "0003_questions_author_id_alter_choices_question_id".to_string(),
		operations: vec![
			// `author_id` references `users.id` (BigInteger) and the model
			// field `Question::author: ForeignKeyField<User>` is non-optional,
			// so the column must be BigInteger + NOT NULL to mirror the
			// model contract. Tutorial migrations assume a fresh database;
			// pre-existing rows are not expected.
			Operation::AddColumn {
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
			},
			// `question_id` references `questions.id` (BigInteger). The
			// previous Uuid declaration was a copy-paste mistake.
			Operation::AlterColumn {
				table: "choices".to_string(),
				column: "question_id".to_string(),
				old_definition: None,
				new_definition: ColumnDefinition {
					name: "question_id".to_string(),
					type_definition: FieldType::BigInteger,
					not_null: true,
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
