//! Initial migration for todos table

use sea_query::{ColumnDef, Expr, Iden, PostgresQueryBuilder, Table};

/// Initial migration creating todos table
pub struct Migration;

impl Migration {
	/// Apply migration - create todos table
	pub fn up() -> String {
		let todos_table = Table::create()
			.table(Todos::Table)
			.col(
				ColumnDef::new(Todos::Id)
					.big_integer()
					.auto_increment()
					.primary_key()
					.not_null(),
			)
			.col(ColumnDef::new(Todos::Title).string_len(255).not_null())
			.col(ColumnDef::new(Todos::Description).text())
			.col(
				ColumnDef::new(Todos::Completed)
					.boolean()
					.not_null()
					.default(false),
			)
			.col(
				ColumnDef::new(Todos::CreatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default(Expr::current_timestamp()),
			)
			.col(
				ColumnDef::new(Todos::UpdatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default(Expr::current_timestamp()),
			)
			.to_owned();

		todos_table.to_string(PostgresQueryBuilder)
	}

	/// Rollback migration - drop todos table
	pub fn down() -> String {
		Table::drop()
			.table(Todos::Table)
			.to_owned()
			.to_string(PostgresQueryBuilder)
	}
}

/// Todos table identifier
/// Note: SeaQuery's #[derive(Iden)] converts enum name to snake_case for Table variant
/// e.g., Users::Table -> "users", Todos::Table -> "todos"
#[derive(Iden)]
enum TodosTable {
	Table,
	Id,
	Title,
	Description,
	Completed,
	CreatedAt,
	UpdatedAt,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_migration_up() {
		let sql = Migration::up();
		assert!(sql.contains("CREATE TABLE"));
		assert!(sql.contains("todos"));
		assert!(sql.contains("title"));
		assert!(sql.contains("description"));
		assert!(sql.contains("completed"));
		assert!(sql.contains("created_at"));
		assert!(sql.contains("updated_at"));
	}

	#[test]
	fn test_migration_down() {
		let sql = Migration::down();
		assert!(sql.contains("DROP TABLE"));
		assert!(sql.contains("todos"));
	}
}
