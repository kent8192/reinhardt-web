//! # reinhardt-query
//!
//! A type-safe SQL query builder for the Reinhardt framework.
//!
//! This crate provides a fluent API for constructing SQL queries that target
//! PostgreSQL, MySQL, and SQLite databases. It generates parameterized queries
//! with proper identifier escaping and value placeholders for each backend.
//!
//! ## Features
//!
//! - **Type-safe query construction** - Build SELECT, INSERT, UPDATE, DELETE statements
//! - **DCL (Data Control Language) support** - Build GRANT and REVOKE statements
//! - **Multi-backend support** - PostgreSQL, MySQL, SQLite with proper dialect handling
//! - **Expression system** - Rich expression API with arithmetic, comparison, and logical operators
//! - **Advanced SQL features** - JOINs, GROUP BY, HAVING, DISTINCT, UNION, CTEs, Window functions
//! - **Parameterized queries** - Automatic placeholder generation (`$1` for PostgreSQL, `?` for MySQL/SQLite)
//!
//! ## Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`value`]: Core value types for representing SQL values
//! - [`types`]: Identifier, column reference, table reference, and operator types
//! - [`expr`]: Expression building with the [`ExprTrait`] system
//! - [`query`]: Query builders ([`SelectStatement`],
//!   [`InsertStatement`], [`UpdateStatement`],
//!   [`DeleteStatement`])
//! - [`dcl`]: DCL (Data Control Language) builders ([`GrantStatement`],
//!   [`RevokeStatement`], [`Privilege`], [`ObjectType`], [`Grantee`])
//! - [`backend`]: Database backend implementations
//!   ([`PostgresQueryBuilder`],
//!   [`MySqlQueryBuilder`],
//!   [`SqliteQueryBuilder`])
//!
//! ## Quick Start
//!
//! ```rust
//! use reinhardt_query::prelude::*;
//!
//! // Build a SELECT query
//! let mut stmt = Query::select();
//! stmt.column("name")
//!     .column("email")
//!     .from("users")
//!     .and_where(Expr::col("active").eq(true))
//!     .order_by("name", Order::Asc)
//!     .limit(10);
//!
//! // Generate SQL for PostgreSQL
//! let builder = PostgresQueryBuilder::new();
//! let (sql, values) = builder.build_select(&stmt);
//! assert_eq!(
//!     sql,
//!     r#"SELECT "name", "email" FROM "users" WHERE "active" = $1 ORDER BY "name" ASC LIMIT $2"#
//! );
//! assert_eq!(values.len(), 2);
//! ```
//!
//! ## Backend Differences
//!
//! | Feature | PostgreSQL | MySQL | SQLite |
//! |---------|-----------|-------|--------|
//! | Identifier quoting | `"name"` | `` `name` `` | `"name"` |
//! | Placeholders | `$1, $2, ...` | `?, ?, ...` | `?, ?, ...` |
//! | NULLS FIRST/LAST | Native | Not supported | Native |
//! | DISTINCT ON | Supported | Not supported | Not supported |
//! | Window functions | Full support | Full support | Full support |
//! | CTEs (WITH) | Supported | Supported | Supported |
//! | **DCL Operations** | | | |
//! | GRANT/REVOKE | Full support | Full support | Not supported (panics) |
//! | CREATE/DROP/ALTER ROLE | Full support | Full support | Not supported (panics) |
//! | CREATE/DROP/ALTER USER | Full support | Full support | Not supported (panics) |
//! | RENAME USER | Not supported (use ALTER ROLE) | Supported | Not supported (panics) |
//! | SET ROLE | Supported | Supported | Not supported (panics) |
//! | RESET ROLE | Supported | Not supported (panics) | Not supported (panics) |
//! | SET DEFAULT ROLE | Not supported (panics) | Supported | Not supported (panics) |
//!
//! ## Expression Examples
//!
//! ```rust
//! use reinhardt_query::prelude::*;
//!
//! // Arithmetic expressions
//! let expr = Expr::col("price").mul(Expr::col("quantity"));
//!
//! // Comparison with chaining
//! let cond = Expr::col("age").gte(18i32).and(Expr::col("active").eq(true));
//!
//! // CASE WHEN expressions
//! let case_expr = Expr::case()
//!     .when(Expr::col("score").gte(90i32), "A")
//!     .when(Expr::col("score").gte(80i32), "B")
//!     .else_result("C");
//!
//! // LIKE pattern matching
//! let like_expr = Expr::col("email").like("%@example.com");
//! ```
//!
//! ## DCL Examples
//!
//! ### Privilege Management
//!
//! ```rust
//! use reinhardt_query::prelude::*;
//!
//! // GRANT privileges
//! let grant_stmt = Query::grant()
//!     .privilege(Privilege::Select)
//!     .privilege(Privilege::Insert)
//!     .on_table("users")
//!     .to("app_user")
//!     .with_grant_option(true);
//!
//! let builder = PostgresQueryBuilder::new();
//! let (sql, values) = builder.build_grant(&grant_stmt);
//! // sql = r#"GRANT SELECT, INSERT ON TABLE "users" TO "app_user" WITH GRANT OPTION"#
//!
//! // REVOKE privileges
//! let revoke_stmt = Query::revoke()
//!     .privilege(Privilege::Insert)
//!     .from_table("users")
//!     .from("app_user")
//!     .cascade(true);
//!
//! let (sql, values) = builder.build_revoke(&revoke_stmt);
//! // sql = r#"REVOKE INSERT ON TABLE "users" FROM "app_user" CASCADE"#
//! ```
//!
//! ### Role Management
//!
//! ```rust
//! use reinhardt_query::prelude::*;
//!
//! // PostgreSQL: CREATE ROLE with attributes
//! let create_role = Query::create_role()
//!     .role("app_admin")
//!     .attribute(RoleAttribute::Login)
//!     .attribute(RoleAttribute::CreateDb)
//!     .attribute(RoleAttribute::Password("secure_password".to_string()));
//!
//! let builder = PostgresQueryBuilder::new();
//! let (sql, _) = builder.build_create_role(&create_role);
//! // sql = r#"CREATE ROLE "app_admin" WITH LOGIN CREATEDB PASSWORD $1"#
//!
//! // ALTER ROLE
//! let alter_role = Query::alter_role()
//!     .role("app_admin")
//!     .attribute(RoleAttribute::CreateRole);
//!
//! let (sql, _) = builder.build_alter_role(&alter_role);
//! // sql = r#"ALTER ROLE "app_admin" WITH CREATEROLE"#
//!
//! // DROP ROLE
//! let drop_role = Query::drop_role()
//!     .role("app_admin")
//!     .if_exists(true);
//!
//! let (sql, _) = builder.build_drop_role(&drop_role);
//! // sql = r#"DROP ROLE IF EXISTS "app_admin""#
//! ```
//!
//! ### User Management
//!
//! ```rust
//! use reinhardt_query::prelude::*;
//!
//! // MySQL: CREATE USER with options
//! let create_user = Query::create_user()
//!     .user("webapp@localhost")
//!     .if_not_exists(true)
//!     .option(UserOption::Password("webapp_pass".to_string()))
//!     .option(UserOption::AccountUnlock);
//!
//! let builder = MySqlQueryBuilder::new();
//! let (sql, _) = builder.build_create_user(&create_user);
//! // sql = r#"CREATE USER IF NOT EXISTS `webapp@localhost` IDENTIFIED BY ? ACCOUNT UNLOCK"#
//!
//! // MySQL: RENAME USER
//! let rename = Query::rename_user()
//!     .rename("old_user", "new_user");
//!
//! let (sql, _) = builder.build_rename_user(&rename);
//! // sql = r#"RENAME USER `old_user` TO `new_user`"#
//! ```
//!
//! ### Session Management
//!
//! ```rust
//! use reinhardt_query::prelude::*;
//!
//! // PostgreSQL: SET ROLE
//! let set_role = Query::set_role()
//!     .role(RoleTarget::Named("admin".to_string()));
//!
//! let builder = PostgresQueryBuilder::new();
//! let (sql, _) = builder.build_set_role(&set_role);
//! // sql = r#"SET ROLE "admin""#
//!
//! // PostgreSQL: RESET ROLE
//! let reset_role = Query::reset_role();
//!
//! let (sql, _) = builder.build_reset_role(&reset_role);
//! // sql = r#"RESET ROLE"#
//!
//! // MySQL: SET DEFAULT ROLE
//! let set_default = Query::set_default_role()
//!     .roles(DefaultRoleSpec::All)
//!     .user("webapp");
//!
//! let builder = MySqlQueryBuilder::new();
//! let (sql, _) = builder.build_set_default_role(&set_default);
//! // sql = r#"SET DEFAULT ROLE ALL TO `webapp`"#
//! ```
//!
//! ## Feature Flags
//!
//! - `thread-safe`: Use `Arc` instead of `Rc` for `DynIden` (enables thread-safe identifiers)
//! - `with-chrono`: Enable chrono date/time types in `Value`
//! - `with-uuid`: Enable UUID type in `Value`
//! - `with-json`: Enable JSON type in `Value`
//! - `with-rust_decimal`: Enable Decimal type in `Value`
//! - `with-bigdecimal`: Enable BigDecimal type in `Value`
//! - `full`: Enable all optional features

// Core modules
pub mod types;
pub mod value;

// Expression module
pub mod expr;

// Query builders
pub mod query;

// Backend implementations
pub mod backend;

// DCL (Data Control Language) module
pub mod dcl;

/// Prelude module for convenient imports.
///
/// Import everything from this module to get started quickly:
///
/// ```rust
/// use reinhardt_query::prelude::*;
/// ```
pub mod prelude {
	pub use crate::backend::{
		MySqlQueryBuilder, PostgresQueryBuilder, QueryBuilder, SqlWriter, SqliteQueryBuilder,
	};
	pub use crate::dcl::{
		AlterRoleStatement, AlterUserStatement, CreateRoleStatement, CreateUserStatement,
		DefaultRoleSpec, DropRoleStatement, DropUserStatement, GrantRoleStatement, GrantStatement,
		Grantee, ObjectType, Privilege, RenameUserStatement, ResetRoleStatement,
		RevokeRoleStatement, RevokeStatement, RoleAttribute, RoleSpecification, RoleTarget,
		SetDefaultRoleStatement, SetRoleStatement, UserOption,
	};
	pub use crate::expr::{
		CaseExprBuilder, CaseStatement, Cond, Condition, ConditionExpression, ConditionHolder,
		ConditionType, Expr, ExprTrait, IntoCondition, Keyword, SimpleExpr,
	};
	pub use crate::query::{
		DeleteStatement, InsertStatement, Query, QueryBuilderTrait, QueryStatementBuilder,
		QueryStatementWriter, SelectStatement, UpdateStatement,
	};
	pub use crate::types::{
		Alias, ColumnRef, DynIden, Iden, IdenStatic, IntoColumnRef, IntoIden, IntoTableRef, Order,
		TableRef,
	};
	pub use crate::value::{ArrayType, IntoValue, Value, ValueTuple, Values};
}

// Re-export commonly used types at crate root
pub use prelude::*;
