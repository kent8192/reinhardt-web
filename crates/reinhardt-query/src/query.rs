//! Query statement builders
//!
//! This module provides builders for SQL query statements (SELECT, INSERT, UPDATE, DELETE).
//!
//! # Usage
//!
//! - Query Select: [`SelectStatement`]
//! - Query Insert: [`InsertStatement`]
//! - Query Update: [`UpdateStatement`]
//! - Query Delete: [`DeleteStatement`]
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

mod delete;
mod insert;
mod returning;
mod select;
mod traits;
mod update;

pub use delete::*;
pub use insert::*;
pub use returning::*;
pub use select::{
	CommonTableExpr, LockBehavior, LockClause, LockType, SelectDistinct, SelectExpr,
	SelectStatement, UnionType,
};
pub use traits::*;
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
