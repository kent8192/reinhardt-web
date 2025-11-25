//! Initial migration for articles table

use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};

/// Initial migration creating articles table
pub struct Migration;

impl Migration {
	/// Apply migration - create articles table
	pub fn up() -> String {
		let articles_table = Table::create()
			.table(ArticlesTable::Table)
			.col(
				ColumnDef::new(ArticlesTable::Id)
					.big_integer()
					.auto_increment()
					.primary_key()
					.not_null(),
			)
			.col(ColumnDef::new(ArticlesTable::Title).string_len(255).not_null())
			.col(ColumnDef::new(ArticlesTable::Content).text().not_null())
			.col(ColumnDef::new(ArticlesTable::Author).string_len(100).not_null())
			.col(
				ColumnDef::new(ArticlesTable::Published)
					.boolean()
					.not_null()
					.default(false),
			)
			.col(
				ColumnDef::new(ArticlesTable::CreatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default("CURRENT_TIMESTAMP"),
			)
			.col(
				ColumnDef::new(ArticlesTable::UpdatedAt)
					.timestamp_with_time_zone()
					.not_null()
					.default("CURRENT_TIMESTAMP"),
			)
			.to_owned();

		articles_table.to_string(PostgresQueryBuilder)
	}

	/// Rollback migration - drop articles table
	pub fn down() -> String {
		Table::drop()
			.table(ArticlesTable::Table)
			.to_owned()
			.to_string(PostgresQueryBuilder)
	}
}

/// Articles table identifier
#[derive(Iden)]
enum ArticlesTable {
	Table,
	Id,
	Title,
	Content,
	Author,
	Published,
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
		assert!(sql.contains("articles"));
		assert!(sql.contains("title"));
		assert!(sql.contains("content"));
		assert!(sql.contains("author"));
		assert!(sql.contains("published"));
		assert!(sql.contains("created_at"));
		assert!(sql.contains("updated_at"));
	}

	#[test]
	fn test_migration_down() {
		let sql = Migration::down();
		assert!(sql.contains("DROP TABLE"));
		assert!(sql.contains("articles"));
	}
}
