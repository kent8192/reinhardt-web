//! Query statement builders
//!
//! This module provides builders for SQL query statements (SELECT, INSERT, UPDATE, DELETE)
//! and DDL statements (CREATE TABLE, ALTER TABLE, DROP TABLE).
//!
//! # DML Usage
//!
//! - Query Select: [`SelectStatement`]
//! - Query Insert: [`InsertStatement`]
//! - Query Update: [`UpdateStatement`]
//! - Query Delete: [`DeleteStatement`]
//!
//! # DDL Usage
//!
//! - Create Table: [`CreateTableStatement`]
//! - Alter Table: [`AlterTableStatement`]
//! - Drop Table: [`DropTableStatement`]
//! - Create Index: [`CreateIndexStatement`]
//! - Drop Index: [`DropIndexStatement`]
//! - Create View: [`CreateViewStatement`]
//! - Drop View: [`DropViewStatement`]
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

mod alter_table;
mod create_index;
mod create_table;
mod create_view;
mod delete;
mod drop_index;
mod drop_table;
mod drop_view;
mod insert;
mod returning;
mod select;
mod traits;
mod truncate_table;
mod update;

pub use alter_table::*;
pub use create_index::*;
pub use create_table::*;
pub use create_view::*;
pub use delete::*;
pub use drop_index::*;
pub use drop_table::*;
pub use drop_view::*;
pub use insert::*;
pub use returning::*;
pub use select::{
	CommonTableExpr, LockBehavior, LockClause, LockType, SelectDistinct, SelectExpr,
	SelectStatement, UnionType,
};
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
