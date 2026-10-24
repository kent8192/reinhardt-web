//! SQL Backend implementations
//!
//! This module provides database-specific SQL generation backends for PostgreSQL,
//! MySQL, SQLite, and CockroachDB.

use crate::{
	dcl::{
		AlterRoleStatement, AlterUserStatement, CreateRoleStatement, CreateUserStatement,
		DropRoleStatement, DropUserStatement, GrantRoleStatement, GrantStatement,
		RenameUserStatement, ResetRoleStatement, RevokeRoleStatement, RevokeStatement,
		SetDefaultRoleStatement, SetRoleStatement,
	},
	expr::SimpleExpr,
	query::{
		AlterDatabaseStatement, AlterFunctionStatement, AlterIndexStatement,
		AlterMaterializedViewStatement, AlterProcedureStatement, AlterSchemaStatement,
		AlterSequenceStatement, AlterTableStatement, AlterTypeStatement, AnalyzeStatement,
		CheckTableStatement, CommentStatement, CreateDatabaseStatement, CreateFunctionStatement,
		CreateIndexStatement, CreateMaterializedViewStatement, CreateProcedureStatement,
		CreateSchemaStatement, CreateSequenceStatement, CreateTableStatement,
		CreateTriggerStatement, CreateTypeStatement, CreateViewStatement, DeleteStatement,
		DropDatabaseStatement, DropFunctionStatement, DropIndexStatement,
		DropMaterializedViewStatement, DropProcedureStatement, DropSchemaStatement,
		DropSequenceStatement, DropTableStatement, DropTriggerStatement, DropTypeStatement,
		DropViewStatement, InsertStatement, OptimizeTableStatement,
		RefreshMaterializedViewStatement, ReindexStatement, RepairTableStatement, SelectStatement,
		TruncateTableStatement, UpdateStatement, VacuumStatement,
	},
	types::{BinOper, DynIden},
	value::Values,
};

mod cockroachdb;
mod mysql;
mod postgres;
mod sql_writer;
mod sqlite;

pub use cockroachdb::CockroachDBQueryBuilder;
pub use mysql::MySqlQueryBuilder;
pub use postgres::PostgresQueryBuilder;
pub use sql_writer::SqlWriter;
pub use sqlite::SqliteQueryBuilder;

/// Render a function name as identifiers, never as executable SQL syntax.
fn safe_function_name(name: &DynIden, quote: char) -> String {
	name.to_string()
		.split('.')
		.map(|segment| {
			let mut chars = segment.chars();
			if chars
				.next()
				.is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
				&& chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
			{
				segment.to_owned()
			} else {
				format!(
					"{quote}{}{quote}",
					segment.replace(quote, &format!("{quote}{quote}"))
				)
			}
		})
		.collect::<Vec<_>>()
		.join(".")
}

#[cfg(test)]
mod security_tests {
	use super::safe_function_name;
	use crate::types::IntoIden;

	#[rstest::rstest]
	fn function_name_cannot_escape_identifier() {
		let name = "count); DROP TABLE users; --".into_iden();
		assert_eq!(
			safe_function_name(&name, '"'),
			"\"count); DROP TABLE users; --\""
		);
		assert_eq!(safe_function_name(&"COUNT".into_iden(), '"'), "COUNT");
	}
}

// Precedence levels shared by the MySQL and SQLite writers.
const PORTABLE_NOT_PRECEDENCE: u8 = 3;
const PORTABLE_PREDICATE_PRECEDENCE: u8 = 4;
const PORTABLE_BITWISE_PRECEDENCE: u8 = 5;

// MySQL and SQLite agree on the relative order of OR, AND, NOT, predicates,
// bitwise operators, and arithmetic, but not within the predicate and bitwise
// levels (SQLite binds `<` tighter than `=`; MySQL orders `|`, `&`, and
// shifts). Those levels are non-associative here, so nested operands within
// them are always grouped. Parentheses are omitted only where both dialects
// parse the rendered text as the expression tree. Operators without a portable
// precedence return `None` and always group their compound operands.
fn portable_precedence(op: BinOper) -> Option<u8> {
	match op {
		BinOper::Or => Some(1),
		BinOper::And => Some(2),
		// Logical NOT has precedence 3.
		BinOper::Equal
		| BinOper::NotEqual
		| BinOper::SmallerThan
		| BinOper::SmallerThanOrEqual
		| BinOper::GreaterThan
		| BinOper::GreaterThanOrEqual
		| BinOper::Is
		| BinOper::IsNot
		| BinOper::Like
		| BinOper::NotLike
		| BinOper::ILike
		| BinOper::NotILike
		| BinOper::SimilarTo
		| BinOper::NotSimilarTo
		| BinOper::Matches
		| BinOper::NotMatches
		| BinOper::Between
		| BinOper::NotBetween
		| BinOper::In
		| BinOper::NotIn => Some(PORTABLE_PREDICATE_PRECEDENCE),
		BinOper::BitAnd | BinOper::BitOr | BinOper::LShift | BinOper::RShift => {
			Some(PORTABLE_BITWISE_PRECEDENCE)
		}
		BinOper::Add | BinOper::Sub => Some(6),
		BinOper::Mul | BinOper::Div | BinOper::Mod => Some(7),
		BinOper::PgOperator(_) => None,
	}
}

// Whether an operand renders as an unparenthesized operator expression.
fn is_compound_operand(expr: &SimpleExpr) -> bool {
	match expr {
		SimpleExpr::Binary(..)
		| SimpleExpr::Unary(..)
		| SimpleExpr::LikeWithEscape(..)
		| SimpleExpr::Custom(_)
		| SimpleExpr::CustomWithExpr(..) => true,
		// MySQL and SQLite render enum casts as their inner expression.
		SimpleExpr::AsEnum(_, inner) => is_compound_operand(inner),
		_ => false,
	}
}

// Whether MySQL or SQLite could bind `expr` differently from the expression
// tree when it is rendered as an operand of `parent`.
fn binary_operand_needs_grouping(expr: &SimpleExpr, parent: BinOper, right_operand: bool) -> bool {
	let Some(parent_precedence) = portable_precedence(parent) else {
		return is_compound_operand(expr);
	};
	match expr {
		SimpleExpr::Binary(_, child, _) => {
			let Some(child_precedence) = portable_precedence(*child) else {
				return true;
			};
			let associative_logical = matches!(
				(parent, child),
				(BinOper::And, BinOper::And) | (BinOper::Or, BinOper::Or)
			);
			let non_associative = matches!(
				parent_precedence,
				PORTABLE_PREDICATE_PRECEDENCE | PORTABLE_BITWISE_PRECEDENCE
			);
			child_precedence < parent_precedence
				|| (child_precedence == parent_precedence
					// Equal-precedence right operands retain their grouping
					// even for addition and multiplication: reassociation can
					// change rounding.
					&& (non_associative || (right_operand && !associative_logical)))
		}
		SimpleExpr::LikeWithEscape(..) => parent_precedence >= PORTABLE_PREDICATE_PRECEDENCE,
		SimpleExpr::Unary(..) => parent_precedence > PORTABLE_NOT_PRECEDENCE,
		// Opaque predicates have unknown precedence. Group them at boolean
		// composition boundaries without interpreting their SQL text.
		SimpleExpr::Custom(_) | SimpleExpr::CustomWithExpr(..) => {
			matches!(parent, BinOper::And | BinOper::Or)
		}
		SimpleExpr::AsEnum(_, inner) => binary_operand_needs_grouping(inner, parent, right_operand),
		_ => false,
	}
}

// Write an operand of a MySQL or SQLite binary operator, preserving the
// expression tree's grouping.
fn write_binary_operand(
	writer: &mut SqlWriter,
	expr: &SimpleExpr,
	parent: BinOper,
	right_operand: bool,
	line_comment_markers: &[&str],
	write_expr: impl FnOnce(&mut SqlWriter, &SimpleExpr),
) {
	let parenthesized = binary_operand_needs_grouping(expr, parent, right_operand);
	write_grouped_operand(
		writer,
		expr,
		parenthesized,
		line_comment_markers,
		write_expr,
	);
}

// Write the operand of a MySQL or SQLite `NOT`. Compound operands are always
// grouped: MySQL's HIGH_NOT_PRECEDENCE mode binds NOT tighter than comparisons.
fn write_not_operand(
	writer: &mut SqlWriter,
	expr: &SimpleExpr,
	line_comment_markers: &[&str],
	write_expr: impl FnOnce(&mut SqlWriter, &SimpleExpr),
) {
	let parenthesized = is_compound_operand(expr) && !matches!(expr, SimpleExpr::Unary(..));
	write_grouped_operand(
		writer,
		expr,
		parenthesized,
		line_comment_markers,
		write_expr,
	);
}

fn write_grouped_operand(
	writer: &mut SqlWriter,
	expr: &SimpleExpr,
	parenthesized: bool,
	line_comment_markers: &[&str],
	write_expr: impl FnOnce(&mut SqlWriter, &SimpleExpr),
) {
	if parenthesized {
		writer.push("(");
	}
	let expression_start = writer.len();
	write_expr(writer, expr);
	if parenthesized {
		// Inspect the whole operand to include nested custom expressions, and
		// conservatively terminate possible line comments without parsing SQL.
		let operand_sql = &writer.sql()[expression_start..];
		if line_comment_markers
			.iter()
			.any(|marker| operand_sql.contains(*marker))
		{
			writer.push("\n");
		}
		writer.push(")");
	}
}

/// Query builder trait for generating SQL from query statements
///
/// This trait defines the interface for database-specific query builders
/// that generate SQL syntax for different backends.
///
/// # Implementations
///
/// - [`PostgresQueryBuilder`] - PostgreSQL backend
/// - [`MySqlQueryBuilder`] - MySQL backend
/// - [`SqliteQueryBuilder`] - SQLite backend
/// - [`CockroachDBQueryBuilder`] - CockroachDB backend
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::backend::{QueryBuilder, PostgresQueryBuilder};
/// use reinhardt_query::prelude::*;
///
/// let builder = PostgresQueryBuilder::new();
/// let stmt = Query::select()
///     .column("id")
///     .column("name")
///     .from("users")
///     .and_where(Expr::col("active").eq(true));
///
/// let (sql, values) = builder.build_select(&stmt);
/// // sql: SELECT "id", "name" FROM "users" WHERE "active" = $1
/// // values: [Value::Bool(true)]
/// ```
pub trait QueryBuilder {
	/// Build SELECT statement
	///
	/// Generates SQL and parameter values for a SELECT statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The SELECT statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_select(&self, stmt: &SelectStatement) -> (String, Values);

	/// Build INSERT statement
	///
	/// Generates SQL and parameter values for an INSERT statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The INSERT statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_insert(&self, stmt: &InsertStatement) -> (String, Values);

	/// Build UPDATE statement
	///
	/// Generates SQL and parameter values for an UPDATE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The UPDATE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_update(&self, stmt: &UpdateStatement) -> (String, Values);

	/// Build DELETE statement
	///
	/// Generates SQL and parameter values for a DELETE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DELETE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_delete(&self, stmt: &DeleteStatement) -> (String, Values);

	/// Build GRANT statement
	///
	/// Generates SQL and parameter values for a GRANT statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The GRANT statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_grant(&self, stmt: &GrantStatement) -> (String, Values);

	/// Build REVOKE statement
	///
	/// Generates SQL and parameter values for a REVOKE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The REVOKE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_revoke(&self, stmt: &RevokeStatement) -> (String, Values);

	/// Build GRANT role membership statement
	///
	/// Generates SQL and parameter values for a GRANT role membership statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The GRANT role statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_grant_role(&self, stmt: &GrantRoleStatement) -> (String, Values);

	/// Build REVOKE role membership statement
	///
	/// Generates SQL and parameter values for a REVOKE role membership statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The REVOKE role statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_revoke_role(&self, stmt: &RevokeRoleStatement) -> (String, Values);

	/// Build CREATE ROLE statement
	///
	/// Generates SQL and parameter values for a CREATE ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_role(&self, stmt: &CreateRoleStatement) -> (String, Values);

	/// Build DROP ROLE statement
	///
	/// Generates SQL and parameter values for a DROP ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_role(&self, stmt: &DropRoleStatement) -> (String, Values);

	/// Build ALTER ROLE statement
	///
	/// Generates SQL and parameter values for an ALTER ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_role(&self, stmt: &AlterRoleStatement) -> (String, Values);

	/// Build CREATE USER statement
	///
	/// Generates SQL and parameter values for a CREATE USER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_user(&self, stmt: &CreateUserStatement) -> (String, Values);

	/// Build DROP USER statement
	///
	/// Generates SQL and parameter values for a DROP USER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_user(&self, stmt: &DropUserStatement) -> (String, Values);

	/// Build ALTER USER statement
	///
	/// Generates SQL and parameter values for an ALTER USER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_user(&self, stmt: &AlterUserStatement) -> (String, Values);

	/// Build RENAME USER statement
	///
	/// Generates SQL and parameter values for a RENAME USER statement.
	/// This is MySQL-only; PostgreSQL and SQLite will panic.
	///
	/// # Arguments
	///
	/// * `stmt` - The RENAME USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_rename_user(&self, stmt: &RenameUserStatement) -> (String, Values);

	/// Build SET ROLE statement
	///
	/// Generates SQL and parameter values for a SET ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The SET ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_set_role(&self, stmt: &SetRoleStatement) -> (String, Values);

	/// Build RESET ROLE statement
	///
	/// Generates SQL and parameter values for a RESET ROLE statement.
	/// This is PostgreSQL-only; MySQL and SQLite will panic.
	///
	/// # Arguments
	///
	/// * `stmt` - The RESET ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_reset_role(&self, stmt: &ResetRoleStatement) -> (String, Values);

	/// Build SET DEFAULT ROLE statement
	///
	/// Generates SQL and parameter values for a SET DEFAULT ROLE statement.
	/// This is MySQL-only; PostgreSQL and SQLite will panic.
	///
	/// # Arguments
	///
	/// * `stmt` - The SET DEFAULT ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_set_default_role(&self, stmt: &SetDefaultRoleStatement) -> (String, Values);

	/// Escape an identifier (table name, column name, etc.)
	///
	/// # Arguments
	///
	/// * `ident` - The identifier to escape
	///
	/// # Returns
	///
	/// The escaped identifier string
	///
	/// # Examples
	///
	/// - PostgreSQL: `escape_identifier("user")` -> `"user"`
	/// - MySQL: `escape_identifier("user")` -> `` `user` ``
	/// - SQLite: `escape_identifier("user")` -> `"user"`
	fn escape_identifier(&self, ident: &str) -> String;

	/// Format a value for SQL
	///
	/// # Arguments
	///
	/// * `value` - The value to format
	/// * `index` - The parameter index (1-based)
	///
	/// # Returns
	///
	/// The formatted placeholder string
	///
	/// # Examples
	///
	/// - PostgreSQL: `format_placeholder(1)` -> `$1`
	/// - MySQL: `format_placeholder(1)` -> `?`
	/// - SQLite: `format_placeholder(1)` -> `?`
	fn format_placeholder(&self, index: usize) -> String;

	/// Build CREATE TABLE statement
	///
	/// Generates SQL and parameter values for a CREATE TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_table(&self, stmt: &CreateTableStatement) -> (String, Values);

	/// Build ALTER TABLE statement
	///
	/// Generates SQL and parameter values for an ALTER TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_table(&self, stmt: &AlterTableStatement) -> (String, Values);

	/// Build DROP TABLE statement
	///
	/// Generates SQL and parameter values for a DROP TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_table(&self, stmt: &DropTableStatement) -> (String, Values);

	/// Build CREATE INDEX statement
	///
	/// Generates SQL and parameter values for a CREATE INDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE INDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_index(&self, stmt: &CreateIndexStatement) -> (String, Values);

	/// Build DROP INDEX statement
	///
	/// Generates SQL and parameter values for a DROP INDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP INDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_index(&self, stmt: &DropIndexStatement) -> (String, Values);

	/// Build CREATE VIEW statement
	///
	/// Generates SQL and parameter values for a CREATE VIEW statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_view(&self, stmt: &CreateViewStatement) -> (String, Values);

	/// Build DROP VIEW statement
	///
	/// Generates SQL and parameter values for a DROP VIEW statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_view(&self, stmt: &DropViewStatement) -> (String, Values);

	/// Build TRUNCATE TABLE statement
	///
	/// Generates SQL and parameter values for a TRUNCATE TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The TRUNCATE TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_truncate_table(&self, stmt: &TruncateTableStatement) -> (String, Values);

	/// Build CREATE TRIGGER statement
	///
	/// Generates SQL and parameter values for a CREATE TRIGGER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE TRIGGER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_trigger(&self, stmt: &CreateTriggerStatement) -> (String, Values);

	/// Build DROP TRIGGER statement
	///
	/// Generates SQL and parameter values for a DROP TRIGGER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP TRIGGER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_trigger(&self, stmt: &DropTriggerStatement) -> (String, Values);

	/// Build ALTER INDEX statement
	///
	/// Generates SQL and parameter values for an ALTER INDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER INDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_index(&self, stmt: &AlterIndexStatement) -> (String, Values);

	/// Build REINDEX statement
	///
	/// Generates SQL and parameter values for a REINDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The REINDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_reindex(&self, stmt: &ReindexStatement) -> (String, Values);

	/// Build CREATE SCHEMA statement
	///
	/// Generates SQL and parameter values for a CREATE SCHEMA statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE SCHEMA statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_schema(&self, stmt: &CreateSchemaStatement) -> (String, Values);

	/// Build ALTER SCHEMA statement
	///
	/// Generates SQL and parameter values for an ALTER SCHEMA statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER SCHEMA statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_schema(&self, stmt: &AlterSchemaStatement) -> (String, Values);

	/// Build DROP SCHEMA statement
	///
	/// Generates SQL and parameter values for a DROP SCHEMA statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP SCHEMA statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_schema(&self, stmt: &DropSchemaStatement) -> (String, Values);

	/// Build CREATE SEQUENCE statement
	///
	/// Generates SQL and parameter values for a CREATE SEQUENCE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE SEQUENCE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_sequence(&self, stmt: &CreateSequenceStatement) -> (String, Values);

	/// Build ALTER SEQUENCE statement
	///
	/// Generates SQL and parameter values for an ALTER SEQUENCE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER SEQUENCE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_sequence(&self, stmt: &AlterSequenceStatement) -> (String, Values);

	/// Build DROP SEQUENCE statement
	///
	/// Generates SQL and parameter values for a DROP SEQUENCE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP SEQUENCE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_sequence(&self, stmt: &DropSequenceStatement) -> (String, Values);

	/// Build COMMENT ON statement
	///
	/// Generates SQL and parameter values for a COMMENT ON statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The COMMENT ON statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_comment(&self, stmt: &CommentStatement) -> (String, Values);

	/// Build CREATE DATABASE statement
	///
	/// Generates SQL and parameter values for a CREATE DATABASE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE DATABASE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_database(&self, stmt: &CreateDatabaseStatement) -> (String, Values);

	/// Build ALTER DATABASE statement
	///
	/// Generates SQL and parameter values for an ALTER DATABASE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER DATABASE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_database(&self, stmt: &AlterDatabaseStatement) -> (String, Values);

	/// Build DROP DATABASE statement
	///
	/// Generates SQL and parameter values for a DROP DATABASE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP DATABASE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_database(&self, stmt: &DropDatabaseStatement) -> (String, Values);

	/// Build OPTIMIZE TABLE statement
	///
	/// Generates SQL and parameter values for an OPTIMIZE TABLE statement.
	/// **MySQL-only**: Other backends will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The OPTIMIZE TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_optimize_table(&self, stmt: &OptimizeTableStatement) -> (String, Values);

	/// Build REPAIR TABLE statement
	///
	/// Generates SQL and parameter values for a REPAIR TABLE statement.
	/// **MySQL-only**: Other backends will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The REPAIR TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_repair_table(&self, stmt: &RepairTableStatement) -> (String, Values);

	/// Build CHECK TABLE statement
	///
	/// Generates SQL and parameter values for a CHECK TABLE statement.
	/// **MySQL-only**: Other backends will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The CHECK TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_check_table(&self, stmt: &CheckTableStatement) -> (String, Values);

	/// Build CREATE FUNCTION statement
	///
	/// Generates SQL and parameter values for a CREATE FUNCTION statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE FUNCTION statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_function(&self, stmt: &CreateFunctionStatement) -> (String, Values);

	/// Build ALTER FUNCTION statement
	///
	/// Generates SQL and parameter values for an ALTER FUNCTION statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER FUNCTION statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_function(&self, stmt: &AlterFunctionStatement) -> (String, Values);

	/// Build DROP FUNCTION statement
	///
	/// Generates SQL and parameter values for a DROP FUNCTION statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP FUNCTION statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_function(&self, stmt: &DropFunctionStatement) -> (String, Values);

	/// Build CREATE PROCEDURE statement
	///
	/// Generates SQL and parameter values for a CREATE PROCEDURE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE PROCEDURE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_procedure(&self, stmt: &CreateProcedureStatement) -> (String, Values);

	/// Build ALTER PROCEDURE statement
	///
	/// Generates SQL and parameter values for an ALTER PROCEDURE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER PROCEDURE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_procedure(&self, stmt: &AlterProcedureStatement) -> (String, Values);

	/// Build DROP PROCEDURE statement
	///
	/// Generates SQL and parameter values for a DROP PROCEDURE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP PROCEDURE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_procedure(&self, stmt: &DropProcedureStatement) -> (String, Values);

	/// Build CREATE TYPE statement
	///
	/// Generates SQL and parameter values for a CREATE TYPE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE TYPE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_type(&self, stmt: &CreateTypeStatement) -> (String, Values);

	/// Build ALTER TYPE statement
	///
	/// Generates SQL and parameter values for an ALTER TYPE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER TYPE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_type(&self, stmt: &AlterTypeStatement) -> (String, Values);

	/// Build DROP TYPE statement
	///
	/// Generates SQL and parameter values for a DROP TYPE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP TYPE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_type(&self, stmt: &DropTypeStatement) -> (String, Values);

	/// Build VACUUM statement
	///
	/// Generates SQL and parameter values for a VACUUM statement.
	/// **PostgreSQL, SQLite, CockroachDB only**: MySQL will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The VACUUM statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_vacuum(&self, stmt: &VacuumStatement) -> (String, Values);

	/// Build ANALYZE statement
	///
	/// Generates SQL and parameter values for an ANALYZE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ANALYZE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_analyze(&self, stmt: &AnalyzeStatement) -> (String, Values);

	/// Build CREATE MATERIALIZED VIEW statement
	///
	/// Generates SQL and parameter values for a CREATE MATERIALIZED VIEW statement.
	/// **PostgreSQL, CockroachDB only**: MySQL and SQLite will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE MATERIALIZED VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_materialized_view(
		&self,
		stmt: &CreateMaterializedViewStatement,
	) -> (String, Values);

	/// Build ALTER MATERIALIZED VIEW statement
	///
	/// Generates SQL and parameter values for an ALTER MATERIALIZED VIEW statement.
	/// **PostgreSQL, CockroachDB only**: MySQL and SQLite will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER MATERIALIZED VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_materialized_view(
		&self,
		stmt: &AlterMaterializedViewStatement,
	) -> (String, Values);

	/// Build DROP MATERIALIZED VIEW statement
	///
	/// Generates SQL and parameter values for a DROP MATERIALIZED VIEW statement.
	/// **PostgreSQL, CockroachDB only**: MySQL and SQLite will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP MATERIALIZED VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_materialized_view(
		&self,
		stmt: &DropMaterializedViewStatement,
	) -> (String, Values);

	/// Build REFRESH MATERIALIZED VIEW statement
	///
	/// Generates SQL and parameter values for a REFRESH MATERIALIZED VIEW statement.
	/// **PostgreSQL, CockroachDB only**: MySQL and SQLite will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The REFRESH MATERIALIZED VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_refresh_materialized_view(
		&self,
		stmt: &RefreshMaterializedViewStatement,
	) -> (String, Values);
}
