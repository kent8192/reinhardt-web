use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;

// The `auth_group.name` unique key is already created by `0001_initial.rs`
// (its `CreateTable` carries a named `Constraint::Unique`
// "auth_group_name_uniq" on the `name` column). The redundant
// `Operation::AddConstraint` that `makemigrations` previously emitted here
// (generator bug reinhardt-web#4448, since fixed) is removed so a fresh
// migration run does not abort with
// `relation "auth_group_name_uniq" already exists` on PostgreSQL
// (reinhardt-web#5045). These files were generated before #4448 was fixed;
// dropping the duplicate matches what current `makemigrations` produces,
// mirroring the same correction already applied in `migrations/users/0002_*`.
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
