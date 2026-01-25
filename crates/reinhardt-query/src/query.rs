//! Query statement builders
//!
//! This module provides builders for SQL query statements (SELECT, INSERT, UPDATE, DELETE)
//! and DDL statements (CREATE/ALTER/DROP for tables, indexes, views, schemas, sequences, databases).
//!
//! # DML Usage
//!
//! - Query Select: [`SelectStatement`]
//! - Query Insert: [`InsertStatement`]
//! - Query Update: [`UpdateStatement`]
//! - Query Delete: [`DeleteStatement`]
//!
//! # DDL Usage - Table Operations
//!
//! - Create Table: [`CreateTableStatement`]
//! - Alter Table: [`AlterTableStatement`]
//! - Drop Table: [`DropTableStatement`]
//!
//! # DDL Usage - Index Operations
//!
//! - Create Index: [`CreateIndexStatement`]
//! - Alter Index: [`AlterIndexStatement`]
//! - Drop Index: [`DropIndexStatement`]
//! - Reindex: [`ReindexStatement`]
//!
//! # DDL Usage - View Operations
//!
//! - Create View: [`CreateViewStatement`]
//! - Drop View: [`DropViewStatement`]
//!
//! # DDL Usage - Schema Operations
//!
//! - Create Schema: [`CreateSchemaStatement`]
//! - Alter Schema: [`AlterSchemaStatement`]
//! - Drop Schema: [`DropSchemaStatement`]
//!
//! # DDL Usage - Sequence Operations
//!
//! - Create Sequence: [`CreateSequenceStatement`]
//! - Alter Sequence: [`AlterSequenceStatement`]
//! - Drop Sequence: [`DropSequenceStatement`]
//!
//! # DDL Usage - Database Operations
//!
//! - Create Database: [`CreateDatabaseStatement`]
//! - Alter Database: [`AlterDatabaseStatement`]
//! - Drop Database: [`DropDatabaseStatement`]
//!
//! # DDL Usage - Comment Operations
//!
//! - Comment: [`CommentStatement`]
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt_query::prelude::*;
//!
//! // SELECT query
//! let select_query = Query::select()
//!     .column(Expr::col("name"))
//!     .from("users")
//!     .and_where(Expr::col("active").eq(true));
//!
//! // INSERT query
//! let insert_query = Query::insert()
//!     .into_table("users")
//!     .columns(["name", "email"])
//!     .values_panic(["Alice", "alice@example.com"]);
//!
//! // UPDATE query
//! let update_query = Query::update()
//!     .table("users")
//!     .value("active", false)
//!     .and_where(Expr::col("id").eq(1));
//!
//! // DELETE query
//! let delete_query = Query::delete()
//!     .from_table("users")
//!     .and_where(Expr::col("active").eq(false));
//! ```

mod alter_index;
mod alter_table;
mod comment;
mod create_index;
mod create_table;
mod create_trigger;
mod create_view;
mod database;
mod delete;
pub mod event;
mod drop_index;
mod drop_table;
mod drop_trigger;
mod drop_view;
mod insert;
pub mod maintenance;
mod reindex;
mod returning;
pub mod schema;
mod select;
pub mod sequence;
mod traits;
mod truncate_table;
mod update;

pub use alter_index::*;
pub use alter_table::*;
pub use comment::CommentStatement;
pub use create_index::*;
pub use create_table::*;
pub use create_trigger::*;
pub use create_view::*;
pub use database::{AlterDatabaseStatement, CreateDatabaseStatement, DropDatabaseStatement};
pub use delete::*;
pub use event::{AlterEventStatement, CreateEventStatement, DropEventStatement};
pub use drop_index::*;
pub use drop_table::*;
pub use drop_trigger::*;
pub use drop_view::*;
pub use insert::*;
// TODO: Maintenance operations will be implemented in future commits
// pub use maintenance::{
// 	AnalyzeStatement,
// 	CheckTableStatement, OptimizeTableStatement, RepairTableStatement,
// 	VacuumStatement,
// };
pub use reindex::*;
pub use returning::*;
pub use schema::{AlterSchemaOperation, AlterSchemaStatement, CreateSchemaStatement, DropSchemaStatement};
pub use select::{
	CommonTableExpr, LockBehavior, LockClause, LockType, SelectDistinct, SelectExpr,
	SelectStatement, UnionType,
};
pub use sequence::{AlterSequenceStatement, CreateSequenceStatement, DropSequenceStatement};
pub use traits::*;
pub use truncate_table::*;
pub use update::*;

/// Shorthand for constructing any table query
///
/// This struct provides static constructor methods for creating query statements.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::prelude::*;
///
/// // Create a SELECT statement
/// let select = Query::select();
///
/// // Create an INSERT statement
/// let insert = Query::insert();
///
/// // Create an UPDATE statement
/// let update = Query::update();
///
/// // Create a DELETE statement
/// let delete = Query::delete();
/// ```
#[derive(Debug, Clone)]
pub struct Query;

impl Query {
	/// Construct a new [`SelectStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::select()
	///     .column(Expr::col("id"))
	///     .column(Expr::col("name"))
	///     .from("users");
	/// ```
	pub fn select() -> SelectStatement {
		SelectStatement::new()
	}

	/// Construct a new [`InsertStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::insert()
	///     .into_table("users")
	///     .columns(["name", "email"])
	///     .values_panic(["Alice", "alice@example.com"]);
	/// ```
	pub fn insert() -> InsertStatement {
		InsertStatement::new()
	}

	/// Construct a new [`UpdateStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::update()
	///     .table("users")
	///     .value("last_updated", Expr::current_timestamp());
	/// ```
	pub fn update() -> UpdateStatement {
		UpdateStatement::new()
	}

	/// Construct a new [`DeleteStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::delete()
	///     .from_table("users")
	///     .and_where(Expr::col("deleted_at").is_not_null());
	/// ```
	pub fn delete() -> DeleteStatement {
		DeleteStatement::new()
	}

	/// Construct a new [`CreateTableStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::types::ddl::{ColumnDef, ColumnType};
	///
	/// let query = Query::create_table()
	///     .table("users")
	///     .if_not_exists()
	///     .col(
	///         ColumnDef::new("id")
	///             .column_type(ColumnType::Integer)
	///             .primary_key(true)
	///     );
	/// ```
	pub fn create_table() -> CreateTableStatement {
		CreateTableStatement::new()
	}

	/// Construct a new [`AlterTableStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::types::ddl::{ColumnDef, ColumnType};
	///
	/// let query = Query::alter_table()
	///     .table("users")
	///     .add_column(
	///         ColumnDef::new("age")
	///             .column_type(ColumnType::Integer)
	///     );
	/// ```
	pub fn alter_table() -> AlterTableStatement {
		AlterTableStatement::new()
	}

	/// Construct a new [`DropTableStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::drop_table()
	///     .table("users")
	///     .if_exists();
	/// ```
	pub fn drop_table() -> DropTableStatement {
		DropTableStatement::new()
	}

	/// Construct a new [`CreateIndexStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::create_index()
	///     .name("idx_email")
	///     .table("users")
	///     .col("email")
	///     .unique();
	/// ```
	pub fn create_index() -> CreateIndexStatement {
		CreateIndexStatement::new()
	}

	/// Construct a new [`DropIndexStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::drop_index()
	///     .name("idx_email")
	///     .table("users")
	///     .if_exists();
	/// ```
	pub fn drop_index() -> DropIndexStatement {
		DropIndexStatement::new()
	}

	/// Construct a new [`CreateViewStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let select = Query::select()
	///     .column(Expr::col("name"))
	///     .from("users");
	///
	/// let query = Query::create_view()
	///     .name("user_names")
	///     .as_select(select)
	///     .if_not_exists();
	/// ```
	pub fn create_view() -> CreateViewStatement {
		CreateViewStatement::new()
	}

	/// Construct a new [`DropViewStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::drop_view()
	///     .name("user_names")
	///     .if_exists();
	/// ```
	pub fn drop_view() -> DropViewStatement {
		DropViewStatement::new()
	}

	/// Construct a new [`TruncateTableStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::truncate_table()
	///     .table("users");
	/// ```
	pub fn truncate_table() -> TruncateTableStatement {
		TruncateTableStatement::new()
	}

	/// Construct a new [`CreateTriggerStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::types::{TriggerTiming, TriggerEvent, TriggerScope};
	///
	/// let query = Query::create_trigger()
	///     .name("audit_insert")
	///     .timing(TriggerTiming::After)
	///     .event(TriggerEvent::Insert)
	///     .on_table("users")
	///     .for_each(TriggerScope::Row)
	///     .execute_function("audit_log_insert");
	/// ```
	pub fn create_trigger() -> CreateTriggerStatement {
		CreateTriggerStatement::new()
	}

	/// Construct a new [`DropTriggerStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::drop_trigger()
	///     .name("audit_insert")
	///     .on_table("users")
	///     .if_exists();
	/// ```
	pub fn drop_trigger() -> DropTriggerStatement {
		DropTriggerStatement::new()
	}

	/// Construct a new [`AlterIndexStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // PostgreSQL: Rename index
	/// let query = Query::alter_index()
	///     .name("idx_users_email")
	///     .rename_to("idx_users_email_new");
	///
	/// // MySQL: Rename index (requires table name)
	/// let query = Query::alter_index()
	///     .table("users")
	///     .name("idx_email")
	///     .rename_to("idx_email_new");
	/// ```
	pub fn alter_index() -> AlterIndexStatement {
		AlterIndexStatement::new()
	}

	/// Construct a new [`ReindexStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // Reindex a specific index
	/// let query = Query::reindex()
	///     .index("idx_users_email");
	///
	/// // Reindex all indexes in a table
	/// let query = Query::reindex()
	///     .table("users");
	/// ```
	pub fn reindex() -> ReindexStatement {
		ReindexStatement::new()
	}

	/// Construct a new [`CreateSchemaStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // CREATE SCHEMA my_schema
	/// let query = Query::create_schema()
	///     .name("my_schema");
	///
	/// // CREATE SCHEMA IF NOT EXISTS my_schema AUTHORIZATION owner_user
	/// let query = Query::create_schema()
	///     .name("my_schema")
	///     .if_not_exists()
	///     .authorization("owner_user");
	/// ```
	pub fn create_schema() -> CreateSchemaStatement {
		CreateSchemaStatement::new()
	}

	/// Construct a new [`AlterSchemaStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // ALTER SCHEMA old_name RENAME TO new_name
	/// let query = Query::alter_schema()
	///     .name("old_name")
	///     .rename_to("new_name");
	///
	/// // ALTER SCHEMA my_schema OWNER TO new_owner
	/// let query = Query::alter_schema()
	///     .name("my_schema")
	///     .owner_to("new_owner");
	/// ```
	pub fn alter_schema() -> AlterSchemaStatement {
		AlterSchemaStatement::new()
	}

	/// Construct a new [`DropSchemaStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // DROP SCHEMA my_schema
	/// let query = Query::drop_schema()
	///     .name("my_schema");
	///
	/// // DROP SCHEMA IF EXISTS my_schema CASCADE
	/// let query = Query::drop_schema()
	///     .name("my_schema")
	///     .if_exists()
	///     .cascade();
	/// ```
	pub fn drop_schema() -> DropSchemaStatement {
		DropSchemaStatement::new()
	}

	/// Construct a new [`CreateSequenceStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // CREATE SEQUENCE user_id_seq
	/// let query = Query::create_sequence()
	///     .name("user_id_seq");
	///
	/// // CREATE SEQUENCE user_id_seq START WITH 1000 INCREMENT BY 1
	/// let query = Query::create_sequence()
	///     .name("user_id_seq")
	///     .start_with(1000)
	///     .increment_by(1);
	/// ```
	pub fn create_sequence() -> CreateSequenceStatement {
		CreateSequenceStatement::new()
	}

	/// Construct a new [`AlterSequenceStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // ALTER SEQUENCE user_id_seq RESTART WITH 2000
	/// let query = Query::alter_sequence()
	///     .name("user_id_seq")
	///     .restart_with(2000);
	///
	/// // ALTER SEQUENCE user_id_seq RENAME TO new_user_id_seq
	/// let query = Query::alter_sequence()
	///     .name("user_id_seq")
	///     .rename_to("new_user_id_seq");
	/// ```
	pub fn alter_sequence() -> AlterSequenceStatement {
		AlterSequenceStatement::new()
	}

	/// Construct a new [`DropSequenceStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // DROP SEQUENCE user_id_seq
	/// let query = Query::drop_sequence()
	///     .name("user_id_seq");
	///
	/// // DROP SEQUENCE IF EXISTS user_id_seq CASCADE
	/// let query = Query::drop_sequence()
	///     .name("user_id_seq")
	///     .if_exists()
	///     .cascade();
	/// ```
	pub fn drop_sequence() -> DropSequenceStatement {
		DropSequenceStatement::new()
	}

	/// Construct a new [`CommentStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::types::CommentTarget;
	///
	/// // COMMENT ON TABLE "users" IS 'User account information'
	/// let query = Query::comment()
	///     .target(CommentTarget::Table("users".into_iden()))
	///     .comment("User account information");
	///
	/// // COMMENT ON COLUMN "users"."email" IS 'User email address'
	/// let query = Query::comment()
	///     .target(CommentTarget::Column("users".into_iden(), "email".into_iden()))
	///     .comment("User email address");
	/// ```
	pub fn comment() -> CommentStatement {
		CommentStatement::new()
	}

	/// Construct a new [`AlterDatabaseStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // ALTER DATABASE mydb RENAME TO newdb
	/// let query = Query::alter_database()
	///     .name("mydb")
	///     .rename_to("newdb");
	///
	/// // CockroachDB: ALTER DATABASE mydb ADD REGION 'us-west-1'
	/// let query = Query::alter_database()
	///     .name("mydb")
	///     .add_region("us-west-1");
	/// ```
	pub fn create_database() -> CreateDatabaseStatement {
		CreateDatabaseStatement::new()
	}

	/// Construct a new [`AlterDatabaseStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // ALTER DATABASE mydb RENAME TO newdb
	/// let query = Query::alter_database()
	///     .name("mydb")
	///     .rename_to("newdb");
	///
	/// // CockroachDB: ALTER DATABASE mydb ADD REGION 'us-west-1'
	/// let query = Query::alter_database()
	///     .name("mydb")
	///     .add_region("us-west-1");
	/// ```
	pub fn alter_database() -> AlterDatabaseStatement {
		AlterDatabaseStatement::new()
	}

	/// Construct a new [`DropDatabaseStatement`]
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // DROP DATABASE mydb
	/// let query = Query::drop_database()
	///     .name("mydb");
	/// ```
	pub fn drop_database() -> DropDatabaseStatement {
		DropDatabaseStatement::new()
	}

	// TODO: Maintenance operations will be implemented in future commits
	// /// Construct a new [`VacuumStatement`]
	// ///
	// /// # Examples
	// ///
	// /// ```rust,ignore
	// /// use reinhardt_query::prelude::*;
	// ///
	// /// // VACUUM users
	// /// let query = Query::vacuum()
	// ///     .table("users");
	// ///
	// /// // VACUUM FULL ANALYZE users
	// /// let query = Query::vacuum()
	// ///     .table("users")
	// ///     .full()
	// ///     .analyze();
	// /// ```
	// pub fn vacuum() -> VacuumStatement {
	// 	VacuumStatement::new()
	// }

	// /// Construct a new [`AnalyzeStatement`]
	// ///
	// /// # Examples
	// ///
	// /// ```rust,ignore
	// /// use reinhardt_query::prelude::*;
	// ///
	// /// // ANALYZE users
	// /// let query = Query::analyze()
	// ///     .table("users");
	// ///
	// /// // ANALYZE VERBOSE users (email, name)
	// /// let query = Query::analyze()
	// ///     .table_columns("users", ["email", "name"])
	// ///     .verbose();
	// /// ```
	// pub fn analyze() -> AnalyzeStatement {
	// 	AnalyzeStatement::new()
	// }

	/// Construct a new [`OptimizeTableStatement`]
	///
	/// **MySQL-only feature**: This statement is specific to MySQL.
	/// Other backends will panic with a helpful message.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // OPTIMIZE TABLE users
	/// let query = Query::optimize_table()
	///     .table("users");
	///
	/// // OPTIMIZE TABLE users, posts
	/// let query = Query::optimize_table()
	///     .table("users")
	///     .table("posts");
	/// ```
	pub fn optimize_table() -> OptimizeTableStatement {
		OptimizeTableStatement::new()
	}

	/// Construct a new [`RepairTableStatement`]
	///
	/// **MySQL-only feature**: This statement is specific to MySQL.
	/// Other backends will panic with a helpful message.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // REPAIR TABLE users
	/// let query = Query::repair_table()
	///     .table("users");
	///
	/// // REPAIR TABLE QUICK users
	/// let query = Query::repair_table()
	///     .table("users")
	///     .quick();
	/// ```
	pub fn repair_table() -> RepairTableStatement {
		RepairTableStatement::new()
	}

	/// Construct a new [`CheckTableStatement`]
	///
	/// **MySQL-only feature**: This statement is specific to MySQL.
	/// Other backends will panic with a helpful message.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::types::CheckTableOption;
	///
	/// // CHECK TABLE users
	/// let query = Query::check_table()
	///     .table("users");
	///
	/// // CHECK TABLE users QUICK
	/// let query = Query::check_table()
	///     .table("users")
	///     .option(CheckTableOption::Quick);
	/// ```
	pub fn check_table() -> CheckTableStatement {
		CheckTableStatement::new()
	}

	/// Construct a new [`CreateEventStatement`]
	///
	/// **MySQL-only feature**: This statement is specific to MySQL.
	/// Other backends will panic with a helpful message.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // CREATE EVENT cleanup_logs
	/// // ON SCHEDULE EVERY 1 DAY
	/// // DO DELETE FROM logs WHERE created_at < NOW() - INTERVAL 7 DAY
	/// let query = Query::create_event()
	///     .name("cleanup_logs")
	///     .on_schedule_every("1 DAY")
	///     .do_body("DELETE FROM logs WHERE created_at < NOW() - INTERVAL 7 DAY");
	/// ```
	pub fn create_event() -> CreateEventStatement {
		CreateEventStatement::new()
	}

	/// Construct a new [`AlterEventStatement`]
	///
	/// **MySQL-only feature**: This statement is specific to MySQL.
	/// Other backends will panic with a helpful message.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // ALTER EVENT cleanup_logs RENAME TO purge_old_logs
	/// let query = Query::alter_event()
	///     .name("cleanup_logs")
	///     .rename_to("purge_old_logs");
	/// ```
	pub fn alter_event() -> AlterEventStatement {
		AlterEventStatement::new()
	}

	/// Construct a new [`DropEventStatement`]
	///
	/// **MySQL-only feature**: This statement is specific to MySQL.
	/// Other backends will panic with a helpful message.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// // DROP EVENT cleanup_logs
	/// let query = Query::drop_event()
	///     .name("cleanup_logs");
	///
	/// // DROP EVENT IF EXISTS cleanup_logs
	/// let query = Query::drop_event()
	///     .name("cleanup_logs")
	///     .if_exists();
	/// ```
	pub fn drop_event() -> DropEventStatement {
		DropEventStatement::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_query_select_constructor() {
		let _query = Query::select();
	}

	#[test]
	fn test_query_insert_constructor() {
		let _query = Query::insert();
	}

	#[test]
	fn test_query_update_constructor() {
		let _query = Query::update();
	}

	#[test]
	fn test_query_delete_constructor() {
		let _query = Query::delete();
	}
}
