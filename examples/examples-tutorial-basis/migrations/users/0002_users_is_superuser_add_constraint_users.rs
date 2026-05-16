use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;

// Workaround for reinhardt-web#4447: `makemigrations` did not translate
// `#[field(default = false)]` on `apps::users::User::is_superuser` into
// `ColumnDefinition.default`, so the auto-generated file initially had
// `default: None`. That works on a freshly-created (empty) `users` table
// because SQLite accepts a NOT NULL ADD COLUMN with no default when
// there are zero rows, but it would fail on any database with existing
// user rows ("Cannot add a NOT NULL column with default value NULL").
// The `Some("0".to_string())` below is added by hand so populated
// databases can apply this migration without manual SQL.
//
// Workaround for reinhardt-web#4448: `makemigrations` additionally
// emitted a second `Operation::AddConstraint` for `UNIQUE (username)`
// alongside the column add, even though `users.username` is already
// declared unique by `0001_initial.rs` (the SQLite schema for the
// initial migration produces `CONSTRAINT sqlite_autoindex_users_1
// UNIQUE (username)`). Adding the second `users_user_username_uniq
// UNIQUE (username)` is at best redundant and at worst broken: SQLite
// does not support `ALTER TABLE ADD CONSTRAINT`, so on a stricter
// dialect this migration would fail outright. The duplicate operation
// is removed by hand below.
//
// Remove these workarounds when the upstream issues are resolved.
//
// Ideal implementation (without workaround): rerun
//   cargo run --bin manage makemigrations users
// once #4447 and #4448 are fixed. `makemigrations` will then emit
// `default: Some("0".to_string())` automatically from the
// `#[field(default = false)]` annotation **and** stop appending a
// redundant UNIQUE constraint for an already-unique column. The
// regenerated file will match the body below verbatim, and this
// header comment can be deleted in the same change.
pub fn migration() -> Migration {
	Migration {
		app_label: "users".to_string(),
		name: "0002_users_is_superuser_add_constraint_users".to_string(),
		operations: vec![Operation::AddColumn {
			table: "users".to_string(),
			column: ColumnDefinition {
				name: "is_superuser".to_string(),
				type_definition: FieldType::Boolean,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: Some("0".to_string()),
			},
			mysql_options: None,
		}],
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
