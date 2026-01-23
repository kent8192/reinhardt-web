//! Expr - The expression builder.
//!
//! This module provides [`Expr`], a builder for creating SQL expressions.
//! It provides a fluent API for building complex expressions.

use super::simple_expr::{CaseStatement, Keyword, SimpleExpr};
use crate::types::{ColumnRef, DynIden, IntoColumnRef, IntoIden};
use crate::value::IntoValue;

/// Expression builder for creating SQL expressions.
///
/// `Expr` provides static methods to create expressions and instance methods
/// to chain operations on them.
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_query::Expr;
///
/// // Simple column reference
/// let col = Expr::col("name");
///
/// // Value expression
/// let val = Expr::val(42);
///
/// // Build complex expressions
/// let expr = Expr::col("age").gte(18).and(Expr::col("active").eq(true));
/// ```
#[derive(Debug, Clone)]
pub struct Expr(SimpleExpr);

impl Expr {
	/// Create an expression from a column reference.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::Expr;
	///
	/// let expr = Expr::col("name");
	/// ```
	pub fn col<C>(col: C) -> Self
	where
		C: IntoColumnRef,
	{
		Self(SimpleExpr::Column(col.into_column_ref()))
	}

	/// Create a table-qualified column expression.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::Expr;
	///
	/// let expr = Expr::tbl("users", "name");
	/// ```
	pub fn tbl<T, C>(table: T, col: C) -> Self
	where
		T: IntoIden,
		C: IntoIden,
	{
		Self(SimpleExpr::TableColumn(table.into_iden(), col.into_iden()))
	}

	/// Create a value expression.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::Expr;
	///
	/// let expr = Expr::val(42);
	/// let expr2 = Expr::val("hello");
	/// ```
	pub fn val<V>(val: V) -> Self
	where
		V: IntoValue,
	{
		Self(SimpleExpr::Value(val.into_value()))
	}

	/// Create a custom SQL expression.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::Expr;
	///
	/// let expr = Expr::cust("NOW()");
	/// ```
	pub fn cust<S>(sql: S) -> Self
	where
		S: Into<String>,
	{
		Self(SimpleExpr::Custom(sql.into()))
	}

	/// Create a custom SQL expression with value placeholders.
	///
	/// Use `?` as placeholders for the values.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::Expr;
	///
	/// let expr = Expr::cust_with_values("? + ?", [1, 2]);
	/// ```
	pub fn cust_with_values<S, I, V>(sql: S, values: I) -> Self
	where
		S: Into<String>,
		I: IntoIterator<Item = V>,
		V: Into<SimpleExpr>,
	{
		Self(SimpleExpr::CustomWithExpr(
			sql.into(),
			values.into_iter().map(|v| v.into()).collect(),
		))
	}

	/// Create a tuple expression.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::Expr;
	///
	/// let expr = Expr::tuple([Expr::val(1), Expr::val(2), Expr::val(3)]);
	/// ```
	pub fn tuple<I>(exprs: I) -> Self
	where
		I: IntoIterator<Item = Self>,
	{
		Self(SimpleExpr::Tuple(
			exprs.into_iter().map(|e| e.into_simple_expr()).collect(),
		))
	}

	/// Create an asterisk expression (`*`).
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::Expr;
	///
	/// let expr = Expr::asterisk();
	/// ```
	pub fn asterisk() -> Self {
		Self(SimpleExpr::Asterisk)
	}

	/// Create a subquery expression.
	///
	/// This creates a standalone subquery that can be used in FROM or SELECT clauses.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let subquery = Query::select().column("id").from("users");
	/// let expr = Expr::subquery(subquery);
	/// ```
	pub fn subquery(select: crate::query::SelectStatement) -> Self {
		Self(SimpleExpr::SubQuery(None, Box::new(select)))
	}

	/// Create an EXISTS subquery expression.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let subquery = Query::select().column("id").from("orders").and_where(
	///     Expr::col(("orders", "user_id")).eq(Expr::col(("users", "id")))
	/// );
	/// let exists = Expr::exists(subquery);
	/// ```
	pub fn exists(select: crate::query::SelectStatement) -> Self {
		Self(SimpleExpr::SubQuery(
			Some(super::simple_expr::SubQueryOper::Exists),
			Box::new(select),
		))
	}

	/// Create a NOT EXISTS subquery expression.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let subquery = Query::select().column("id").from("banned_users").and_where(
	///     Expr::col(("banned_users", "id")).eq(Expr::col(("users", "id")))
	/// );
	/// let not_exists = Expr::not_exists(subquery);
	/// ```
	pub fn not_exists(select: crate::query::SelectStatement) -> Self {
		Self(SimpleExpr::SubQuery(
			Some(super::simple_expr::SubQueryOper::NotExists),
			Box::new(select),
		))
	}

	/// Create an IN subquery expression.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let subquery = Query::select().column("user_id").from("premium_users");
	/// let in_expr = Expr::col("id").in_subquery(subquery);
	/// ```
	pub fn in_subquery(self, select: crate::query::SelectStatement) -> Self {
		Self(SimpleExpr::Binary(
			Box::new(self.0),
			crate::types::BinOper::In,
			Box::new(SimpleExpr::SubQuery(None, Box::new(select))),
		))
	}

	/// Create a NOT IN subquery expression.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let subquery = Query::select().column("user_id").from("banned_users");
	/// let not_in_expr = Expr::col("id").not_in_subquery(subquery);
	/// ```
	pub fn not_in_subquery(self, select: crate::query::SelectStatement) -> Self {
		Self(SimpleExpr::Binary(
			Box::new(self.0),
			crate::types::BinOper::NotIn,
			Box::new(SimpleExpr::SubQuery(None, Box::new(select))),
		))
	}

	/// Create a CASE expression.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::expr::{Expr, ExprTrait};
	///
	/// let case = Expr::case()
	///     .when(Expr::col("status").eq("active"), "Active")
	///     .when(Expr::col("status").eq("pending"), "Pending")
	///     .else_result("Unknown");
	/// ```
	pub fn case() -> CaseExprBuilder {
		CaseExprBuilder {
			case: CaseStatement::new(),
		}
	}

	/// Create a value expression (alias for [`val`]).
	pub fn value<V>(val: V) -> Self
	where
		V: IntoValue,
	{
		Self::val(val)
	}

	/// Create an expression from a DynIden column.
	pub fn expr_col(col: DynIden) -> Self {
		Self(SimpleExpr::Column(ColumnRef::Column(col)))
	}

	// Constant expressions

	/// Create a NULL constant expression.
	pub fn null() -> Self {
		Self(SimpleExpr::Constant(Keyword::Null))
	}

	/// Create a TRUE constant expression.
	pub fn constant_true() -> Self {
		Self(SimpleExpr::Constant(Keyword::True))
	}

	/// Create a FALSE constant expression.
	pub fn constant_false() -> Self {
		Self(SimpleExpr::Constant(Keyword::False))
	}

	/// Create a DEFAULT constant expression.
	pub fn default() -> Self {
		Self(SimpleExpr::Constant(Keyword::Default))
	}

	/// Create a CURRENT_TIMESTAMP expression.
	pub fn current_timestamp() -> Self {
		Self(SimpleExpr::Constant(Keyword::CurrentTimestamp))
	}

	/// Create a CURRENT_DATE expression.
	pub fn current_date() -> Self {
		Self(SimpleExpr::Constant(Keyword::CurrentDate))
	}

	/// Create a CURRENT_TIME expression.
	pub fn current_time() -> Self {
		Self(SimpleExpr::Constant(Keyword::CurrentTime))
	}

	// Conversion methods

	/// Convert this Expr into a SimpleExpr.
	#[must_use]
	pub fn into_simple_expr(self) -> SimpleExpr {
		self.0
	}

	/// Get a reference to the underlying SimpleExpr.
	#[must_use]
	pub fn as_simple_expr(&self) -> &SimpleExpr {
		&self.0
	}
}

// Allow Expr to be converted into SimpleExpr
impl From<Expr> for SimpleExpr {
	fn from(e: Expr) -> Self {
		e.0
	}
}

// Allow creating Expr from SimpleExpr
impl From<SimpleExpr> for Expr {
	fn from(e: SimpleExpr) -> Self {
		Self(e)
	}
}

/// Builder for CASE expressions.
#[derive(Debug, Clone)]
pub struct CaseExprBuilder {
	case: CaseStatement,
}

impl CaseExprBuilder {
	/// Add a WHEN clause.
	#[must_use]
	pub fn when<C, R>(mut self, condition: C, result: R) -> Self
	where
		C: Into<SimpleExpr>,
		R: Into<SimpleExpr>,
	{
		self.case = self.case.when(condition, result);
		self
	}

	/// Set the ELSE clause and return the built Expr.
	#[must_use]
	pub fn else_result<E>(mut self, result: E) -> Expr
	where
		E: Into<SimpleExpr>,
	{
		self.case = self.case.else_result(result);
		Expr(SimpleExpr::Case(Box::new(self.case)))
	}

	/// Build the CASE expression without an ELSE clause.
	#[must_use]
	pub fn build(self) -> Expr {
		Expr(SimpleExpr::Case(Box::new(self.case)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::value::Value;
	use rstest::rstest;

	#[rstest]
	fn test_expr_col() {
		let expr = Expr::col("name");
		assert!(matches!(expr.0, SimpleExpr::Column(_)));
	}

	#[rstest]
	fn test_expr_tbl() {
		let expr = Expr::tbl("users", "name");
		assert!(matches!(expr.0, SimpleExpr::TableColumn(_, _)));
	}

	#[rstest]
	fn test_expr_val() {
		let expr = Expr::val(42);
		assert!(matches!(expr.0, SimpleExpr::Value(Value::Int(Some(42)))));
	}

	#[rstest]
	fn test_expr_cust() {
		let expr = Expr::cust("NOW()");
		if let SimpleExpr::Custom(s) = expr.0 {
			assert_eq!(s, "NOW()");
		} else {
			panic!("Expected Custom variant");
		}
	}

	#[rstest]
	fn test_expr_cust_with_values() {
		let expr = Expr::cust_with_values("? + ?", [1i32, 2i32]);
		assert!(matches!(expr.0, SimpleExpr::CustomWithExpr(_, _)));
	}

	#[rstest]
	fn test_expr_tuple() {
		let expr = Expr::tuple([Expr::val(1), Expr::val(2), Expr::val(3)]);
		if let SimpleExpr::Tuple(v) = expr.0 {
			assert_eq!(v.len(), 3);
		} else {
			panic!("Expected Tuple variant");
		}
	}

	#[rstest]
	fn test_expr_asterisk() {
		let expr = Expr::asterisk();
		assert!(matches!(expr.0, SimpleExpr::Asterisk));
	}

	#[rstest]
	fn test_expr_null() {
		let expr = Expr::null();
		assert!(matches!(expr.0, SimpleExpr::Constant(Keyword::Null)));
	}

	#[rstest]
	fn test_expr_current_timestamp() {
		let expr = Expr::current_timestamp();
		assert!(matches!(
			expr.0,
			SimpleExpr::Constant(Keyword::CurrentTimestamp)
		));
	}

	#[rstest]
	fn test_case_expr_builder() {
		let expr = Expr::case()
			.when(true, 1i32)
			.when(false, 0i32)
			.else_result(-1i32);

		assert!(matches!(expr.0, SimpleExpr::Case(_)));
	}

	#[rstest]
	fn test_case_expr_without_else() {
		let expr = Expr::case().when(true, 1i32).build();

		assert!(matches!(expr.0, SimpleExpr::Case(_)));
	}

	#[rstest]
	fn test_expr_into_simple_expr() {
		let expr = Expr::val(42);
		let simple = expr.into_simple_expr();
		assert!(matches!(simple, SimpleExpr::Value(Value::Int(Some(42)))));
	}
}
