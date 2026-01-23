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
