//! # reinhardt-query
//!
//! SQL query builder for Reinhardt framework, compatible with SeaQuery API.
//!
//! This crate provides a type-safe, fluent API for building SQL queries that can be
//! executed against PostgreSQL, MySQL, and SQLite databases.
//!
//! ## Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`value`]: Core value types for representing SQL values
//! - [`types`]: Identifier, column reference, and table reference types
//! - [`expr`]: Expression building and the expression trait system
//! - [`query`]: Query builders (SELECT, INSERT, UPDATE, DELETE)
//! - [`schema`]: Schema builders (CREATE, ALTER, DROP TABLE)
//! - [`backend`]: Database backend implementations (PostgreSQL, MySQL, SQLite)
//! - [`func`]: SQL function support
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_query::prelude::*;
//!
//! // Build a SELECT query
//! let query = Query::select()
//!     .column(Expr::col(Users::Id))
//!     .column(Expr::col(Users::Name))
//!     .from(Users::Table)
//!     .and_where(Expr::col(Users::Active).eq(true))
//!     .order_by(Users::Name, Order::Asc)
//!     .limit(10);
//!
//! // Generate SQL for PostgreSQL
//! let (sql, values) = query.build(PostgresQueryBuilder);
//! // sql = "SELECT \"id\", \"name\" FROM \"users\" WHERE \"active\" = $1 ORDER BY \"name\" ASC LIMIT $2"
//! // values = [true, 10]
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

// Core modules (Phase 1)
pub mod types;
pub mod value;

// Expression module (Phase 2)
pub mod expr;

// Query builders (Phase 3)
pub mod query;

// Schema builders (Phase 4 - placeholder)
// pub mod schema;

// Backend implementations (Phase 5)
pub mod backend;

// SQL functions (Phase 6 - placeholder)
// pub mod func;

// SQL writer infrastructure
// pub mod prepare;

/// Prelude module for convenient imports
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
		Alias, ColumnRef, DynIden, Iden, IdenStatic, IntoColumnRef, IntoIden, IntoTableRef,
		TableRef,
	};
	pub use crate::value::{ArrayType, IntoValue, Value, ValueTuple, Values};
}

// Re-export commonly used types at crate root
pub use prelude::*;
