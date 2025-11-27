//! Initial migration for users table

use sea_query::{ColumnDef, Expr, Iden, PostgresQueryBuilder, Table};

/// Initial migration creating users table
pub struct Migration;

impl Migration {
	/// Apply migration - create users table
	pub fn up() -> String {
		let users_table = Table::create()
			.table(Users::Table)
			.col(
				ColumnDef::new(Users::Id)
					.big_integer()
					.auto_increment()
					.primary_key()
					.not_null(),
			)
			.col(ColumnDef::new(Users::Name).string_len(255).not_null())
			.col(
				ColumnDef::new(Users::Email)
					.string_len(255)
					.not_null()
					.unique_key(),
			)
			.col(
				ColumnDef::new(Users::CreatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default(Expr::current_timestamp()),
			)
			.to_owned();

		users_table.to_string(PostgresQueryBuilder)
	}

	/// Rollback migration - drop users table
	pub fn down() -> String {
		Table::drop()
			.table(Users::Table)
			.to_owned()
			.to_string(PostgresQueryBuilder)
	}
}

/// Users table identifier
/// Note: SeaQuery's #[derive(Iden)] converts enum name to snake_case for Table variant
/// e.g., Users::Table -> "users", Todos::Table -> "todos"
#[derive(Iden)]
enum Users {
	Table,
	Id,
	Name,
	Email,
	CreatedAt,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_migration_up() {
		let sql = Migration::up();
		assert!(sql.contains("CREATE TABLE"));
		assert!(sql.contains("users"));
		assert!(sql.contains("name"));
		assert!(sql.contains("email"));
		assert!(sql.contains("created_at"));
		assert!(sql.contains("UNIQUE"));
	}

	#[test]
	fn test_migration_down() {
		let sql = Migration::down();
		assert!(sql.contains("DROP TABLE"));
		assert!(sql.contains("users"));
	}
}
