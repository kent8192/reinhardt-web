//! CockroachDB query builder backend
//!
//! This module implements the SQL generation backend for CockroachDB.
//! Since CockroachDB is PostgreSQL-compatible, most operations delegate to PostgreSQL.

use super::{PostgresQueryBuilder, QueryBuilder};
use crate::{
	query::{
		AlterDatabaseStatement, AlterIndexStatement, AlterSchemaStatement, AlterSequenceStatement,
		AlterTableStatement, CheckTableStatement, CommentStatement, CreateDatabaseStatement,
		CreateIndexStatement, CreateSchemaStatement, CreateSequenceStatement, CreateTableStatement,
		CreateTriggerStatement, CreateViewStatement, DeleteStatement, DropDatabaseStatement,
		DropIndexStatement, DropSchemaStatement, DropSequenceStatement, DropTableStatement,
		DropTriggerStatement, DropViewStatement, InsertStatement, OptimizeTableStatement,
		ReindexStatement, RepairTableStatement, SelectStatement, TruncateTableStatement,
		UpdateStatement,
	},
	value::Values,
};

/// CockroachDB query builder
///
/// This struct implements SQL generation for CockroachDB. Since CockroachDB is PostgreSQL-compatible,
/// most operations delegate to [`PostgresQueryBuilder`].
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::backend::{CockroachDBQueryBuilder, QueryBuilder};
/// use reinhardt_query::prelude::*;
///
/// let builder = CockroachDBQueryBuilder::new();
/// let stmt = Query::select()
///     .column("id")
///     .from("users");
///
/// let (sql, values) = builder.build_select(&stmt);
/// // sql: SELECT "id" FROM "users"
/// ```
#[derive(Debug, Clone, Default)]
pub struct CockroachDBQueryBuilder {
	postgres: PostgresQueryBuilder,
}

impl CockroachDBQueryBuilder {
	/// Create a new CockroachDB query builder
	pub fn new() -> Self {
		Self {
			postgres: PostgresQueryBuilder::new(),
		}
	}
}

impl QueryBuilder for CockroachDBQueryBuilder {
	fn escape_identifier(&self, ident: &str) -> String {
		self.postgres.escape_identifier(ident)
	}

	fn format_placeholder(&self, index: usize) -> String {
		self.postgres.format_placeholder(index)
	}

	fn build_select(&self, stmt: &SelectStatement) -> (String, Values) {
		self.postgres.build_select(stmt)
	}

	fn build_insert(&self, stmt: &InsertStatement) -> (String, Values) {
		self.postgres.build_insert(stmt)
	}

	fn build_update(&self, stmt: &UpdateStatement) -> (String, Values) {
		self.postgres.build_update(stmt)
	}

	fn build_delete(&self, stmt: &DeleteStatement) -> (String, Values) {
		self.postgres.build_delete(stmt)
	}

	fn build_create_table(&self, stmt: &CreateTableStatement) -> (String, Values) {
		self.postgres.build_create_table(stmt)
	}

	fn build_alter_table(&self, stmt: &AlterTableStatement) -> (String, Values) {
		self.postgres.build_alter_table(stmt)
	}

	fn build_drop_table(&self, stmt: &DropTableStatement) -> (String, Values) {
		self.postgres.build_drop_table(stmt)
	}

	fn build_create_index(&self, stmt: &CreateIndexStatement) -> (String, Values) {
		self.postgres.build_create_index(stmt)
	}

	fn build_drop_index(&self, stmt: &DropIndexStatement) -> (String, Values) {
		self.postgres.build_drop_index(stmt)
	}

	fn build_create_view(&self, stmt: &CreateViewStatement) -> (String, Values) {
		self.postgres.build_create_view(stmt)
	}

	fn build_drop_view(&self, stmt: &DropViewStatement) -> (String, Values) {
		self.postgres.build_drop_view(stmt)
	}

	fn build_truncate_table(&self, stmt: &TruncateTableStatement) -> (String, Values) {
		self.postgres.build_truncate_table(stmt)
	}

	fn build_create_trigger(&self, stmt: &CreateTriggerStatement) -> (String, Values) {
		self.postgres.build_create_trigger(stmt)
	}

	fn build_drop_trigger(&self, stmt: &DropTriggerStatement) -> (String, Values) {
		self.postgres.build_drop_trigger(stmt)
	}

	fn build_alter_index(&self, stmt: &AlterIndexStatement) -> (String, Values) {
		self.postgres.build_alter_index(stmt)
	}

	fn build_reindex(&self, stmt: &ReindexStatement) -> (String, Values) {
		self.postgres.build_reindex(stmt)
	}

	fn build_create_schema(&self, _stmt: &CreateSchemaStatement) -> (String, Values) {
		todo!("CockroachDB: CREATE SCHEMA support")
	}

	fn build_alter_schema(&self, _stmt: &AlterSchemaStatement) -> (String, Values) {
		todo!("CockroachDB: ALTER SCHEMA support")
	}

	fn build_drop_schema(&self, _stmt: &DropSchemaStatement) -> (String, Values) {
		todo!("CockroachDB: DROP SCHEMA support")
	}

	fn build_create_sequence(&self, _stmt: &CreateSequenceStatement) -> (String, Values) {
		todo!("CockroachDB: CREATE SEQUENCE support")
	}

	fn build_alter_sequence(&self, _stmt: &AlterSequenceStatement) -> (String, Values) {
		todo!("CockroachDB: ALTER SEQUENCE support")
	}

	fn build_drop_sequence(&self, _stmt: &DropSequenceStatement) -> (String, Values) {
		todo!("CockroachDB: DROP SEQUENCE support")
	}

	fn build_comment(&self, _stmt: &CommentStatement) -> (String, Values) {
		todo!("CockroachDB: COMMENT ON support")
	}

	fn build_create_database(&self, stmt: &CreateDatabaseStatement) -> (String, Values) {
		// CockroachDB supports CREATE DATABASE with multi-region extensions
		// Delegate to PostgreSQL implementation which includes CockroachDB compatibility
		self.postgres.build_create_database(stmt)
	}

	fn build_alter_database(&self, stmt: &AlterDatabaseStatement) -> (String, Values) {
		// CockroachDB supports ALTER DATABASE operations with multi-region support
		// Delegate to PostgreSQL implementation which includes CockroachDB compatibility
		self.postgres.build_alter_database(stmt)
	}

	fn build_drop_database(&self, stmt: &DropDatabaseStatement) -> (String, Values) {
		// CockroachDB supports DROP DATABASE similar to PostgreSQL
		// Delegate to PostgreSQL implementation
		self.postgres.build_drop_database(stmt)
	}

	fn build_optimize_table(&self, _stmt: &OptimizeTableStatement) -> (String, Values) {
		panic!(
			"OPTIMIZE TABLE is MySQL-specific. CockroachDB automatically optimizes tables in the background."
		);
	}

	fn build_repair_table(&self, _stmt: &RepairTableStatement) -> (String, Values) {
		panic!(
			"REPAIR TABLE is not supported in CockroachDB. CockroachDB automatically repairs data through replication and consistency checks."
		);
	}

	fn build_check_table(&self, _stmt: &CheckTableStatement) -> (String, Values) {
		panic!(
			"CHECK TABLE is not supported in CockroachDB. Use SHOW EXPERIMENTAL_RANGES or other system tables to monitor table health."
		);
	}

// 	fn build_create_function(&self, stmt: &CreateFunctionStatement) -> (String, Values) {
// 		// CockroachDB delegates to PostgreSQL for functions
// 		self.postgres.build_create_function(stmt)
// 	}
// 
// 	fn build_alter_function(&self, stmt: &AlterFunctionStatement) -> (String, Values) {
// 		// CockroachDB delegates to PostgreSQL for functions
// 		self.postgres.build_alter_function(stmt)
// 	}
// 
// 	fn build_drop_function(&self, stmt: &DropFunctionStatement) -> (String, Values) {
// 		// CockroachDB delegates to PostgreSQL for functions
// 		self.postgres.build_drop_function(stmt)
// 	}
// 
// 	fn escape_identifier(&self, ident: &str) -> String {
// 		self.postgres.escape_identifier(ident)
// 	}
// 
// 	fn format_placeholder(&self, index: usize) -> String {
// 		self.postgres.format_placeholder(index)
// 	}
// 
// 	fn build_analyze(&self, stmt: &AnalyzeStatement) -> (String, Values) {
// 		// CockroachDB supports ANALYZE similar to PostgreSQL
// 		self.postgres.build_analyze(stmt)
// 	}
// 
// 	fn build_vacuum(&self, _stmt: &VacuumStatement) -> (String, Values) {
// 		// CockroachDB does not support VACUUM (automatic garbage collection)
// 		panic!("CockroachDB does not support VACUUM. Garbage collection is automatic.");
// 	}
// 
// 	fn build_create_materialized_view(
// 		&self,
// 		stmt: &CreateMaterializedViewStatement,
// 	) -> (String, Values) {
// 		// CockroachDB supports materialized views similar to PostgreSQL
// 		self.postgres.build_create_materialized_view(stmt)
// 	}
// 
// 	fn build_alter_materialized_view(
// 		&self,
// 		stmt: &AlterMaterializedViewStatement,
// 	) -> (String, Values) {
// 		// CockroachDB supports materialized views similar to PostgreSQL
// 		self.postgres.build_alter_materialized_view(stmt)
// 	}
// 
// 	fn build_drop_materialized_view(
// 		&self,
// 		stmt: &DropMaterializedViewStatement,
// 	) -> (String, Values) {
// 		// CockroachDB supports materialized views similar to PostgreSQL
// 		self.postgres.build_drop_materialized_view(stmt)
// 	}
// 
// 	fn build_refresh_materialized_view(
// 		&self,
// 		stmt: &RefreshMaterializedViewStatement,
// 	) -> (String, Values) {
// 		// CockroachDB supports materialized views similar to PostgreSQL
// 		self.postgres.build_refresh_materialized_view(stmt)
// 	}
// 
// 	fn build_create_procedure(&self, _stmt: &CreateProcedureStatement) -> (String, Values) {
// 		todo!("CockroachDB: CREATE PROCEDURE support (Phase C)")
// 	}
// 
// 	fn build_alter_procedure(&self, _stmt: &AlterProcedureStatement) -> (String, Values) {
// 		todo!("CockroachDB: ALTER PROCEDURE support (Phase C)")
// 	}
// 
// 	fn build_drop_procedure(&self, _stmt: &DropProcedureStatement) -> (String, Values) {
// 		todo!("CockroachDB: DROP PROCEDURE support (Phase C)")
// 	}
// 
// 	fn build_create_type(&self, stmt: &CreateTypeStatement) -> (String, Values) {
// 		// CockroachDB supports custom types similar to PostgreSQL
// 		self.postgres.build_create_type(stmt)
// 	}
// 
// 	fn build_alter_type(&self, stmt: &AlterTypeStatement) -> (String, Values) {
// 		// CockroachDB supports custom types similar to PostgreSQL
// 		self.postgres.build_alter_type(stmt)
// 	}
// 
// 	fn build_drop_type(&self, stmt: &DropTypeStatement) -> (String, Values) {
// 		// CockroachDB supports custom types similar to PostgreSQL
// 		self.postgres.build_drop_type(stmt)
// 	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::query::Query;

	// FUNCTION tests - verify CockroachDB delegates to PostgreSQL
	#[test]
	fn test_create_function_delegates_to_postgres() {
		use crate::types::function::FunctionLanguage;

		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::create_function();
		stmt.name("my_func")
			.returns("integer")
			.language(FunctionLanguage::Sql)
			.body("SELECT 1");

		let (sql, values) = builder.build_create_function(&stmt);
		// Should generate PostgreSQL-style SQL with double quotes and $$
		assert_eq!(
			sql,
			r#"CREATE FUNCTION "my_func"() RETURNS integer LANGUAGE SQL AS $$SELECT 1$$"#
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_function_delegates_to_postgres() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::alter_function();
		stmt.name("my_func").rename_to("new_func");

		let (sql, values) = builder.build_alter_function(&stmt);
		// Should generate PostgreSQL-style SQL with double quotes
		assert_eq!(sql, r#"ALTER FUNCTION "my_func" RENAME TO "new_func""#);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_drop_function_delegates_to_postgres() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::drop_function();
		stmt.name("my_func").if_exists().cascade();

		let (sql, values) = builder.build_drop_function(&stmt);
		// Should generate PostgreSQL-style SQL with double quotes and CASCADE
		assert_eq!(sql, r#"DROP FUNCTION IF EXISTS "my_func" CASCADE"#);
		assert_eq!(values.len(), 0);
	}

	// TYPE tests - verify CockroachDB delegates to PostgreSQL
	#[test]
	fn test_create_type_enum_delegates_to_postgres() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::create_type();
		stmt.name("mood")
			.as_enum(vec!["happy".to_string(), "sad".to_string()]);

		let (sql, values) = builder.build_create_type(&stmt);
		// Should generate PostgreSQL-style SQL with double quotes
		assert_eq!(sql, r#"CREATE TYPE "mood" AS ENUM ('happy', 'sad')"#);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_type_delegates_to_postgres() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::alter_type();
		stmt.name("mood").rename_to("feeling");

		let (sql, values) = builder.build_alter_type(&stmt);
		// Should generate PostgreSQL-style SQL with double quotes
		assert_eq!(sql, r#"ALTER TYPE "mood" RENAME TO "feeling""#);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_drop_type_delegates_to_postgres() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::drop_type();
		stmt.name("mood").if_exists().cascade();

		let (sql, values) = builder.build_drop_type(&stmt);
		// Should generate PostgreSQL-style SQL with double quotes and CASCADE
		assert_eq!(sql, r#"DROP TYPE IF EXISTS "mood" CASCADE"#);
		assert_eq!(values.len(), 0);
	}

	// MySQL-specific maintenance command panic tests
	#[test]
	#[should_panic(expected = "CockroachDB automatically optimizes tables")]
	fn test_optimize_table_panics() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::optimize_table();
		stmt.table("users");

		let _ = builder.build_optimize_table(&stmt);
	}

	#[test]
	#[should_panic(expected = "not supported in CockroachDB")]
	fn test_repair_table_panics() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::repair_table();
		stmt.table("users");

		let _ = builder.build_repair_table(&stmt);
	}

	#[test]
	#[should_panic(expected = "not supported in CockroachDB")]
	fn test_check_table_panics() {
		let builder = CockroachDBQueryBuilder::new();
		let mut stmt = Query::check_table();
		stmt.table("users");

		let _ = builder.build_check_table(&stmt);
	}
}
