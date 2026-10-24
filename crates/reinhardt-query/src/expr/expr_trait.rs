//! ExprTrait - Expression operations trait.
//!
//! This module provides [`ExprTrait`], which defines all the operations
//! that can be performed on expressions (comparisons, logical ops, etc.).

use super::simple_expr::{Keyword, SimpleExpr};
use crate::types::{BinOper, UnOper};
use crate::value::Value;

/// Escape SQL LIKE wildcard characters in user input.
///
/// Escapes `\` -> `\\`, `%` -> `\%`, and `_` -> `\_` so that user-supplied
/// strings are treated as literal text in LIKE patterns.
///
/// This escaping relies on an explicit backslash `ESCAPE` clause in the
/// generated SQL. The helper functions [`ExprTrait::starts_with`],
/// [`ExprTrait::ends_with`], and [`ExprTrait::contains`] automatically
/// include this clause.
fn escape_like_pattern(input: &str) -> String {
	let mut escaped = String::with_capacity(input.len());
	for ch in input.chars() {
		match ch {
			'\\' => escaped.push_str("\\\\"),
			'%' => escaped.push_str("\\%"),
			'_' => escaped.push_str("\\_"),
			_ => escaped.push(ch),
		}
	}
	escaped
}

/// Build a LIKE expression with an explicit backslash escape character.
///
/// The explicit ESCAPE clause ensures that backslash escaping works
/// consistently across all SQL backends, including those that do not treat
/// `\` as a LIKE escape character by default (e.g., SQLite). Its literal is
/// rendered by the backend: MySQL uses `0x5C`, which is independent of its
/// string-literal backslash mode, while PostgreSQL and SQLite use `'\'`.
fn like_with_escape(expr: SimpleExpr, pattern: String) -> SimpleExpr {
	SimpleExpr::LikeWithEscape(
		Box::new(expr),
		Box::new(SimpleExpr::Value(Value::String(Some(Box::new(pattern))))),
	)
}

/// Trait for expression operations.
///
/// This trait provides methods for building complex expressions through
/// operator chaining. It is implemented for [`Expr`](crate::expr::Expr) and [`SimpleExpr`].
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_query::{Expr, ExprTrait};
///
/// // Comparison
/// let expr = Expr::col("age").gte(18);
///
/// // Logical operations
/// let expr = Expr::col("active").eq(true).and(Expr::col("verified").eq(true));
///
/// // Arithmetic
/// let expr = Expr::col("price").mul(Expr::col("quantity"));
/// ```
// Expression trait methods consume self for builder-pattern chaining,
// so is_*/as_* methods intentionally take self by value.
#[allow(clippy::wrong_self_convention)]
pub trait ExprTrait: Sized {
	/// Build the final SimpleExpr.
	fn into_simple_expr(self) -> SimpleExpr;

	/// Preserve explicit parentheses around this expression on every backend.
	///
	/// Grouping is structural; nested parameters keep their renderer order and
	/// validators inspect the inner expression. This supports native/WASM
	/// behavioral parity (P2).
	///
	/// # Example
	///
	/// ```
	/// use reinhardt_query::{Expr, ExprTrait, MySqlQueryBuilder, Query};
	/// let statement = Query::select()
	///     .expr(Expr::val(3_i64).add(5_i64).grouped().mul(2_i64))
	///     .to_owned();
	/// let (sql, values) = MySqlQueryBuilder.build_select_checked(&statement).unwrap();
	/// assert_eq!(sql, "SELECT (? + ?) * ?");
	/// assert_eq!(values.0, vec![3_i64.into(), 5_i64.into(), 2_i64.into()]);
	/// ```
	fn grouped(self) -> SimpleExpr {
		SimpleExpr::Grouped(Box::new(self.into_simple_expr()))
	}

	// =========================================================================
	// Comparison operations
	// =========================================================================

	/// Equal (`=`).
	///
	/// # Example
	///
	/// ```rust,ignore
	/// Expr::col("name").eq("Alice")
	/// // Generates: "name" = 'Alice'
	/// ```
	fn eq<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Equal,
			Box::new(v.into()),
		)
	}

	/// Not equal (`<>`).
	fn ne<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::NotEqual,
			Box::new(v.into()),
		)
	}

	/// Less than (`<`).
	fn lt<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::SmallerThan,
			Box::new(v.into()),
		)
	}

	/// Less than or equal (`<=`).
	fn lte<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::SmallerThanOrEqual,
			Box::new(v.into()),
		)
	}

	/// Greater than (`>`).
	fn gt<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::GreaterThan,
			Box::new(v.into()),
		)
	}

	/// Greater than or equal (`>=`).
	fn gte<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::GreaterThanOrEqual,
			Box::new(v.into()),
		)
	}

	// =========================================================================
	// NULL checks
	// =========================================================================

	/// IS NULL.
	fn is_null(self) -> SimpleExpr {
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Is,
			Box::new(SimpleExpr::Constant(Keyword::Null)),
		)
	}

	/// IS NOT NULL.
	fn is_not_null(self) -> SimpleExpr {
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::IsNot,
			Box::new(SimpleExpr::Constant(Keyword::Null)),
		)
	}

	// =========================================================================
	// Range operations
	// =========================================================================

	/// BETWEEN.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// Expr::col("age").between(18, 65)
	/// // Generates: "age" BETWEEN 18 AND 65
	/// ```
	fn between<A, B>(self, a: A, b: B) -> SimpleExpr
	where
		A: Into<SimpleExpr>,
		B: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Between,
			Box::new(SimpleExpr::Tuple(vec![a.into(), b.into()])),
		)
	}

	/// NOT BETWEEN.
	fn not_between<A, B>(self, a: A, b: B) -> SimpleExpr
	where
		A: Into<SimpleExpr>,
		B: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::NotBetween,
			Box::new(SimpleExpr::Tuple(vec![a.into(), b.into()])),
		)
	}

	// =========================================================================
	// Set membership
	// =========================================================================

	/// IN.
	///
	/// Returns `FALSE` when the iterator is empty, since `x IN ()` is invalid SQL.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// Expr::col("status").is_in(["active", "pending"])
	/// // Generates: "status" IN ('active', 'pending')
	/// ```
	fn is_in<I, V>(self, values: I) -> SimpleExpr
	where
		I: IntoIterator<Item = V>,
		V: Into<SimpleExpr>,
	{
		let collected: Vec<SimpleExpr> = values.into_iter().map(|v| v.into()).collect();
		if collected.is_empty() {
			// Empty IN () is invalid SQL in all databases; use FALSE instead
			return SimpleExpr::Constant(Keyword::False);
		}
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::In,
			Box::new(SimpleExpr::Tuple(collected)),
		)
	}

	/// NOT IN.
	///
	/// Returns `TRUE` when the iterator is empty, since `x NOT IN ()` is invalid SQL
	/// and logically equivalent to `TRUE`.
	fn is_not_in<I, V>(self, values: I) -> SimpleExpr
	where
		I: IntoIterator<Item = V>,
		V: Into<SimpleExpr>,
	{
		let collected: Vec<SimpleExpr> = values.into_iter().map(|v| v.into()).collect();
		if collected.is_empty() {
			// Empty NOT IN () is invalid SQL in all databases; use TRUE instead
			return SimpleExpr::Constant(Keyword::True);
		}
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::NotIn,
			Box::new(SimpleExpr::Tuple(collected)),
		)
	}

	// =========================================================================
	// Pattern matching
	// =========================================================================

	/// LIKE.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// Expr::col("name").like("%john%")
	/// // Generates: "name" LIKE '%john%'
	/// ```
	fn like<V>(self, pattern: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Like,
			Box::new(pattern.into()),
		)
	}

	/// NOT LIKE.
	fn not_like<V>(self, pattern: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::NotLike,
			Box::new(pattern.into()),
		)
	}

	/// ILIKE (case-insensitive LIKE, PostgreSQL).
	fn ilike<V>(self, pattern: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::ILike,
			Box::new(pattern.into()),
		)
	}

	/// Portable case-insensitive LIKE with a fixed backslash escape character.
	///
	/// The supplied pattern is already escaped: `%` and `_` remain wildcards
	/// unless preceded by a backslash, and a literal backslash must be doubled. Ordinary
	/// values are bound by the renderer. PostgreSQL/CockroachDB use ILIKE;
	/// MySQL/SQLite use LOWER on both operands with LIKE. Case folding follows
	/// each backend's locale/collation and LOWER behavior; SQLite's built-in
	/// LOWER only folds ASCII. MySQL renders the escape as a hex literal, which
	/// is independent of NO_BACKSLASH_ESCAPES. Native/WASM behavioral parity (P2).
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::{Expr, ExprTrait, PostgresQueryBuilder, Query, QueryStatementBuilder};
	/// let statement = Query::select()
	///     .column("name")
	///     .from("users")
	///     .and_where(Expr::col("name").ilike_with_escape("%Alice\\_%"))
	///     .to_owned();
	/// let (sql, values) = statement.build(PostgresQueryBuilder);
	/// assert_eq!(sql, "SELECT \"name\" FROM \"users\" WHERE (\"name\" ILIKE $1 ESCAPE '\\')");
	/// assert_eq!(values.0, vec!["%Alice\\_%".into()]);
	/// ```
	fn ilike_with_escape<V>(self, pattern: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::InsensitiveLikeWithEscape(
			Box::new(self.into_simple_expr()),
			Box::new(pattern.into()),
		)
	}

	/// NOT ILIKE (PostgreSQL).
	fn not_ilike<V>(self, pattern: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::NotILike,
			Box::new(pattern.into()),
		)
	}

	/// Helper for LIKE with prefix wildcard.
	///
	/// SQL wildcard characters (`%`, `_`) and the escape character (`\`) in
	/// user input are escaped before constructing the pattern. The generated
	/// SQL includes an explicit backslash `ESCAPE` clause so that the
	/// escaping is portable across all backends (including SQLite, which does
	/// not treat `\` as an escape character by default). MySQL renders the
	/// escape character as `0x5C`; PostgreSQL and SQLite render it as `'\'`.
	fn starts_with<S>(self, prefix: S) -> SimpleExpr
	where
		S: Into<String>,
	{
		let escaped = escape_like_pattern(&prefix.into());
		let pattern = format!("{}%", escaped);
		like_with_escape(self.into_simple_expr(), pattern)
	}

	/// Helper for LIKE with suffix wildcard.
	///
	/// SQL wildcard characters (`%`, `_`) and the escape character (`\`) in
	/// user input are escaped before constructing the pattern. The generated
	/// SQL includes an explicit backslash `ESCAPE` clause for cross-backend
	/// portability.
	fn ends_with<S>(self, suffix: S) -> SimpleExpr
	where
		S: Into<String>,
	{
		let escaped = escape_like_pattern(&suffix.into());
		let pattern = format!("%{}", escaped);
		like_with_escape(self.into_simple_expr(), pattern)
	}

	/// Helper for LIKE with both wildcards.
	///
	/// SQL wildcard characters (`%`, `_`) and the escape character (`\`) in
	/// user input are escaped before constructing the pattern. The generated
	/// SQL includes an explicit backslash `ESCAPE` clause for cross-backend
	/// portability.
	fn contains<S>(self, substring: S) -> SimpleExpr
	where
		S: Into<String>,
	{
		let escaped = escape_like_pattern(&substring.into());
		let pattern = format!("%{}%", escaped);
		like_with_escape(self.into_simple_expr(), pattern)
	}

	// =========================================================================
	// Logical operations
	// =========================================================================

	/// AND.
	fn and<E>(self, other: E) -> SimpleExpr
	where
		E: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::And,
			Box::new(other.into()),
		)
	}

	/// OR.
	fn or<E>(self, other: E) -> SimpleExpr
	where
		E: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Or,
			Box::new(other.into()),
		)
	}

	/// NOT (unary).
	fn not(self) -> SimpleExpr {
		SimpleExpr::Unary(UnOper::Not, Box::new(self.into_simple_expr()))
	}

	// =========================================================================
	// Arithmetic operations
	// =========================================================================

	/// Addition (`+`).
	fn add<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Add,
			Box::new(v.into()),
		)
	}

	/// Subtraction (`-`).
	fn sub<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Sub,
			Box::new(v.into()),
		)
	}

	/// Multiplication (`*`).
	fn mul<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Mul,
			Box::new(v.into()),
		)
	}

	/// Division (`/`).
	fn div<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Div,
			Box::new(v.into()),
		)
	}

	/// Modulo (`%`).
	fn modulo<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::Mod,
			Box::new(v.into()),
		)
	}

	// =========================================================================
	// Bitwise operations
	// =========================================================================

	/// Bitwise AND (`&`).
	fn bit_and<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::BitAnd,
			Box::new(v.into()),
		)
	}

	/// Bitwise OR (`|`).
	fn bit_or<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::BitOr,
			Box::new(v.into()),
		)
	}

	/// Left shift (`<<`).
	fn left_shift<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::LShift,
			Box::new(v.into()),
		)
	}

	/// Right shift (`>>`).
	fn right_shift<V>(self, v: V) -> SimpleExpr
	where
		V: Into<SimpleExpr>,
	{
		SimpleExpr::Binary(
			Box::new(self.into_simple_expr()),
			BinOper::RShift,
			Box::new(v.into()),
		)
	}

	// =========================================================================
	// Type casting
	// =========================================================================

	/// Cast an expression to a type identifier.
	///
	/// The type name is quoted and escaped as a single identifier, preserving its
	/// case. It is not raw SQL: type modifiers and multi-word SQL type keywords
	/// cannot be passed as the type name.
	///
	/// PostgreSQL resolves quoted type names case-sensitively. Use lowercase
	/// catalog names for built-in types, such as `"text"`, `"int4"`, and
	/// `"timestamptz"`. For example, `"TEXT"` refers to a user-defined type named
	/// exactly `TEXT`, not the built-in `text` type. User-defined types must use
	/// their exact name, including case.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_query::prelude::*;
	///
	/// let (sql, values) = Query::select()
	///     .expr(Expr::col("age").cast_as("text"))
	///     .from("users")
	///     .build(PostgresQueryBuilder);
	///
	/// assert_eq!(sql, r#"SELECT CAST("age" AS "text") FROM "users""#);
	/// assert!(values.0.is_empty());
	/// ```
	///
	/// A timestamp with time zone uses the PostgreSQL catalog name `timestamptz`:
	///
	/// ```rust
	/// use reinhardt_query::prelude::*;
	///
	/// let (sql, values) = Query::select()
	///     .expr(Expr::value("2026-01-02T03:04:05Z").cast_as("timestamptz"))
	///     .build(PostgresQueryBuilder);
	///
	/// assert_eq!(sql, r#"SELECT CAST($1 AS "timestamptz")"#);
	/// assert_eq!(values.0, vec![Value::from("2026-01-02T03:04:05Z")]);
	/// ```
	fn cast_as<T>(self, type_name: T) -> SimpleExpr
	where
		T: crate::types::IntoIden,
	{
		SimpleExpr::Cast(Box::new(self.into_simple_expr()), type_name.into_iden())
	}

	/// Cast to the selected backend's text type (P2 native/WASM parity).
	///
	/// Uses the closed built-in type grammar rather than an escaped user type
	/// identifier. MySQL renders `CHAR`; PostgreSQL and SQLite render `TEXT`.
	///
	/// ```
	/// use reinhardt_query::{Expr, ExprTrait, MySqlQueryBuilder, Query};
	/// let statement = Query::select().expr(Expr::val(7_i64).cast_as_text()).to_owned();
	/// let (sql, values) = MySqlQueryBuilder.build_select_checked(&statement).unwrap();
	/// assert_eq!(sql, "SELECT CAST(? AS CHAR)");
	/// assert_eq!(values.0, vec![7_i64.into()]);
	/// ```
	fn cast_as_text(self) -> SimpleExpr {
		SimpleExpr::TextCast(Box::new(self.into_simple_expr()))
	}

	/// Cast to the selected backend's signed integer type (P2).
	///
	/// PostgreSQL renders `BIGINT`, MySQL `SIGNED`, and SQLite `INTEGER`.
	/// The database determines conversion and overflow behavior for the source.
	fn cast_as_signed_integer(self) -> SimpleExpr {
		SimpleExpr::SignedIntegerCast(Box::new(self.into_simple_expr()))
	}

	/// AS ENUM expression (PostgreSQL).
	fn as_enum<T>(self, type_name: T) -> SimpleExpr
	where
		T: crate::types::IntoIden,
	{
		SimpleExpr::AsEnum(type_name.into_iden(), Box::new(self.into_simple_expr()))
	}
}

// Implement ExprTrait for SimpleExpr
impl ExprTrait for SimpleExpr {
	fn into_simple_expr(self) -> SimpleExpr {
		self
	}
}

// Implement ExprTrait for Expr
impl ExprTrait for super::expr::Expr {
	fn into_simple_expr(self) -> SimpleExpr {
		self.into_simple_expr()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::expr::Expr;
	use rstest::rstest;

	#[rstest]
	fn test_eq() {
		let expr = Expr::col("name").eq("Alice");
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Equal, _)));
	}

	#[rstest]
	fn test_ne() {
		let expr = Expr::col("name").ne("Bob");
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::NotEqual, _)));
	}

	#[rstest]
	fn test_lt() {
		let expr = Expr::col("age").lt(18);
		assert!(matches!(
			expr,
			SimpleExpr::Binary(_, BinOper::SmallerThan, _)
		));
	}

	#[rstest]
	fn test_lte() {
		let expr = Expr::col("age").lte(65);
		assert!(matches!(
			expr,
			SimpleExpr::Binary(_, BinOper::SmallerThanOrEqual, _)
		));
	}

	#[rstest]
	fn test_gt() {
		let expr = Expr::col("age").gt(18);
		assert!(matches!(
			expr,
			SimpleExpr::Binary(_, BinOper::GreaterThan, _)
		));
	}

	#[rstest]
	fn test_gte() {
		let expr = Expr::col("age").gte(18);
		assert!(matches!(
			expr,
			SimpleExpr::Binary(_, BinOper::GreaterThanOrEqual, _)
		));
	}

	#[rstest]
	fn test_is_null() {
		let expr = Expr::col("deleted_at").is_null();
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Is, _)));
	}

	#[rstest]
	fn test_is_not_null() {
		let expr = Expr::col("name").is_not_null();
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::IsNot, _)));
	}

	#[rstest]
	fn test_between() {
		let expr = Expr::col("age").between(18, 65);
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Between, _)));
	}

	#[rstest]
	fn test_not_between() {
		let expr = Expr::col("age").not_between(0, 17);
		assert!(matches!(
			expr,
			SimpleExpr::Binary(_, BinOper::NotBetween, _)
		));
	}

	#[rstest]
	fn test_is_in() {
		let expr = Expr::col("status").is_in(["active", "pending"]);
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::In, _)));
	}

	#[rstest]
	fn test_is_not_in() {
		let expr = Expr::col("status").is_not_in(["deleted", "banned"]);
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::NotIn, _)));
	}

	#[rstest]
	fn test_like() {
		let expr = Expr::col("name").like("%john%");
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Like, _)));
	}

	#[rstest]
	fn test_not_like() {
		let expr = Expr::col("name").not_like("%admin%");
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::NotLike, _)));
	}

	#[rstest]
	fn test_and() {
		let expr = Expr::col("active")
			.eq(true)
			.and(Expr::col("verified").eq(true));
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::And, _)));
	}

	#[rstest]
	fn test_or() {
		let expr = Expr::col("role")
			.eq("admin")
			.or(Expr::col("role").eq("moderator"));
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Or, _)));
	}

	#[rstest]
	fn test_not() {
		let expr = Expr::col("deleted").not();
		assert!(matches!(expr, SimpleExpr::Unary(UnOper::Not, _)));
	}

	#[rstest]
	fn test_add() {
		let expr = Expr::col("price").add(10);
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Add, _)));
	}

	#[rstest]
	fn test_sub() {
		let expr = Expr::col("quantity").sub(1);
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Sub, _)));
	}

	#[rstest]
	fn test_mul() {
		let expr = Expr::col("price").mul(Expr::col("quantity"));
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Mul, _)));
	}

	#[rstest]
	fn test_div() {
		let expr = Expr::col("total").div(Expr::col("count"));
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Div, _)));
	}

	#[rstest]
	fn test_modulo() {
		let expr = Expr::col("value").modulo(2);
		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::Mod, _)));
	}

	#[rstest]
	fn test_cast_as() {
		let expr = Expr::col("age").cast_as("TEXT");
		assert!(matches!(expr, SimpleExpr::Cast(_, _)));
	}

	#[rstest]
	fn test_chained_operations() {
		// Test complex expression chaining
		let expr = Expr::col("age")
			.gte(18)
			.and(Expr::col("active").eq(true))
			.and(Expr::col("verified").is_not_null());

		assert!(matches!(expr, SimpleExpr::Binary(_, BinOper::And, _)));
	}
}
