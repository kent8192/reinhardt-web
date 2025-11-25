//! Initial migration for snippets app
//!
//! Creates the snippets table with the following schema:
//! - id: BIGSERIAL PRIMARY KEY
//! - title: TEXT NOT NULL
//! - code: TEXT NOT NULL
//! - language: TEXT NOT NULL

use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};

/// Initial migration creating snippets table
pub struct Migration;

impl Migration {
	/// Apply migration - create snippets table
	pub fn up() -> String {
		let snippets_table = Table::create()
			.table(SnippetsTable::Table)
			.if_not_exists()
			.col(
				ColumnDef::new(SnippetsTable::Id)
					.big_integer()
					.not_null()
					.auto_increment()
					.primary_key(),
			)
			.col(ColumnDef::new(SnippetsTable::Title).text().not_null())
			.col(ColumnDef::new(SnippetsTable::Code).text().not_null())
			.col(ColumnDef::new(SnippetsTable::Language).text().not_null())
			.to_owned();

		snippets_table.to_string(PostgresQueryBuilder)
	}

	/// Rollback migration - drop snippets table
	pub fn down() -> String {
		Table::drop()
			.table(SnippetsTable::Table)
			.if_exists()
			.to_owned()
			.to_string(PostgresQueryBuilder)
	}
}

/// Snippets table identifier
#[derive(Iden)]
enum SnippetsTable {
	Table,
	Id,
	Title,
	Code,
	Language,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_migration_up() {
		let sql = Migration::up();
		assert!(sql.contains("CREATE TABLE"));
		assert!(sql.contains("snippets"));
		assert!(sql.contains("title"));
		assert!(sql.contains("code"));
		assert!(sql.contains("language"));
	}

	#[test]
	fn test_migration_down() {
		let sql = Migration::down();
		assert!(sql.contains("DROP TABLE"));
		assert!(sql.contains("snippets"));
	}
}
