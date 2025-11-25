//! Initial migration for users table

use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};

/// Initial migration creating users table
pub struct Migration;

impl Migration {
	/// Apply migration - create users table
	pub fn up() -> String {
		let users_table = Table::create()
			.table(UsersTable::Table)
			.col(
				ColumnDef::new(UsersTable::Id)
					.big_integer()
					.auto_increment()
					.primary_key()
					.not_null(),
			)
			.col(ColumnDef::new(UsersTable::Name).string_len(255).not_null())
			.col(
				ColumnDef::new(UsersTable::Email)
					.string_len(255)
					.not_null()
					.unique_key(),
			)
			.col(
				ColumnDef::new(UsersTable::CreatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default("CURRENT_TIMESTAMP"),
			)
			.to_owned();

		users_table.to_string(PostgresQueryBuilder)
	}

	/// Rollback migration - drop users table
	pub fn down() -> String {
		Table::drop()
			.table(UsersTable::Table)
			.to_owned()
			.to_string(PostgresQueryBuilder)
	}
}

/// Users table identifier
#[derive(Iden)]
enum UsersTable {
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
