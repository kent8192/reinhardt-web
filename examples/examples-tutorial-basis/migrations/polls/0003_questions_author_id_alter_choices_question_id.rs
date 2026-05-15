// Workaround for reinhardt-web#4430 and reinhardt-web#4431.
//
// This file is the output of `cargo make makemigrations`, but the auto-generated
// version has two bugs in the FK column emission that make the polls tutorial
// non-functional:
//
//   reinhardt-web#4430: FK columns are always emitted as `FieldType::Uuid`
//     regardless of the referenced model's PK type. `Question::author:
//     ForeignKeyField<User>` should map to `BigInteger` (User.id is `i64`),
//     not `Uuid`.
//
//   reinhardt-web#4431: FK columns ignore `ForeignKeyField<T>` vs
//     `Option<ForeignKeyField<T>>` and always emit `not_null: false`. The
//     non-optional `Question::author` field should produce `not_null: true`.
//
// Until both issues are fixed in `crates/reinhardt-core/macros/src/model_derive.rs`
// (and the autodetector path), the `author_id` and `choices.question_id` entries
// below are hand-edited to the correct values. Re-running `cargo make
// makemigrations` will silently regress this file; verify the diff or wait
// until reinhardt-web#4430 + #4431 ship.
//
// Ideal implementation (post-fix, identical to what the generator should
// produce on its own):
//
//   Operation::AddColumn {
//       table: "questions".to_string(),
//       column: ColumnDefinition {
//           name: "author_id".to_string(),
//           type_definition: FieldType::BigInteger,   // inferred from User.id
//           not_null: true,                           // inferred from non-Option FK
//           unique: false,
//           primary_key: false,
//           auto_increment: false,
//           default: None,
//       },
//       mysql_options: None,
//   },
//   Operation::AlterColumn {
//       table: "choices".to_string(),
//       column: "question_id".to_string(),
//       old_definition: None,
//       new_definition: ColumnDefinition {
//           name: "question_id".to_string(),
//           type_definition: FieldType::BigInteger,   // inferred from Question.id
//           not_null: true,
//           unique: false,
//           primary_key: false,
//           auto_increment: false,
//           default: None,
//       },
//       mysql_options: None,
//   },

use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {
	Migration {
		app_label: "polls".to_string(),
		name: "0003_questions_author_id_alter_choices_question_id".to_string(),
		operations: vec![
			// Workaround for reinhardt-web#4430 + #4431:
			// generator emits `FieldType::Uuid` + `not_null: false`.
			// Hand-corrected to match `User.id: i64` PK and the non-optional
			// `Question::author: ForeignKeyField<User>` model contract.
			// Tutorial migrations assume a fresh database; pre-existing rows
			// are not expected.
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
