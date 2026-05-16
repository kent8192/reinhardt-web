use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;

// Workaround for reinhardt-web#4448: `makemigrations` still emits a
// redundant `Operation::AddConstraint` for `UNIQUE (username)` here,
// even though `users.username` is already declared unique by
// `0001_initial.rs` (the SQLite schema for the initial migration
// produces `CONSTRAINT sqlite_autoindex_users_1 UNIQUE (username)`).
// Re-adding the same constraint is at best redundant and at worst broken:
// SQLite does not support `ALTER TABLE ADD CONSTRAINT`, so on a stricter
// dialect this migration would fail outright. The duplicate operation is
// removed by hand below.
//
// Remove this workaround when the upstream issue is resolved.
//
// Ideal implementation (without workaround): rerun
//   cargo run --bin manage makemigrations users
// once #4448 is fixed. `makemigrations` will then stop appending a
// redundant UNIQUE constraint for an already-unique column. The
// regenerated file will match the body below verbatim, and this header
// comment can be deleted in the same change.
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
