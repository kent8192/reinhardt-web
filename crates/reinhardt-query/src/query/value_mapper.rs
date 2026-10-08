//! Fallible value-to-expression adaptation before dialect rendering.

use super::{DeleteStatement, InsertSource, InsertStatement, SelectStatement, UpdateStatement};
use crate::{
	Value,
	expr::{ConditionExpression, SimpleExpr},
	types::{JoinOn, OrderExpr, OrderExprKind, TableRef, WindowStatement},
};

impl SelectStatement {
	/// Clone this statement and replace each value expression using `map`.
	///
	/// This adapts executor-specific parameter representations structurally,
	/// before checked dialect rendering assigns placeholders and collects Values.
	/// It visits nested conditions, subqueries, CTEs, joins, unions, ordering and
	/// window expressions. LIMIT/OFFSET values and window frame bounds retain
	/// their integer grammar and are not passed to the callback. Identifiers,
	/// keywords and custom SQL text remain unchanged; embedded expressions in
	/// `CustomWithExpr` are visited.
	///
	/// An array is one Value; its elements are the callback's responsibility.
	/// Returned expressions are not visited again. Callback order is not a bind
	/// order contract: always consume the final renderer's SQL/Values pair.
	/// An error leaves this original statement unchanged. Native and WASM
	/// behavior is identical (P2).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::{Expr, ExprTrait, PostgresQueryBuilder, Query, Value};
	/// let source = Query::select().expr(Expr::val("bound' $99")).to_owned();
	/// let adapted = source.try_map_value_expressions(|value| {
	///     Ok::<_, std::convert::Infallible>(Expr::val(value.clone()).cast_as_text())
	/// }).unwrap();
	/// let (sql, values) = PostgresQueryBuilder.build_select_checked(&adapted).unwrap();
	/// assert_eq!(sql, "SELECT CAST($1 AS TEXT)");
	/// assert_eq!(values.0, vec![Value::from("bound' $99")]);
	/// ```
	pub fn try_map_value_expressions<E>(
		&self,
		mut map: impl FnMut(&Value) -> Result<SimpleExpr, E>,
	) -> Result<Self, E> {
		let mut statement = self.clone();
		ValueMapper { map: &mut map }.select(&mut statement)?;
		Ok(statement)
	}
}

impl InsertStatement {
	/// Clone and adapt value expressions before rendering, including INSERT
	/// rows/source, conflict conditions and RETURNING expressions.
	///
	/// See [`SelectStatement::try_map_value_expressions`] for callback, error and
	/// traversal semantics. Native and WASM behavior is identical (P2).
	pub fn try_map_value_expressions<E>(
		&self,
		mut map: impl FnMut(&Value) -> Result<SimpleExpr, E>,
	) -> Result<Self, E> {
		let mut statement = self.clone();
		let mut mapper = ValueMapper { map: &mut map };
		if let Some(table) = &mut statement.table {
			mapper.table(table)?;
		}
		match &mut statement.source {
			InsertSource::Subquery(select) => mapper.select(select)?,
			InsertSource::Values(rows) => {
				if let Some(expressions) = &mut statement.expression_values {
					for row in expressions {
						mapper.expressions(row)?;
					}
				} else if !rows.is_empty() {
					statement.expression_values = Some(
						rows.iter()
							.map(|row| row.iter().map(&mut *mapper.map).collect())
							.collect::<Result<_, E>>()?,
					);
				}
			}
		}
		if let Some(conflict) = &mut statement.on_conflict
			&& let Some(condition) = &mut conflict.action_condition
		{
			mapper.expression(condition)?;
		}
		if let Some(expressions) = &mut statement.returning_exprs {
			mapper.expressions(expressions)?;
		}
		Ok(statement)
	}
}

impl UpdateStatement {
	/// Clone and adapt SET, WHERE, subquery and RETURNING value expressions.
	///
	/// See [`SelectStatement::try_map_value_expressions`] for callback, error and
	/// traversal semantics. Native and WASM behavior is identical (P2).
	pub fn try_map_value_expressions<E>(
		&self,
		mut map: impl FnMut(&Value) -> Result<SimpleExpr, E>,
	) -> Result<Self, E> {
		let mut statement = self.clone();
		let mut mapper = ValueMapper { map: &mut map };
		if let Some(table) = &mut statement.table {
			mapper.table(table)?;
		}
		for (_, expression) in &mut statement.values {
			mapper.expression(expression)?;
		}
		mapper.conditions(&mut statement.r#where.conditions)?;
		if let Some(expressions) = &mut statement.returning_exprs {
			mapper.expressions(expressions)?;
		}
		Ok(statement)
	}
}

impl DeleteStatement {
	/// Clone and adapt WHERE, subquery and RETURNING value expressions.
	///
	/// See [`SelectStatement::try_map_value_expressions`] for callback, error and
	/// traversal semantics. Native and WASM behavior is identical (P2).
	pub fn try_map_value_expressions<E>(
		&self,
		mut map: impl FnMut(&Value) -> Result<SimpleExpr, E>,
	) -> Result<Self, E> {
		let mut statement = self.clone();
		let mut mapper = ValueMapper { map: &mut map };
		if let Some(table) = &mut statement.table {
			mapper.table(table)?;
		}
		mapper.conditions(&mut statement.r#where.conditions)?;
		if let Some(expressions) = &mut statement.returning_exprs {
			mapper.expressions(expressions)?;
		}
		Ok(statement)
	}
}

struct ValueMapper<'a, E> {
	map: &'a mut dyn FnMut(&Value) -> Result<SimpleExpr, E>,
}

impl<E> ValueMapper<'_, E> {
	fn expression(&mut self, expression: &mut SimpleExpr) -> Result<(), E> {
		match expression {
			SimpleExpr::Value(value) => *expression = (self.map)(value)?,
			SimpleExpr::Unary(_, inner)
			| SimpleExpr::AsEnum(_, inner)
			| SimpleExpr::ExprAlias(inner, _)
			| SimpleExpr::Cast(inner, _)
			| SimpleExpr::Grouped(inner)
			| SimpleExpr::TextCast(inner)
			| SimpleExpr::SignedIntegerCast(inner)
			| SimpleExpr::PgExtractEpoch(inner)
			| SimpleExpr::TemporalTrunc { expr: inner, .. }
			| SimpleExpr::WindowNamed { func: inner, .. } => self.expression(inner)?,
			SimpleExpr::Binary(left, _, right)
			| SimpleExpr::LikeWithEscape(left, right)
			| SimpleExpr::InsensitiveLikeWithEscape(left, right) => {
				self.expression(left)?;
				self.expression(right)?;
			}
			SimpleExpr::FunctionCall(_, expressions)
			| SimpleExpr::Tuple(expressions)
			| SimpleExpr::CustomWithExpr(_, expressions) => self.expressions(expressions)?,
			SimpleExpr::SubQuery(_, statement) => self.select(statement)?,
			SimpleExpr::Case(case) => {
				for (condition, result) in &mut case.when_clauses {
					self.expression(condition)?;
					self.expression(result)?;
				}
				if let Some(expression) = &mut case.else_clause {
					self.expression(expression)?;
				}
			}
			SimpleExpr::Window { func, window } => {
				self.expression(func)?;
				self.window(window)?;
			}
			SimpleExpr::MySqlLastInsertId(Some(inner)) => self.expression(inner)?,
			SimpleExpr::Column(_)
			| SimpleExpr::TableColumn(_, _)
			| SimpleExpr::Custom(_)
			| SimpleExpr::Constant(_)
			| SimpleExpr::Asterisk
			| SimpleExpr::MySqlLastInsertId(None) => {}
		}
		Ok(())
	}

	fn expressions(&mut self, expressions: &mut [SimpleExpr]) -> Result<(), E> {
		for expression in expressions {
			self.expression(expression)?;
		}
		Ok(())
	}

	fn conditions(&mut self, conditions: &mut [ConditionExpression]) -> Result<(), E> {
		for condition in conditions {
			match condition {
				ConditionExpression::SimpleExpr(expression) => self.expression(expression)?,
				ConditionExpression::Condition(condition) => {
					self.conditions(&mut condition.conditions)?;
				}
			}
		}
		Ok(())
	}

	fn table(&mut self, table: &mut TableRef) -> Result<(), E> {
		if let TableRef::SubQuery(statement, _) = table {
			self.select(statement)?;
		}
		Ok(())
	}

	fn orders(&mut self, orders: &mut [OrderExpr]) -> Result<(), E> {
		for order in orders {
			if let OrderExprKind::Expr(expression) = &mut order.expr {
				self.expression(expression)?;
			}
		}
		Ok(())
	}

	fn window(&mut self, window: &mut WindowStatement) -> Result<(), E> {
		self.expressions(&mut window.partition_by)?;
		self.orders(&mut window.order_by)
	}

	fn select(&mut self, statement: &mut SelectStatement) -> Result<(), E> {
		for cte in &mut statement.ctes {
			self.select(&mut cte.query)?;
		}
		for select in &mut statement.selects {
			self.expression(&mut select.expr)?;
		}
		for table in &mut statement.from {
			self.table(table)?;
		}
		for join in &mut statement.join {
			self.table(&mut join.table)?;
			if let Some(JoinOn::Condition(condition)) = &mut join.on {
				self.conditions(&mut condition.conditions)?;
			}
		}
		self.conditions(&mut statement.r#where.conditions)?;
		self.expressions(&mut statement.groups)?;
		self.conditions(&mut statement.having.conditions)?;
		for (_, union) in &mut statement.unions {
			self.select(union)?;
		}
		self.orders(&mut statement.orders)?;
		for (_, window) in &mut statement.windows {
			self.window(window)?;
		}
		if let Some(lock) = &mut statement.lock {
			for table in &mut lock.tables {
				self.table(table)?;
			}
		}
		Ok(())
	}
}
