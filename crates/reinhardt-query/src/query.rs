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

	/// Construct a new GRANT statement
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let grant = Query::grant()
	///     .privilege(Privilege::Select)
	///     .on_table("users")
	///     .to("app_user");
	/// ```
	pub fn grant() -> crate::dcl::GrantStatement {
		crate::dcl::GrantStatement::new()
	}

	/// Construct a new REVOKE statement
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let revoke = Query::revoke()
	///     .privilege(Privilege::Insert)
	///     .from_table("users")
	///     .from("app_user");
	/// ```
	pub fn revoke() -> crate::dcl::RevokeStatement {
		crate::dcl::RevokeStatement::new()
	}

	/// Construct a new CREATE ROLE statement
	///
	/// # Examples
	///
	/// PostgreSQL:
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::dcl::{CreateRoleStatement, RoleAttribute};
	///
	/// let create_role = Query::create_role()
	///     .role("developer")
	///     .attribute(RoleAttribute::Login)
	///     .attribute(RoleAttribute::Password("secret".to_string()));
	/// ```
	///
	/// MySQL:
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::dcl::{CreateRoleStatement, UserOption};
	///
	/// let create_role = Query::create_role()
	///     .role("app_role")
	///     .if_not_exists(true)
	///     .option(UserOption::Comment("Application role".to_string()));
	/// ```
	pub fn create_role() -> crate::dcl::CreateRoleStatement {
		crate::dcl::CreateRoleStatement::new()
	}

	/// Construct a new DROP ROLE statement
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::dcl::DropRoleStatement;
	///
	/// let drop_role = Query::drop_role()
	///     .role("old_role")
	///     .if_exists(true);
	/// ```
	pub fn drop_role() -> crate::dcl::DropRoleStatement {
		crate::dcl::DropRoleStatement::new()
	}

	/// Construct a new ALTER ROLE statement
	///
	/// # Examples
	///
	/// PostgreSQL:
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::dcl::{AlterRoleStatement, RoleAttribute};
	///
	/// let alter_role = Query::alter_role()
	///     .role("developer")
	///     .attribute(RoleAttribute::NoLogin);
	/// ```
	///
	/// MySQL:
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	/// use reinhardt_query::dcl::{AlterRoleStatement, UserOption};
	///
	/// let alter_role = Query::alter_role()
	///     .role("app_role")
	///     .option(UserOption::AccountLock);
	/// ```
	pub fn alter_role() -> crate::dcl::AlterRoleStatement {
		crate::dcl::AlterRoleStatement::new()
	}

	/// Create a new CREATE USER statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::query::Query;
	///
	/// let stmt = Query::create_user();
	/// ```
	pub fn create_user() -> crate::dcl::CreateUserStatement {
		crate::dcl::CreateUserStatement::new()
	}

	/// Create a new DROP USER statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::query::Query;
	///
	/// let stmt = Query::drop_user();
	/// ```
	pub fn drop_user() -> crate::dcl::DropUserStatement {
		crate::dcl::DropUserStatement::new()
	}

	/// Create a new ALTER USER statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::query::Query;
	///
	/// let stmt = Query::alter_user();
	/// ```
	pub fn alter_user() -> crate::dcl::AlterUserStatement {
		crate::dcl::AlterUserStatement::new()
	}

	/// Create a new RENAME USER statement (MySQL only)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::query::Query;
	///
	/// let stmt = Query::rename_user();
	/// ```
	pub fn rename_user() -> crate::dcl::RenameUserStatement {
		crate::dcl::RenameUserStatement::new()
	}

	/// Create a new SET ROLE statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::query::Query;
	///
	/// let stmt = Query::set_role();
	/// ```
	pub fn set_role() -> crate::dcl::SetRoleStatement {
		crate::dcl::SetRoleStatement::new()
	}

	/// Create a new RESET ROLE statement (PostgreSQL only)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::query::Query;
	///
	/// let stmt = Query::reset_role();
	/// ```
	pub fn reset_role() -> crate::dcl::ResetRoleStatement {
		crate::dcl::ResetRoleStatement::new()
	}

	/// Create a new SET DEFAULT ROLE statement (MySQL only)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::query::Query;
	///
	/// let stmt = Query::set_default_role();
	/// ```
	pub fn set_default_role() -> crate::dcl::SetDefaultRoleStatement {
		crate::dcl::SetDefaultRoleStatement::new()
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
