//! # Query Execution
//!
//! SQLAlchemy-inspired query execution methods.
//!
//! This module provides execution methods similar to SQLAlchemy's Query class

use crate::Model;
use sea_query::{Alias, Expr, ExprTrait, Func, Query, SelectStatement};
use std::marker::PhantomData;

/// Query execution result types
#[derive(Debug)]
pub enum ExecutionResult<T> {
	/// Single result
	One(T),
	/// Optional single result
	OneOrNone(Option<T>),
	/// Multiple results
	All(Vec<T>),
	/// Scalar value (for aggregates)
	Scalar(String),
	/// No result (for mutations)
	None,
}

/// Query execution methods
/// These would be async in a real implementation
pub trait QueryExecution<T: Model> {
	/// Get a single result by primary key
	/// Corresponds to SQLAlchemy's .get()
	fn get(&self, pk: &T::PrimaryKey) -> SelectStatement;

	/// Get all results
	/// Corresponds to SQLAlchemy's .all()
	fn all(&self) -> SelectStatement;

	/// Get first result or None
	/// Corresponds to SQLAlchemy's .first()
	fn first(&self) -> SelectStatement;

	/// Get exactly one result, raise if 0 or >1
	/// Corresponds to SQLAlchemy's .one()
	fn one(&self) -> SelectStatement;

	/// Get one result or None, raise if >1
	/// Corresponds to SQLAlchemy's .one_or_none()
	fn one_or_none(&self) -> SelectStatement;

	/// Get scalar value (first column of first row)
	/// Corresponds to SQLAlchemy's .scalar()
	fn scalar(&self) -> SelectStatement;

	/// Count results
	/// Corresponds to SQLAlchemy's .count()
	fn count(&self) -> SelectStatement;

	/// Check if any results exist
	/// Corresponds to SQLAlchemy's .exists()
	fn exists(&self) -> SelectStatement;
}

/// Execution context for SELECT queries
pub struct SelectExecution<T: Model> {
	stmt: SelectStatement,
	_phantom: PhantomData<T>,
}

impl<T: Model> SelectExecution<T> {
	/// Create a new query execution context with the given SelectStatement
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_orm::execution::SelectExecution;
	/// use reinhardt_orm::Model;
	/// use sea_query::{Alias, Query};
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let stmt = Query::select().from(Alias::new("users")).to_owned();
	/// let exec = SelectExecution::<User>::new(stmt);
	/// ```
	pub fn new(stmt: SelectStatement) -> Self {
		Self {
			stmt,
			_phantom: PhantomData,
		}
	}
	/// Get a reference to the underlying SelectStatement
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_orm::execution::SelectExecution;
	/// use reinhardt_orm::Model;
	/// use sea_query::{Alias, Expr, ExprTrait, Query};
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let stmt = Query::select()
	///     .from(Alias::new("users"))
	///     .and_where(Expr::col(Alias::new("active")).eq(true))
	///     .to_owned();
	/// let exec = SelectExecution::<User>::new(stmt);
	/// ```
	pub fn statement(&self) -> &SelectStatement {
		&self.stmt
	}
}

impl<T: Model> QueryExecution<T> for SelectExecution<T>
where
	T::PrimaryKey: Into<sea_query::Value> + Clone,
{
	fn get(&self, pk: &T::PrimaryKey) -> SelectStatement {
		Query::select()
			.from(Alias::new(T::table_name()))
			.column(sea_query::Asterisk)
			.and_where(Expr::col(Alias::new(T::primary_key_field())).eq(pk.clone()))
			.limit(1)
			.to_owned()
	}

	fn all(&self) -> SelectStatement {
		self.stmt.clone()
	}

	fn first(&self) -> SelectStatement {
		let mut stmt = self.stmt.clone();
		stmt.limit(1);
		stmt
	}

	fn one(&self) -> SelectStatement {
		// In real implementation, this would check result count
		let mut stmt = self.stmt.clone();
		stmt.limit(2);
		stmt
	}

	fn one_or_none(&self) -> SelectStatement {
		let mut stmt = self.stmt.clone();
		stmt.limit(2);
		stmt
	}

	fn scalar(&self) -> SelectStatement {
		let mut stmt = self.stmt.clone();
		stmt.limit(1);
		stmt
	}

	fn count(&self) -> SelectStatement {
		// Use the original statement as a subquery and count all rows from it
		// This preserves all WHERE, JOIN, and other conditions
		Query::select()
			.expr(Func::count(Expr::col(sea_query::Asterisk)))
			.from_subquery(self.stmt.clone(), Alias::new("subquery"))
			.to_owned()
	}

	fn exists(&self) -> SelectStatement {
		Query::select()
			.expr(Expr::exists(self.stmt.clone()))
			.to_owned()
	}
}

/// Loading options for relationships
/// Corresponds to SQLAlchemy's loader options
#[derive(Debug, Clone)]
pub enum LoadOption {
	/// Eager load with JOIN
	/// Corresponds to joinedload()
	JoinedLoad(String),

	/// Eager load with separate SELECT
	/// Corresponds to selectinload()
	SelectInLoad(String),

	/// Lazy load on access
	/// Corresponds to lazyload()
	LazyLoad(String),

	/// Don't load at all
	/// Corresponds to noload()
	NoLoad(String),

	/// Raise error if accessed
	/// Corresponds to raiseload()
	RaiseLoad(String),

	/// Defer column loading
	/// Corresponds to defer()
	Defer(String),

	/// Undefer column loading
	/// Corresponds to undefer()
	Undefer(String),

	/// Load only specified columns
	/// Corresponds to load_only()
	LoadOnly(Vec<String>),
}

impl LoadOption {
	/// Convert load option to SQL comment for debugging
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::execution::LoadOption;
	///
	/// let option = LoadOption::JoinedLoad("profile".to_string());
	/// assert_eq!(option.to_sql_comment(), "/* joinedload(profile) */");
	///
	/// let option = LoadOption::Defer("password".to_string());
	/// assert_eq!(option.to_sql_comment(), "/* defer(password) */");
	///
	/// let option = LoadOption::LoadOnly(vec!["id".to_string(), "name".to_string()]);
	/// assert_eq!(option.to_sql_comment(), "/* load_only(id, name) */");
	/// ```
	pub fn to_sql_comment(&self) -> String {
		match self {
			LoadOption::JoinedLoad(rel) => format!("/* joinedload({}) */", rel),
			LoadOption::SelectInLoad(rel) => format!("/* selectinload({}) */", rel),
			LoadOption::LazyLoad(rel) => format!("/* lazyload({}) */", rel),
			LoadOption::NoLoad(rel) => format!("/* noload({}) */", rel),
			LoadOption::RaiseLoad(rel) => format!("/* raiseload({}) */", rel),
			LoadOption::Defer(col) => format!("/* defer({}) */", col),
			LoadOption::Undefer(col) => format!("/* undefer({}) */", col),
			LoadOption::LoadOnly(cols) => format!("/* load_only({}) */", cols.join(", ")),
		}
	}
}

/// Query options container
pub struct QueryOptions {
	pub load_options: Vec<LoadOption>,
}

impl QueryOptions {
	/// Create a new empty query options container
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::execution::QueryOptions;
	///
	/// let options = QueryOptions::new();
	/// assert_eq!(options.to_sql_comments(), "");
	/// ```
	pub fn new() -> Self {
		Self {
			load_options: Vec::new(),
		}
	}
	/// Add a load option to the query
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::execution::{QueryOptions, LoadOption};
	///
	/// let options = QueryOptions::new()
	///     .add_option(LoadOption::JoinedLoad("profile".to_string()))
	///     .add_option(LoadOption::Defer("password".to_string()));
	///
	/// let comments = options.to_sql_comments();
	/// assert!(comments.contains("joinedload(profile)"));
	/// assert!(comments.contains("defer(password)"));
	/// ```
	pub fn add_option(mut self, option: LoadOption) -> Self {
		self.load_options.push(option);
		self
	}
	/// Convert all options to SQL comments
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::execution::{QueryOptions, LoadOption};
	///
	/// let options = QueryOptions::new()
	///     .add_option(LoadOption::SelectInLoad("posts".to_string()));
	///
	/// assert!(options.to_sql_comments().contains("selectinload(posts)"));
	/// ```
	pub fn to_sql_comments(&self) -> String {
		if self.load_options.is_empty() {
			String::new()
		} else {
			format!(
				" {}",
				self.load_options
					.iter()
					.map(|o| o.to_sql_comment())
					.collect::<Vec<_>>()
					.join(" ")
			)
		}
	}
}

impl Default for QueryOptions {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_validators::TableName;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct User {
		id: Option<i64>,
		name: String,
	}

	const USER_TABLE: TableName = TableName::new_const("users");

	impl Model for User {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			USER_TABLE.as_str()
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[test]
	fn test_execution_get() {
		use sea_query::{Alias, PostgresQueryBuilder, Query};

		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(sea_query::Asterisk)
			.to_owned();
		let exec = SelectExecution::<User>::new(stmt);
		let result_stmt = exec.get(&123);
		let sql = result_stmt.to_string(PostgresQueryBuilder);
		assert!(sql.contains("WHERE"));
		assert!(sql.contains("LIMIT"));
	}

	#[test]
	fn test_all() {
		use sea_query::{Alias, PostgresQueryBuilder, Query};

		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(sea_query::Asterisk)
			.to_owned();
		let exec = SelectExecution::<User>::new(stmt);
		let result_stmt = exec.all();
		let sql = result_stmt.to_string(PostgresQueryBuilder);
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("users"));
	}

	#[test]
	fn test_first() {
		use sea_query::{Alias, Expr, PostgresQueryBuilder, Query};

		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(sea_query::Asterisk)
			.and_where(Expr::col(Alias::new("active")).eq(true))
			.to_owned();
		let exec = SelectExecution::<User>::new(stmt);
		let result_stmt = exec.first();
		let sql = result_stmt.to_string(PostgresQueryBuilder);
		assert!(sql.contains("LIMIT"));
	}

	#[test]
	fn test_execution_count() {
		use sea_query::{Alias, Expr, PostgresQueryBuilder, Query};

		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(sea_query::Asterisk)
			.and_where(Expr::col(Alias::new("active")).eq(true))
			.to_owned();
		let exec = SelectExecution::<User>::new(stmt);
		let result_stmt = exec.count();
		let sql = result_stmt.to_string(PostgresQueryBuilder);
		assert!(sql.contains("COUNT"));
	}

	#[test]
	fn test_execution_exists() {
		use sea_query::{Alias, Expr, PostgresQueryBuilder, Query};

		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(sea_query::Asterisk)
			.and_where(Expr::col(Alias::new("name")).eq("Alice"))
			.to_owned();
		let exec = SelectExecution::<User>::new(stmt);
		let result_stmt = exec.exists();
		let sql = result_stmt.to_string(PostgresQueryBuilder);
		assert!(sql.contains("EXISTS"));
	}

	#[test]
	fn test_load_options() {
		let options = QueryOptions::new()
			.add_option(LoadOption::JoinedLoad("profile".to_string()))
			.add_option(LoadOption::Defer("password".to_string()));

		let comments = options.to_sql_comments();
		assert!(comments.contains("joinedload(profile)"));
		assert!(comments.contains("defer(password)"));
	}

	#[test]
	fn test_load_only() {
		let option = LoadOption::LoadOnly(vec!["id".to_string(), "name".to_string()]);
		let comment = option.to_sql_comment();
		assert!(comment.contains("load_only(id, name)"));
	}
}
