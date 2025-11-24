//! Initial migration for snippets app
//!
//! Creates the snippets table with the following schema:
//! - id: BIGSERIAL PRIMARY KEY
//! - title: TEXT NOT NULL
//! - code: TEXT NOT NULL
//! - language: TEXT NOT NULL

use reinhardt::db::migrations::Migration;
use reinhardt::Result;
use sea_query::{ColumnDef, PostgresQueryBuilder, Table};

pub struct CreateSnippetsTable;

impl Migration for CreateSnippetsTable {
	fn name(&self) -> &str {
		"0001_initial"
	}

	fn up(&self) -> Result<String> {
		let sql = Table::create()
			.table("snippets")
			.if_not_exists()
			.col(
				ColumnDef::new("id")
					.big_integer()
					.not_null()
					.auto_increment()
					.primary_key(),
			)
			.col(ColumnDef::new("title").text().not_null())
			.col(ColumnDef::new("code").text().not_null())
			.col(ColumnDef::new("language").text().not_null())
			.build(PostgresQueryBuilder);

		Ok(sql)
	}

	fn down(&self) -> Result<String> {
		let sql = Table::drop()
			.table("snippets")
			.if_exists()
			.build(PostgresQueryBuilder);

		Ok(sql)
	}
}
