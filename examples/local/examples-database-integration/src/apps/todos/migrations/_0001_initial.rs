//! Initial migration for todos table

use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};

/// Initial migration creating todos table
pub struct Migration;

impl Migration {
	/// Apply migration - create todos table
	pub fn up() -> String {
		let todos_table = Table::create()
			.table(TodosTable::Table)
			.col(
				ColumnDef::new(TodosTable::Id)
					.big_integer()
					.auto_increment()
					.primary_key()
					.not_null(),
			)
			.col(ColumnDef::new(TodosTable::Title).string_len(255).not_null())
			.col(ColumnDef::new(TodosTable::Description).text())
			.col(
				ColumnDef::new(TodosTable::Completed)
					.boolean()
					.not_null()
					.default(false),
			)
			.col(
				ColumnDef::new(TodosTable::CreatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default("CURRENT_TIMESTAMP"),
			)
			.col(
				ColumnDef::new(TodosTable::UpdatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default("CURRENT_TIMESTAMP"),
			)
			.to_owned();

		todos_table.to_string(PostgresQueryBuilder)
	}

	/// Rollback migration - drop todos table
	pub fn down() -> String {
		Table::drop()
			.table(TodosTable::Table)
			.to_owned()
			.to_string(PostgresQueryBuilder)
	}
}

/// Todos table identifier
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
