//! PostgreSQL query builder backend
//!
//! This module implements the SQL generation backend for PostgreSQL.

use super::{QueryBuilder, SqlWriter};
use crate::{
	expr::{Condition, SimpleExpr},
	query::{DeleteStatement, InsertStatement, SelectStatement, UpdateStatement},
	types::{BinOper, ColumnRef, TableRef},
	value::Values,
};

/// PostgreSQL query builder
///
/// This struct implements SQL generation for PostgreSQL, using the following conventions:
/// - Identifiers: Double quotes (`"table_name"`)
/// - Placeholders: Numbered (`$1`, `$2`, ...)
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::backend::{PostgresQueryBuilder, QueryBuilder};
/// use reinhardt_query::prelude::*;
///
/// let builder = PostgresQueryBuilder::new();
/// let stmt = Query::select()
///     .column("id")
///     .from("users");
///
/// let (sql, values) = builder.build_select(&stmt);
/// // sql: SELECT "id" FROM "users"
/// ```
#[derive(Debug, Clone, Default)]
pub struct PostgresQueryBuilder;

impl PostgresQueryBuilder {
	/// Create a new PostgreSQL query builder
	pub fn new() -> Self {
		Self
	}

	/// Escape an identifier for PostgreSQL
	///
	/// PostgreSQL uses double quotes for identifiers.
	///
	/// # Arguments
	///
	/// * `ident` - The identifier to escape
	///
	/// # Returns
	///
	/// The escaped identifier (e.g., `"user"`)
	fn escape_iden(&self, ident: &str) -> String {
		// Escape double quotes within the identifier
		let escaped = ident.replace('"', "\"\"");
		format!("\"{}\"", escaped)
	}

	/// Format a placeholder for PostgreSQL
	///
	/// PostgreSQL uses numbered placeholders ($1, $2, ...).
	///
	/// # Arguments
	///
	/// * `index` - The parameter index (1-based)
	///
	/// # Returns
	///
	/// The placeholder string (e.g., `$1`)
	fn placeholder(&self, index: usize) -> String {
		format!("${}", index)
	}

	/// Write a table reference
	fn write_table_ref(&self, writer: &mut SqlWriter, table_ref: &TableRef) {
		match table_ref {
			TableRef::Table(iden) => {
				writer.push_identifier(&iden.to_string(), |s| self.escape_iden(s));
			}
			TableRef::SchemaTable(schema, table) => {
				writer.push_identifier(&schema.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
			}
			TableRef::DatabaseSchemaTable(db, schema, table) => {
				writer.push_identifier(&db.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&schema.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
			}
			TableRef::TableAlias(table, alias) => {
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
				writer.push_keyword("AS");
				writer.push_space();
				writer.push_identifier(&alias.to_string(), |s| self.escape_iden(s));
			}
			TableRef::SchemaTableAlias(schema, table, alias) => {
				writer.push_identifier(&schema.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
				writer.push_keyword("AS");
				writer.push_space();
				writer.push_identifier(&alias.to_string(), |s| self.escape_iden(s));
			}
		}
	}

	/// Write a column reference
	fn write_column_ref(&self, writer: &mut SqlWriter, col_ref: &ColumnRef) {
		match col_ref {
			ColumnRef::Column(iden) => {
				writer.push_identifier(&iden.to_string(), |s| self.escape_iden(s));
			}
			ColumnRef::TableColumn(table, col) => {
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&col.to_string(), |s| self.escape_iden(s));
			}
			ColumnRef::SchemaTableColumn(schema, table, col) => {
				writer.push_identifier(&schema.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&col.to_string(), |s| self.escape_iden(s));
			}
			ColumnRef::Asterisk => {
				writer.push("*");
			}
			ColumnRef::TableAsterisk(table) => {
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
				writer.push(".*");
			}
		}
	}

	/// Write a simple expression
	fn write_simple_expr(&self, writer: &mut SqlWriter, expr: &SimpleExpr) {
		match expr {
			SimpleExpr::Column(col_ref) => {
				self.write_column_ref(writer, col_ref);
			}
			SimpleExpr::Value(value) => {
				writer.push_value(value.clone(), |i| self.placeholder(i));
			}
			SimpleExpr::Binary(left, op, right) => match (op, right.as_ref()) {
				(BinOper::Between | BinOper::NotBetween, SimpleExpr::Tuple(items))
					if items.len() == 2 =>
				{
					self.write_simple_expr(writer, left);
					writer.push_space();
					writer.push(op.as_str());
					writer.push_space();
					self.write_simple_expr(writer, &items[0]);
					writer.push(" AND ");
					self.write_simple_expr(writer, &items[1]);
				}
				(BinOper::In | BinOper::NotIn, SimpleExpr::Tuple(items)) => {
					self.write_simple_expr(writer, left);
					writer.push_space();
					writer.push(op.as_str());
					writer.push(" (");
					writer.push_list(items, ", ", |w, item| {
						self.write_simple_expr(w, item);
					});
					writer.push(")");
				}
				_ => {
					self.write_simple_expr(writer, left);
					writer.push_space();
					writer.push(op.as_str());
					writer.push_space();
					self.write_simple_expr(writer, right);
				}
			},
			SimpleExpr::Unary(op, expr) => {
				writer.push(op.as_str());
				writer.push_space();
				self.write_simple_expr(writer, expr);
			}
			SimpleExpr::FunctionCall(func_name, args) => {
				writer.push(&func_name.to_string());
				writer.push("(");
				writer.push_list(args, ", ", |w, arg| {
					self.write_simple_expr(w, arg);
				});
				writer.push(")");
			}
			SimpleExpr::Constant(val) => {
				writer.push(val.as_str());
			}
			SimpleExpr::SubQuery(op, select_stmt) => {
				use crate::expr::SubQueryOper;

				// Write the operator keyword if present
				if let Some(operator) = op {
					match operator {
						SubQueryOper::Exists => {
							writer.push("EXISTS");
							writer.push_space();
						}
						SubQueryOper::NotExists => {
							writer.push("NOT EXISTS");
							writer.push_space();
						}
						SubQueryOper::In | SubQueryOper::NotIn => {
							// These are handled in Binary expressions
						}
						SubQueryOper::All => {
							writer.push("ALL");
							writer.push_space();
						}
						SubQueryOper::Any => {
							writer.push("ANY");
							writer.push_space();
						}
						SubQueryOper::Some => {
							writer.push("SOME");
							writer.push_space();
						}
					}
				}

				// Write the subquery wrapped in parentheses
				writer.push("(");

				// Recursively build the subquery
				let (subquery_sql, subquery_values) = self.build_select(select_stmt);

				// Adjust placeholders for PostgreSQL ($1, $2, ... -> $N, $N+1, ...)
				let offset = writer.param_index() - 1;
				let adjusted_sql = if offset > 0 {
					let mut sql = subquery_sql;
					for i in (1..=subquery_values.len()).rev() {
						sql = sql.replace(&format!("${}", i), &format!("${}", i + offset));
					}
					sql
				} else {
					subquery_sql
				};

				writer.push(&adjusted_sql);
				writer.push(")");

				// Merge the values from the subquery
				writer.append_values(&subquery_values);
			}
			SimpleExpr::Window { func, window } => {
				// Write the function
				self.write_simple_expr(writer, func);
				writer.push_space();
				writer.push_keyword("OVER");
				writer.push_space();
				writer.push("(");
				self.write_window_statement(writer, window);
				writer.push(")");
			}
			SimpleExpr::WindowNamed { func, name } => {
				// Write the function
				self.write_simple_expr(writer, func);
				writer.push_space();
				writer.push_keyword("OVER");
				writer.push_space();
				writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
			}
			SimpleExpr::Tuple(items) => {
				writer.push("(");
				writer.push_list(items, ", ", |w, item| {
					self.write_simple_expr(w, item);
				});
				writer.push(")");
			}
			SimpleExpr::Case(case) => {
				writer.push_keyword("CASE");
				for (condition, result) in &case.when_clauses {
					writer.push_space();
					writer.push_keyword("WHEN");
					writer.push_space();
					self.write_simple_expr(writer, condition);
					writer.push_space();
					writer.push_keyword("THEN");
					writer.push_space();
					self.write_simple_expr(writer, result);
				}
				if let Some(else_result) = &case.else_clause {
					writer.push_space();
					writer.push_keyword("ELSE");
					writer.push_space();
					self.write_simple_expr(writer, else_result);
				}
				writer.push_space();
				writer.push_keyword("END");
			}
			_ => {
				writer.push("(EXPR)");
			}
		}
	}

	/// Write a condition
	fn write_condition(&self, writer: &mut SqlWriter, condition: &Condition) {
		use crate::expr::ConditionType;

		if condition.conditions.is_empty() {
			return;
		}

		if condition.negate {
			writer.push("NOT ");
		}

		if condition.conditions.len() == 1 {
			self.write_condition_expr(writer, &condition.conditions[0]);
			return;
		}

		writer.push("(");
		let separator = match condition.condition_type {
			ConditionType::All => " AND ",
			ConditionType::Any => " OR ",
		};
		writer.push_list(&condition.conditions, separator, |w, cond_expr| {
			self.write_condition_expr(w, cond_expr);
		});
		writer.push(")");
	}

	/// Write a condition expression
	fn write_condition_expr(
		&self,
		writer: &mut SqlWriter,
		cond_expr: &crate::expr::ConditionExpression,
	) {
		use crate::expr::ConditionExpression;

		match cond_expr {
			ConditionExpression::Condition(cond) => {
				self.write_condition(writer, cond);
			}
			ConditionExpression::SimpleExpr(expr) => {
				self.write_simple_expr(writer, expr);
			}
		}
	}

	/// Write a JOIN expression
	fn write_join_expr(&self, writer: &mut SqlWriter, join: &crate::types::JoinExpr) {
		use crate::types::JoinOn;

		// JOIN type
		writer.push_keyword(join.join.as_str());
		writer.push_space();

		// Table
		self.write_table_ref(writer, &join.table);

		// ON or USING clause
		if let Some(on) = &join.on {
			match on {
				JoinOn::Columns(pair) => {
					writer.push_keyword("ON");
					writer.push_space();
					self.write_column_spec(writer, &pair.left);
					writer.push(" = ");
					self.write_column_spec(writer, &pair.right);
				}
				JoinOn::Condition(cond) => {
					writer.push_keyword("ON");
					writer.push_space();
					self.write_condition(writer, cond);
				}
				JoinOn::Using(cols) => {
					writer.push_keyword("USING");
					writer.push_space();
					writer.push("(");
					writer.push_list(cols, ", ", |w, col| {
						w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
					});
					writer.push(")");
				}
			}
		}
	}

	/// Write a column specification (for JOIN ON conditions)
	fn write_column_spec(&self, writer: &mut SqlWriter, spec: &crate::types::ColumnSpec) {
		match spec {
			crate::types::ColumnSpec::Column(iden) => {
				writer.push_identifier(&iden.to_string(), |s| self.escape_iden(s));
			}
			crate::types::ColumnSpec::TableColumn(table, col) => {
				writer.push_identifier(&table.to_string(), |s| self.escape_iden(s));
				writer.push(".");
				writer.push_identifier(&col.to_string(), |s| self.escape_iden(s));
			}
		}
	}

	/// Write a window statement (PARTITION BY, ORDER BY, frame clause)
	fn write_window_statement(
		&self,
		writer: &mut SqlWriter,
		window: &crate::types::WindowStatement,
	) {
		// PARTITION BY clause
		if !window.partition_by.is_empty() {
			writer.push_keyword("PARTITION BY");
			writer.push_space();
			writer.push_list(&window.partition_by, ", ", |w, expr| {
				self.write_simple_expr(w, expr);
			});
			writer.push_space();
		}

		// ORDER BY clause
		if !window.order_by.is_empty() {
			writer.push_keyword("ORDER BY");
			writer.push_space();
			writer.push_list(&window.order_by, ", ", |w, order_expr| {
				use crate::types::OrderExprKind;
				match &order_expr.expr {
					OrderExprKind::Column(iden) => {
						w.push_identifier(&iden.to_string(), |s| self.escape_iden(s));
					}
					OrderExprKind::TableColumn(table, col) => {
						w.push_identifier(&table.to_string(), |s| self.escape_iden(s));
						w.push(".");
						w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
					}
					OrderExprKind::Expr(expr) => {
						self.write_simple_expr(w, expr);
					}
				}
				match order_expr.order {
					crate::types::Order::Asc => {
						w.push_keyword("ASC");
					}
					crate::types::Order::Desc => {
						w.push_keyword("DESC");
					}
				}
				if let Some(nulls) = order_expr.nulls {
					w.push_space();
					w.push(nulls.as_str());
				}
			});
			writer.push_space();
		}

		// Frame clause
		if let Some(frame) = &window.frame {
			self.write_frame_clause(writer, frame);
		}
	}

	/// Write a frame clause (ROWS/RANGE/GROUPS BETWEEN ... AND ...)
	fn write_frame_clause(&self, writer: &mut SqlWriter, frame: &crate::types::FrameClause) {
		use crate::types::FrameType;

		// Frame type (ROWS, RANGE, GROUPS)
		match frame.frame_type {
			FrameType::Rows => writer.push_keyword("ROWS"),
			FrameType::Range => writer.push_keyword("RANGE"),
			FrameType::Groups => writer.push_keyword("GROUPS"),
		}
		writer.push_space();

		// Frame specification
		if let Some(end) = &frame.end {
			writer.push_keyword("BETWEEN");
			writer.push_space();
			self.write_frame_boundary(writer, &frame.start);
			writer.push_keyword("AND");
			writer.push_space();
			self.write_frame_boundary(writer, end);
		} else {
			self.write_frame_boundary(writer, &frame.start);
		}
	}

	/// Write a frame boundary (UNBOUNDED PRECEDING, CURRENT ROW, etc.)
	fn write_frame_boundary(&self, writer: &mut SqlWriter, frame: &crate::types::Frame) {
		use crate::types::Frame;
		match frame {
			Frame::UnboundedPreceding => writer.push("UNBOUNDED PRECEDING"),
			Frame::Preceding(n) => {
				writer.push(&n.to_string());
				writer.push(" PRECEDING");
			}
			Frame::CurrentRow => writer.push("CURRENT ROW"),
			Frame::Following(n) => {
				writer.push(&n.to_string());
				writer.push(" FOLLOWING");
			}
			Frame::UnboundedFollowing => writer.push("UNBOUNDED FOLLOWING"),
		}
	}
}

impl QueryBuilder for PostgresQueryBuilder {
	fn build_select(&self, stmt: &SelectStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// WITH clause (Common Table Expressions)
		if !stmt.ctes.is_empty() {
			// Check if any CTE is recursive
			let has_recursive = stmt.ctes.iter().any(|cte| cte.recursive);

			// Write WITH [RECURSIVE]
			writer.push_keyword("WITH");
			writer.push_space();
			if has_recursive {
				writer.push_keyword("RECURSIVE");
				writer.push_space();
			}

			// Write each CTE
			writer.push_list(&stmt.ctes, ", ", |w, cte| {
				// CTE name
				w.push_identifier(&cte.name.to_string(), |s| self.escape_iden(s));
				w.push_space();
				w.push_keyword("AS");
				w.push_space();
				w.push("(");

				// Recursively build the CTE query
				let (cte_sql, cte_values) = self.build_select(&cte.query);

				// Adjust placeholders for PostgreSQL ($1, $2, ... -> $N, $N+1, ...)
				let offset = w.param_index() - 1;
				let adjusted_sql = if offset > 0 {
					let mut sql = cte_sql;
					for i in (1..=cte_values.len()).rev() {
						sql = sql.replace(&format!("${}", i), &format!("${}", i + offset));
					}
					sql
				} else {
					cte_sql
				};

				w.push(&adjusted_sql);
				w.push(")");

				// Merge the values from the CTE query
				w.append_values(&cte_values);
			});

			writer.push_space();
		}

		// SELECT clause
		writer.push("SELECT");
		writer.push_space();

		// DISTINCT clause
		if let Some(distinct) = &stmt.distinct {
			use crate::query::SelectDistinct;
			match distinct {
				SelectDistinct::All => {
					// SELECT ALL - explicit but not required in PostgreSQL
				}
				SelectDistinct::Distinct => {
					writer.push_keyword("DISTINCT");
					writer.push_space();
				}
				SelectDistinct::DistinctRow => {
					panic!("PostgreSQL does not support DISTINCT ROW. Use DISTINCT instead.");
				}
				SelectDistinct::DistinctOn(cols) => {
					writer.push_keyword("DISTINCT ON");
					writer.push_space();
					writer.push("(");
					writer.push_list(cols, ", ", |w, col_ref| {
						self.write_column_ref(w, col_ref);
					});
					writer.push(")");
					writer.push_space();
				}
			}
		}

		if stmt.selects.is_empty() {
			writer.push("*");
		} else {
			writer.push_list(&stmt.selects, ", ", |w, select_expr| {
				self.write_simple_expr(w, &select_expr.expr);
				if let Some(alias) = &select_expr.alias {
					w.push_keyword("AS");
					w.push_space();
					w.push_identifier(&alias.to_string(), |s| self.escape_iden(s));
				}
			});
		}

		// FROM clause
		if !stmt.from.is_empty() {
			writer.push_keyword("FROM");
			writer.push_space();
			writer.push_list(&stmt.from, ", ", |w, table_ref| {
				self.write_table_ref(w, table_ref);
			});
		}

		// JOIN clauses
		for join in &stmt.join {
			writer.push_space();
			self.write_join_expr(&mut writer, join);
		}

		// WHERE clause
		if !stmt.r#where.is_empty() {
			writer.push_keyword("WHERE");
			writer.push_space();
			// Write all conditions in the ConditionHolder with AND
			writer.push_list(&stmt.r#where.conditions, " AND ", |w, cond_expr| {
				self.write_condition_expr(w, cond_expr);
			});
		}

		// GROUP BY clause
		if !stmt.groups.is_empty() {
			writer.push_keyword("GROUP BY");
			writer.push_space();
			writer.push_list(&stmt.groups, ", ", |w, expr| {
				self.write_simple_expr(w, expr);
			});
		}

		// HAVING clause
		if !stmt.having.conditions.is_empty() {
			writer.push_keyword("HAVING");
			writer.push_space();
			// Write all conditions in the ConditionHolder with AND
			writer.push_list(&stmt.having.conditions, " AND ", |w, cond_expr| {
				self.write_condition_expr(w, cond_expr);
			});
		}

		// ORDER BY clause
		if !stmt.orders.is_empty() {
			writer.push_keyword("ORDER BY");
			writer.push_space();
			writer.push_list(&stmt.orders, ", ", |w, order_expr| {
				use crate::types::OrderExprKind;
				match &order_expr.expr {
					OrderExprKind::Column(iden) => {
						w.push_identifier(&iden.to_string(), |s| self.escape_iden(s));
					}
					OrderExprKind::TableColumn(table, col) => {
						w.push_identifier(&table.to_string(), |s| self.escape_iden(s));
						w.push(".");
						w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
					}
					OrderExprKind::Expr(expr) => {
						self.write_simple_expr(w, expr);
					}
				}
				match order_expr.order {
					crate::types::Order::Asc => {
						w.push_keyword("ASC");
					}
					crate::types::Order::Desc => {
						w.push_keyword("DESC");
					}
				}
				if let Some(nulls) = order_expr.nulls {
					w.push_space();
					w.push(nulls.as_str());
				}
			});
		}

		// WINDOW clause
		if !stmt.windows.is_empty() {
			writer.push_keyword("WINDOW");
			writer.push_space();
			writer.push_list(&stmt.windows, ", ", |w, (name, window)| {
				w.push_identifier(&name.to_string(), |s| self.escape_iden(s));
				w.push_space();
				w.push_keyword("AS");
				w.push_space();
				w.push("(");
				self.write_window_statement(w, window);
				w.push(")");
			});
		}

		// LIMIT clause
		if let Some(limit) = &stmt.limit {
			writer.push_keyword("LIMIT");
			writer.push_space();
			writer.push_value(limit.clone(), |i| self.placeholder(i));
		}

		// OFFSET clause
		if let Some(offset) = &stmt.offset {
			writer.push_keyword("OFFSET");
			writer.push_space();
			writer.push_value(offset.clone(), |i| self.placeholder(i));
		}

		// UNION/INTERSECT/EXCEPT clauses
		for (union_type, union_stmt) in &stmt.unions {
			writer.push_space();
			use crate::query::UnionType;
			match union_type {
				UnionType::Distinct => {
					writer.push_keyword("UNION");
				}
				UnionType::All => {
					writer.push_keyword("UNION ALL");
				}
				UnionType::Intersect => {
					writer.push_keyword("INTERSECT");
				}
				UnionType::Except => {
					writer.push_keyword("EXCEPT");
				}
			}
			writer.push_space();

			// Recursively build the union query
			let (mut union_sql, union_values) = self.build_select(union_stmt);

			// Adjust placeholders for PostgreSQL ($1, $2, ... -> $N, $N+1, ...)
			let offset = writer.param_index() - 1;
			if offset > 0 {
				for i in (1..=union_values.len()).rev() {
					union_sql = union_sql.replace(&format!("${}", i), &format!("${}", i + offset));
				}
			}

			// Append the union SQL (wrapped in parentheses if it has unions itself)
			if !union_stmt.unions.is_empty() {
				writer.push("(");
				writer.push(&union_sql);
				writer.push(")");
			} else {
				writer.push(&union_sql);
			}

			// Merge the values from the union query
			writer.append_values(&union_values);
		}

		writer.finish()
	}

	fn build_insert(&self, stmt: &InsertStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// INSERT INTO clause
		writer.push("INSERT INTO");
		writer.push_space();

		if let Some(table) = &stmt.table {
			self.write_table_ref(&mut writer, table);
		} else {
			// No table specified - this should not happen in valid SQL
			writer.push("(NO_TABLE)");
		}

		// Column list
		if !stmt.columns.is_empty() {
			writer.push_space();
			writer.push("(");
			writer.push_list(&stmt.columns, ", ", |w, col| {
				w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
			});
			writer.push(")");
		}

		// VALUES clause
		if !stmt.values.is_empty() {
			writer.push_keyword("VALUES");
			writer.push_space();

			writer.push_list(&stmt.values, ", ", |w, row| {
				w.push("(");
				w.push_list(row, ", ", |w2, value| {
					w2.push_value(value.clone(), |i| self.placeholder(i));
				});
				w.push(")");
			});
		}

		// RETURNING clause (PostgreSQL specific)
		if let Some(returning) = &stmt.returning {
			writer.push_keyword("RETURNING");
			writer.push_space();

			use crate::query::ReturningClause;
			match returning {
				ReturningClause::All => {
					writer.push("*");
				}
				ReturningClause::Columns(cols) => {
					writer.push_list(cols, ", ", |w, col| {
						self.write_column_ref(w, col);
					});
				}
			}
		}

		writer.finish()
	}

	fn build_update(&self, stmt: &UpdateStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// UPDATE clause
		writer.push("UPDATE");
		writer.push_space();

		if let Some(table) = &stmt.table {
			self.write_table_ref(&mut writer, table);
		} else {
			writer.push("(NO_TABLE)");
		}

		// SET clause
		if !stmt.values.is_empty() {
			writer.push_keyword("SET");
			writer.push_space();

			writer.push_list(&stmt.values, ", ", |w, (col, value)| {
				w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
				w.push(" = ");
				w.push_value(value.clone(), |i| self.placeholder(i));
			});
		}

		// WHERE clause
		if !stmt.r#where.is_empty() {
			writer.push_keyword("WHERE");
			writer.push_space();
			writer.push_list(&stmt.r#where.conditions, " AND ", |w, cond_expr| {
				self.write_condition_expr(w, cond_expr);
			});
		}

		// RETURNING clause (PostgreSQL specific)
		if let Some(returning) = &stmt.returning {
			writer.push_keyword("RETURNING");
			writer.push_space();

			use crate::query::ReturningClause;
			match returning {
				ReturningClause::All => {
					writer.push("*");
				}
				ReturningClause::Columns(cols) => {
					writer.push_list(cols, ", ", |w, col| {
						self.write_column_ref(w, col);
					});
				}
			}
		}

		writer.finish()
	}

	fn build_delete(&self, stmt: &DeleteStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// DELETE FROM clause
		writer.push("DELETE FROM");
		writer.push_space();

		if let Some(table) = &stmt.table {
			self.write_table_ref(&mut writer, table);
		} else {
			writer.push("(NO_TABLE)");
		}

		// WHERE clause
		if !stmt.r#where.is_empty() {
			writer.push_keyword("WHERE");
			writer.push_space();
			writer.push_list(&stmt.r#where.conditions, " AND ", |w, cond_expr| {
				self.write_condition_expr(w, cond_expr);
			});
		}

		// RETURNING clause (PostgreSQL specific)
		if let Some(returning) = &stmt.returning {
			writer.push_keyword("RETURNING");
			writer.push_space();

			use crate::query::ReturningClause;
			match returning {
				ReturningClause::All => {
					writer.push("*");
				}
				ReturningClause::Columns(cols) => {
					writer.push_list(cols, ", ", |w, col| {
						self.write_column_ref(w, col);
					});
				}
			}
		}

		writer.finish()
	}

	fn build_grant(&self, stmt: &crate::dcl::GrantStatement) -> (String, Values) {
		use crate::dcl::Grantee;

		let mut writer = SqlWriter::new();

		// GRANT keyword
		writer.push("GRANT");
		writer.push_space();

		// Privileges
		writer.push_list(&stmt.privileges, ", ", |w, privilege| {
			w.push(privilege.as_sql());
		});

		// ON clause
		writer.push_keyword("ON");
		writer.push_space();
		writer.push(stmt.object_type.as_sql());
		writer.push_space();

		// Objects
		writer.push_list(&stmt.objects, ", ", |w, obj| {
			w.push_identifier(&obj.to_string(), |s| self.escape_iden(s));
		});

		// TO clause
		writer.push_keyword("TO");
		writer.push_space();

		// Grantees
		writer.push_list(&stmt.grantees, ", ", |w, grantee| {
			match grantee {
				Grantee::Role(name) => {
					w.push_identifier(name, |s| self.escape_iden(s));
				}
				Grantee::User(_, _) => {
					// MySQL-specific, not supported in PostgreSQL
					w.push_identifier("(UNSUPPORTED_USER)", |s| self.escape_iden(s));
				}
				Grantee::Public => {
					w.push("PUBLIC");
				}
				Grantee::CurrentRole => {
					w.push("CURRENT_ROLE");
				}
				Grantee::CurrentUser => {
					w.push("CURRENT_USER");
				}
				Grantee::SessionUser => {
					w.push("SESSION_USER");
				}
			}
		});

		// WITH GRANT OPTION
		if stmt.with_grant_option {
			writer.push_keyword("WITH GRANT OPTION");
		}

		// GRANTED BY clause
		if let Some(grantor) = &stmt.granted_by {
			writer.push_keyword("GRANTED BY");
			writer.push_space();
			match grantor {
				Grantee::Role(name) => {
					writer.push_identifier(name, |s| self.escape_iden(s));
				}
				Grantee::User(_, _) => {
					writer.push_identifier("(UNSUPPORTED_USER)", |s| self.escape_iden(s));
				}
				Grantee::Public => {
					writer.push("PUBLIC");
				}
				Grantee::CurrentRole => {
					writer.push("CURRENT_ROLE");
				}
				Grantee::CurrentUser => {
					writer.push("CURRENT_USER");
				}
				Grantee::SessionUser => {
					writer.push("SESSION_USER");
				}
			}
		}

		writer.finish()
	}

	fn build_revoke(&self, stmt: &crate::dcl::RevokeStatement) -> (String, Values) {
		use crate::dcl::Grantee;

		let mut writer = SqlWriter::new();

		// REVOKE keyword
		writer.push("REVOKE");
		writer.push_space();

		// GRANT OPTION FOR (if specified)
		if stmt.grant_option_for {
			writer.push("GRANT OPTION FOR");
			writer.push_space();
		}

		// Privileges
		writer.push_list(&stmt.privileges, ", ", |w, privilege| {
			w.push(privilege.as_sql());
		});

		// ON clause
		writer.push_keyword("ON");
		writer.push_space();
		writer.push(stmt.object_type.as_sql());
		writer.push_space();

		// Objects
		writer.push_list(&stmt.objects, ", ", |w, obj| {
			w.push_identifier(&obj.to_string(), |s| self.escape_iden(s));
		});

		// FROM clause
		writer.push_keyword("FROM");
		writer.push_space();

		// Grantees
		writer.push_list(&stmt.grantees, ", ", |w, grantee| {
			match grantee {
				Grantee::Role(name) => {
					w.push_identifier(name, |s| self.escape_iden(s));
				}
				Grantee::User(_, _) => {
					// MySQL-specific, not supported in PostgreSQL
					w.push_identifier("(UNSUPPORTED_USER)", |s| self.escape_iden(s));
				}
				Grantee::Public => {
					w.push("PUBLIC");
				}
				Grantee::CurrentRole => {
					w.push("CURRENT_ROLE");
				}
				Grantee::CurrentUser => {
					w.push("CURRENT_USER");
				}
				Grantee::SessionUser => {
					w.push("SESSION_USER");
				}
			}
		});

		// CASCADE / RESTRICT
		if stmt.cascade {
			writer.push_keyword("CASCADE");
		}

		writer.finish()
	}

	fn build_grant_role(&self, stmt: &crate::dcl::GrantRoleStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// GRANT keyword
		writer.push("GRANT");
		writer.push_space();

		// Roles (comma-separated list)
		writer.push_list(&stmt.roles, ", ", |w, role| {
			w.push_identifier(role, |s| self.escape_iden(s));
		});

		// TO clause
		writer.push_keyword("TO");
		writer.push_space();

		// Grantees
		writer.push_list(&stmt.grantees, ", ", |w, grantee| {
			w.push(Self::format_role_specification(grantee));
		});

		// WITH ADMIN OPTION
		if stmt.with_admin_option {
			writer.push_keyword("WITH ADMIN OPTION");
		}

		// GRANTED BY
		if let Some(ref grantor) = stmt.granted_by {
			writer.push_keyword("GRANTED BY");
			writer.push_space();
			writer.push(Self::format_role_specification(grantor));
		}

		writer.finish()
	}

	fn build_revoke_role(&self, stmt: &crate::dcl::RevokeRoleStatement) -> (String, Values) {
		use crate::dcl::DropBehavior;

		let mut writer = SqlWriter::new();

		// REVOKE keyword
		writer.push("REVOKE");
		writer.push_space();

		// ADMIN OPTION FOR
		if stmt.admin_option_for {
			writer.push("ADMIN OPTION FOR");
			writer.push_space();
		}

		// Roles (comma-separated list)
		writer.push_list(&stmt.roles, ", ", |w, role| {
			w.push_identifier(role, |s| self.escape_iden(s));
		});

		// FROM clause
		writer.push_keyword("FROM");
		writer.push_space();

		// Grantees
		writer.push_list(&stmt.grantees, ", ", |w, grantee| {
			w.push(Self::format_role_specification(grantee));
		});

		// GRANTED BY
		if let Some(ref grantor) = stmt.granted_by {
			writer.push_keyword("GRANTED BY");
			writer.push_space();
			writer.push(Self::format_role_specification(grantor));
		}

		// CASCADE / RESTRICT
		if let Some(behavior) = stmt.drop_behavior {
			match behavior {
				DropBehavior::Cascade => writer.push_keyword("CASCADE"),
				DropBehavior::Restrict => writer.push_keyword("RESTRICT"),
			}
		}

		writer.finish()
	}

	fn build_create_role(&self, stmt: &crate::dcl::CreateRoleStatement) -> (String, Values) {
		use crate::dcl::RoleAttribute;

		let mut writer = SqlWriter::new();

		// CREATE ROLE keyword
		writer.push("CREATE ROLE");
		writer.push_space();

		// Role name
		writer.push_identifier(&stmt.role_name, |s| self.escape_iden(s));

		// WITH keyword (optional but commonly used)
		if !stmt.attributes.is_empty() {
			writer.push_keyword("WITH");
		}

		// Attributes
		for attr in &stmt.attributes {
			writer.push_space();
			match attr {
				RoleAttribute::SuperUser => writer.push("SUPERUSER"),
				RoleAttribute::NoSuperUser => writer.push("NOSUPERUSER"),
				RoleAttribute::CreateDb => writer.push("CREATEDB"),
				RoleAttribute::NoCreateDb => writer.push("NOCREATEDB"),
				RoleAttribute::CreateRole => writer.push("CREATEROLE"),
				RoleAttribute::NoCreateRole => writer.push("NOCREATEROLE"),
				RoleAttribute::Inherit => writer.push("INHERIT"),
				RoleAttribute::NoInherit => writer.push("NOINHERIT"),
				RoleAttribute::Login => writer.push("LOGIN"),
				RoleAttribute::NoLogin => writer.push("NOLOGIN"),
				RoleAttribute::Replication => writer.push("REPLICATION"),
				RoleAttribute::NoReplication => writer.push("NOREPLICATION"),
				RoleAttribute::BypassRls => writer.push("BYPASSRLS"),
				RoleAttribute::NoBypassRls => writer.push("NOBYPASSRLS"),
				RoleAttribute::ConnectionLimit(limit) => {
					writer.push("CONNECTION LIMIT");
					writer.push_space();
					writer.push(&limit.to_string());
				}
				RoleAttribute::Password(pwd) => {
					writer.push("PASSWORD");
					writer.push_space();
					writer.push("'");
					writer.push(pwd);
					writer.push("'");
				}
				RoleAttribute::EncryptedPassword(pwd) => {
					writer.push("ENCRYPTED PASSWORD");
					writer.push_space();
					writer.push("'");
					writer.push(pwd);
					writer.push("'");
				}
				RoleAttribute::UnencryptedPassword(pwd) => {
					writer.push("UNENCRYPTED PASSWORD");
					writer.push_space();
					writer.push("'");
					writer.push(pwd);
					writer.push("'");
				}
				RoleAttribute::ValidUntil(timestamp) => {
					writer.push("VALID UNTIL");
					writer.push_space();
					writer.push("'");
					writer.push(timestamp);
					writer.push("'");
				}
				RoleAttribute::InRole(roles) => {
					writer.push("IN ROLE");
					writer.push_space();
					writer.push_list(roles, ", ", |w, role| {
						w.push_identifier(role, |s| self.escape_iden(s));
					});
				}
				RoleAttribute::Role(roles) => {
					writer.push("ROLE");
					writer.push_space();
					writer.push_list(roles, ", ", |w, role| {
						w.push_identifier(role, |s| self.escape_iden(s));
					});
				}
				RoleAttribute::Admin(roles) => {
					writer.push("ADMIN");
					writer.push_space();
					writer.push_list(roles, ", ", |w, role| {
						w.push_identifier(role, |s| self.escape_iden(s));
					});
				}
			}
		}

		writer.finish()
	}

	fn build_drop_role(&self, stmt: &crate::dcl::DropRoleStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// DROP ROLE keyword
		writer.push("DROP ROLE");
		writer.push_space();

		// IF EXISTS clause
		if stmt.if_exists {
			writer.push("IF EXISTS");
			writer.push_space();
		}

		// Role names (comma-separated)
		writer.push_list(&stmt.role_names, ", ", |w, role_name| {
			w.push_identifier(role_name, |s| self.escape_iden(s));
		});

		writer.finish()
	}

	fn build_alter_role(&self, stmt: &crate::dcl::AlterRoleStatement) -> (String, Values) {
		use crate::dcl::RoleAttribute;

		let mut writer = SqlWriter::new();

		// Check for RENAME TO (special case in PostgreSQL)
		if let Some(ref new_name) = stmt.rename_to {
			writer.push("ALTER ROLE");
			writer.push_space();
			writer.push_identifier(&stmt.role_name, |s| self.escape_iden(s));
			writer.push_keyword("RENAME TO");
			writer.push_space();
			writer.push_identifier(new_name, |s| self.escape_iden(s));
			return writer.finish();
		}

		// ALTER ROLE keyword
		writer.push("ALTER ROLE");
		writer.push_space();

		// Role name
		writer.push_identifier(&stmt.role_name, |s| self.escape_iden(s));

		// WITH keyword (optional but commonly used)
		if !stmt.attributes.is_empty() {
			writer.push_keyword("WITH");
		}

		// Attributes (same as CREATE ROLE)
		for attr in &stmt.attributes {
			writer.push_space();
			match attr {
				RoleAttribute::SuperUser => writer.push("SUPERUSER"),
				RoleAttribute::NoSuperUser => writer.push("NOSUPERUSER"),
				RoleAttribute::CreateDb => writer.push("CREATEDB"),
				RoleAttribute::NoCreateDb => writer.push("NOCREATEDB"),
				RoleAttribute::CreateRole => writer.push("CREATEROLE"),
				RoleAttribute::NoCreateRole => writer.push("NOCREATEROLE"),
				RoleAttribute::Inherit => writer.push("INHERIT"),
				RoleAttribute::NoInherit => writer.push("NOINHERIT"),
				RoleAttribute::Login => writer.push("LOGIN"),
				RoleAttribute::NoLogin => writer.push("NOLOGIN"),
				RoleAttribute::Replication => writer.push("REPLICATION"),
				RoleAttribute::NoReplication => writer.push("NOREPLICATION"),
				RoleAttribute::BypassRls => writer.push("BYPASSRLS"),
				RoleAttribute::NoBypassRls => writer.push("NOBYPASSRLS"),
				RoleAttribute::ConnectionLimit(limit) => {
					writer.push("CONNECTION LIMIT");
					writer.push_space();
					writer.push(&limit.to_string());
				}
				RoleAttribute::Password(pwd) => {
					writer.push("PASSWORD");
					writer.push_space();
					writer.push("'");
					writer.push(pwd);
					writer.push("'");
				}
				RoleAttribute::EncryptedPassword(pwd) => {
					writer.push("ENCRYPTED PASSWORD");
					writer.push_space();
					writer.push("'");
					writer.push(pwd);
					writer.push("'");
				}
				RoleAttribute::UnencryptedPassword(pwd) => {
					writer.push("UNENCRYPTED PASSWORD");
					writer.push_space();
					writer.push("'");
					writer.push(pwd);
					writer.push("'");
				}
				RoleAttribute::ValidUntil(timestamp) => {
					writer.push("VALID UNTIL");
					writer.push_space();
					writer.push("'");
					writer.push(timestamp);
					writer.push("'");
				}
				RoleAttribute::InRole(roles) => {
					writer.push("IN ROLE");
					writer.push_space();
					writer.push_list(roles, ", ", |w, role| {
						w.push_identifier(role, |s| self.escape_iden(s));
					});
				}
				RoleAttribute::Role(roles) => {
					writer.push("ROLE");
					writer.push_space();
					writer.push_list(roles, ", ", |w, role| {
						w.push_identifier(role, |s| self.escape_iden(s));
					});
				}
				RoleAttribute::Admin(roles) => {
					writer.push("ADMIN");
					writer.push_space();
					writer.push_list(roles, ", ", |w, role| {
						w.push_identifier(role, |s| self.escape_iden(s));
					});
				}
			}
		}

		writer.finish()
	}

	fn build_create_user(&self, stmt: &crate::dcl::CreateUserStatement) -> (String, Values) {
		use crate::dcl::{CreateRoleStatement, RoleAttribute};

		// PostgreSQL CREATE USER is CREATE ROLE WITH LOGIN
		let mut create_role = CreateRoleStatement::new()
			.role(&stmt.user_name)
			.attribute(RoleAttribute::Login);

		// Add all attributes from CREATE USER
		for attr in &stmt.attributes {
			create_role = create_role.attribute(attr.clone());
		}

		// Use build_create_role to generate the SQL
		self.build_create_role(&create_role)
	}

	fn build_drop_user(&self, stmt: &crate::dcl::DropUserStatement) -> (String, Values) {
		use crate::dcl::DropRoleStatement;

		// PostgreSQL DROP USER is DROP ROLE
		let mut drop_role = DropRoleStatement::new();
		drop_role.role_names = stmt.user_names.clone();
		drop_role.if_exists = stmt.if_exists;

		// Use build_drop_role to generate the SQL
		self.build_drop_role(&drop_role)
	}

	fn build_alter_user(&self, stmt: &crate::dcl::AlterUserStatement) -> (String, Values) {
		use crate::dcl::AlterRoleStatement;

		// PostgreSQL ALTER USER is ALTER ROLE
		let mut alter_role = AlterRoleStatement::new().role(&stmt.user_name);

		// Add all attributes from ALTER USER
		for attr in &stmt.attributes {
			alter_role = alter_role.attribute(attr.clone());
		}

		// Use build_alter_role to generate the SQL
		self.build_alter_role(&alter_role)
	}

	fn build_rename_user(&self, _stmt: &crate::dcl::RenameUserStatement) -> (String, Values) {
		panic!("RENAME USER is not supported by PostgreSQL. Use ALTER USER ... RENAME TO instead.");
	}

	fn build_set_role(&self, stmt: &crate::dcl::SetRoleStatement) -> (String, Values) {
		use crate::dcl::RoleTarget;

		let mut writer = SqlWriter::new();

		writer.push("SET ROLE");
		writer.push_space();

		match &stmt.target {
			Some(RoleTarget::Named(name)) => {
				writer.push_identifier(name, |s| self.escape_iden(s));
			}
			Some(RoleTarget::None) => {
				writer.push("NONE");
			}
			Some(RoleTarget::All) => {
				panic!("SET ROLE ALL is not supported by PostgreSQL (MySQL only)");
			}
			Some(RoleTarget::AllExcept(_)) => {
				panic!("SET ROLE ALL EXCEPT is not supported by PostgreSQL (MySQL only)");
			}
			Some(RoleTarget::Default) => {
				panic!("SET ROLE DEFAULT is not supported by PostgreSQL (MySQL only)");
			}
			None => {
				panic!("SET ROLE requires a role target");
			}
		}

		writer.finish()
	}

	fn build_reset_role(&self, _stmt: &crate::dcl::ResetRoleStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();
		writer.push("RESET ROLE");
		writer.finish()
	}

	fn build_set_default_role(
		&self,
		_stmt: &crate::dcl::SetDefaultRoleStatement,
	) -> (String, Values) {
		panic!("SET DEFAULT ROLE is not supported by PostgreSQL (MySQL only)");
	}

	fn escape_identifier(&self, ident: &str) -> String {
		self.escape_iden(ident)
	}

	fn format_placeholder(&self, index: usize) -> String {
		self.placeholder(index)
	}
}

impl PostgresQueryBuilder {
	/// Format a role specification for PostgreSQL
	///
	/// # Arguments
	///
	/// * `spec` - The role specification to format
	///
	/// # Returns
	///
	/// The SQL representation of the role specification
	fn format_role_specification(spec: &crate::dcl::RoleSpecification) -> &str {
		use crate::dcl::RoleSpecification;

		match spec {
			RoleSpecification::RoleName(name) => name,
			RoleSpecification::CurrentRole => "CURRENT_ROLE",
			RoleSpecification::CurrentUser => "CURRENT_USER",
			RoleSpecification::SessionUser => "SESSION_USER",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		expr::{Expr, ExprTrait},
		query::Query,
		types::IntoIden,
	};

	#[test]
	fn test_escape_identifier() {
		let builder = PostgresQueryBuilder::new();
		assert_eq!(builder.escape_identifier("user"), "\"user\"");
		assert_eq!(builder.escape_identifier("table_name"), "\"table_name\"");
	}

	#[test]
	fn test_escape_identifier_with_quotes() {
		let builder = PostgresQueryBuilder::new();
		assert_eq!(builder.escape_identifier("user\"name"), "\"user\"\"name\"");
	}

	#[test]
	fn test_format_placeholder() {
		let builder = PostgresQueryBuilder::new();
		assert_eq!(builder.format_placeholder(1), "$1");
		assert_eq!(builder.format_placeholder(2), "$2");
		assert_eq!(builder.format_placeholder(10), "$10");
	}

	#[test]
	fn test_select_basic() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("id").column("name").from("users");

		let (sql, values) = builder.build_select(&stmt);
		assert_eq!(sql, "SELECT \"id\", \"name\" FROM \"users\"");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_select_asterisk() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.from("users");

		let (sql, values) = builder.build_select(&stmt);
		assert_eq!(sql, "SELECT * FROM \"users\"");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_select_with_where() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("id")
			.from("users")
			.and_where(Expr::col("active").eq(true));

		let (sql, _values) = builder.build_select(&stmt);
		// Note: The exact WHERE clause format depends on Expr implementation
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("FROM"));
		assert!(sql.contains("WHERE"));
	}

	#[test]
	fn test_select_with_limit_offset() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("id").from("users").limit(10).offset(20);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("FROM"));
		assert!(sql.contains("LIMIT"));
		assert!(sql.contains("OFFSET"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_insert_basic() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(["name", "email"])
			.values_panic(["Alice", "alice@example.com"]);

		let (sql, values) = builder.build_insert(&stmt);
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"name\", \"email\") VALUES ($1, $2)"
		);
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_insert_multiple_rows() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(["name", "email"])
			.values_panic(["Alice", "alice@example.com"])
			.values_panic(["Bob", "bob@example.com"]);

		let (sql, values) = builder.build_insert(&stmt);
		assert_eq!(
			sql,
			"INSERT INTO \"users\" (\"name\", \"email\") VALUES ($1, $2), ($3, $4)"
		);
		assert_eq!(values.len(), 4);
	}

	#[test]
	fn test_insert_with_returning() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(["name"])
			.values_panic(["Alice"])
			.returning(["id", "created_at"]);

		let (sql, values) = builder.build_insert(&stmt);
		assert!(sql.contains("INSERT INTO"));
		assert!(sql.contains("VALUES"));
		assert!(sql.contains("RETURNING"));
		assert!(sql.contains("\"id\""));
		assert!(sql.contains("\"created_at\""));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_insert_with_returning_all() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(["name"])
			.values_panic(["Alice"])
			.returning_all();

		let (sql, values) = builder.build_insert(&stmt);
		assert!(sql.contains("RETURNING *"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_update_basic() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::update();
		stmt.table("users")
			.value("name", "Alice")
			.value("email", "alice@example.com");

		let (sql, values) = builder.build_update(&stmt);
		assert_eq!(sql, "UPDATE \"users\" SET \"name\" = $1, \"email\" = $2");
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_update_with_where() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::update();
		stmt.table("users")
			.value("active", false)
			.and_where(Expr::col("id").eq(1));

		let (sql, values) = builder.build_update(&stmt);
		assert!(sql.contains("UPDATE"));
		assert!(sql.contains("SET"));
		assert!(sql.contains("WHERE"));
		assert_eq!(values.len(), 2); // false + 1
	}

	#[test]
	fn test_update_with_returning() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::update();
		stmt.table("users")
			.value("active", false)
			.and_where(Expr::col("id").eq(1))
			.returning(["id", "updated_at"]);

		let (sql, values) = builder.build_update(&stmt);
		assert!(sql.contains("UPDATE"));
		assert!(sql.contains("RETURNING"));
		assert!(sql.contains("\"id\""));
		assert!(sql.contains("\"updated_at\""));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_delete_basic() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::delete();
		stmt.from_table("users")
			.and_where(Expr::col("active").eq(false));

		let (sql, values) = builder.build_delete(&stmt);
		assert!(sql.contains("DELETE FROM"));
		assert!(sql.contains("\"users\""));
		assert!(sql.contains("WHERE"));
		assert_eq!(values.len(), 1); // false
	}

	#[test]
	fn test_delete_no_where() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::delete();
		stmt.from_table("users");

		let (sql, values) = builder.build_delete(&stmt);
		assert_eq!(sql, "DELETE FROM \"users\"");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_delete_with_returning() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::delete();
		stmt.from_table("users")
			.and_where(Expr::col("id").eq(1))
			.returning(["id", "name"]);

		let (sql, values) = builder.build_delete(&stmt);
		assert!(sql.contains("DELETE FROM"));
		assert!(sql.contains("RETURNING"));
		assert!(sql.contains("\"id\""));
		assert!(sql.contains("\"name\""));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_delete_with_returning_all() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::delete();
		stmt.from_table("users")
			.and_where(Expr::col("id").eq(1))
			.returning_all();

		let (sql, values) = builder.build_delete(&stmt);
		assert!(sql.contains("RETURNING *"));
		assert_eq!(values.len(), 1);
	}

	// JOIN tests

	#[test]
	fn test_inner_join_simple() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("FROM \"users\""));
		assert!(sql.contains("INNER JOIN \"orders\""));
		assert!(sql.contains("ON \"users\".\"id\" = \"orders\".\"user_id\""));
	}

	#[test]
	fn test_left_join() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("profiles.bio")
			.from("users")
			.left_join(
				"profiles",
				Expr::col(("users", "id")).eq(Expr::col(("profiles", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("LEFT JOIN \"profiles\""));
		assert!(sql.contains("ON \"users\".\"id\" = \"profiles\".\"user_id\""));
	}

	#[test]
	fn test_right_join() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.from("users")
			.right_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("RIGHT JOIN \"orders\""));
		assert!(sql.contains("ON \"users\".\"id\" = \"orders\".\"user_id\""));
	}

	#[test]
	fn test_full_outer_join() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.from("users")
			.full_outer_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("FULL OUTER JOIN \"orders\""));
		assert!(sql.contains("ON \"users\".\"id\" = \"orders\".\"user_id\""));
	}

	#[test]
	fn test_cross_join() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("roles.title")
			.from("users")
			.cross_join("roles");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("CROSS JOIN \"roles\""));
		assert!(!sql.contains("ON"));
	}

	#[test]
	fn test_multiple_joins() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.column("products.title")
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			)
			.inner_join(
				"products",
				Expr::col(("orders", "product_id")).eq(Expr::col(("products", "id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("INNER JOIN \"orders\""));
		assert!(sql.contains("INNER JOIN \"products\""));
		assert!(sql.contains("\"users\".\"id\" = \"orders\".\"user_id\""));
		assert!(sql.contains("\"orders\".\"product_id\" = \"products\".\"id\""));
	}

	#[test]
	fn test_join_with_complex_condition() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id"))
					.eq(Expr::col(("orders", "user_id")))
					.and(Expr::col(("orders", "status")).eq("active")),
			);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("INNER JOIN \"orders\""));
		assert!(sql.contains("ON"));
		assert!(sql.contains("\"users\".\"id\" = \"orders\".\"user_id\""));
		assert!(sql.contains("AND"));
		assert!(sql.contains("\"orders\".\"status\" = $"));
		assert_eq!(values.len(), 1);
	}

	// GROUP BY / HAVING tests

	#[test]
	fn test_group_by_single_column() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("category")
			.from("products")
			.group_by("category");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("GROUP BY \"category\""));
	}

	#[test]
	fn test_group_by_multiple_columns() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("category")
			.column("brand")
			.from("products")
			.group_by("category")
			.group_by("brand");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("GROUP BY \"category\", \"brand\""));
	}

	#[test]
	fn test_group_by_with_count() {
		use crate::expr::SimpleExpr;
		use crate::types::{ColumnRef, IntoIden};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("category")
			.expr(SimpleExpr::FunctionCall(
				"COUNT".into_iden(),
				vec![SimpleExpr::Column(ColumnRef::Asterisk)],
			))
			.from("products")
			.group_by("category");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("COUNT(*)"));
		assert!(sql.contains("GROUP BY \"category\""));
	}

	#[test]
	fn test_having_simple() {
		use crate::expr::SimpleExpr;
		use crate::types::{BinOper, ColumnRef, IntoIden};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		let count_expr = SimpleExpr::FunctionCall(
			"COUNT".into_iden(),
			vec![SimpleExpr::Column(ColumnRef::Asterisk)],
		);

		stmt.column("category")
			.expr(count_expr.clone())
			.from("products")
			.group_by("category")
			.and_having(SimpleExpr::Binary(
				Box::new(count_expr),
				BinOper::GreaterThan,
				Box::new(SimpleExpr::Value(5.into())),
			));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("GROUP BY \"category\""));
		assert!(sql.contains("HAVING"));
		assert!(sql.contains("COUNT(*)"));
		assert!(sql.contains(">"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_group_by_having_with_sum() {
		use crate::expr::SimpleExpr;
		use crate::types::{BinOper, ColumnRef, IntoIden};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		let sum_expr = SimpleExpr::FunctionCall(
			"SUM".into_iden(),
			vec![SimpleExpr::Column(ColumnRef::column("amount"))],
		);

		stmt.column("user_id")
			.expr(sum_expr.clone())
			.from("orders")
			.group_by("user_id")
			.and_having(SimpleExpr::Binary(
				Box::new(sum_expr),
				BinOper::GreaterThan,
				Box::new(SimpleExpr::Value(1000.into())),
			));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("SUM(\"amount\")"));
		assert!(sql.contains("GROUP BY \"user_id\""));
		assert!(sql.contains("HAVING"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_select_distinct() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("category").from("products").distinct();

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.starts_with("SELECT DISTINCT"));
		assert!(sql.contains("\"category\""));
		assert!(sql.contains("FROM \"products\""));
	}

	#[test]
	fn test_select_distinct_on() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("id")
			.column("name")
			.from("users")
			.distinct_on(vec!["category"])
			.order_by("category", crate::types::Order::Asc);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("SELECT DISTINCT ON (\"category\")"));
		assert!(sql.contains("\"id\""));
		assert!(sql.contains("\"name\""));
		assert!(sql.contains("ORDER BY \"category\" ASC"));
	}

	#[test]
	#[should_panic(expected = "PostgreSQL does not support DISTINCT ROW")]
	fn test_select_distinct_row_panics() {
		use crate::query::SelectDistinct;

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("products");
		stmt.distinct = Some(SelectDistinct::DistinctRow);

		let _ = builder.build_select(&stmt);
	}

	#[test]
	fn test_select_union() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("id").from("users");

		let mut stmt2 = Query::select();
		stmt2.column("id").from("customers");

		stmt1.union(stmt2);

		let (sql, _values) = builder.build_select(&stmt1);
		assert!(sql.contains("SELECT \"id\" FROM \"users\""));
		assert!(sql.contains("UNION SELECT \"id\" FROM \"customers\""));
	}

	#[test]
	fn test_select_union_all() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("name").from("products");

		let mut stmt2 = Query::select();
		stmt2.column("name").from("archived_products");

		stmt1.union_all(stmt2);

		let (sql, _values) = builder.build_select(&stmt1);
		assert!(sql.contains("SELECT \"name\" FROM \"products\""));
		assert!(sql.contains("UNION ALL SELECT \"name\" FROM \"archived_products\""));
	}

	#[test]
	fn test_select_intersect() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("email").from("subscribers");

		let mut stmt2 = Query::select();
		stmt2.column("email").from("customers");

		stmt1.intersect(stmt2);

		let (sql, _values) = builder.build_select(&stmt1);
		assert!(sql.contains("SELECT \"email\" FROM \"subscribers\""));
		assert!(sql.contains("INTERSECT SELECT \"email\" FROM \"customers\""));
	}

	#[test]
	fn test_select_except() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("id").from("all_users");

		let mut stmt2 = Query::select();
		stmt2.column("id").from("banned_users");

		stmt1.except(stmt2);

		let (sql, _values) = builder.build_select(&stmt1);
		assert!(sql.contains("SELECT \"id\" FROM \"all_users\""));
		assert!(sql.contains("EXCEPT SELECT \"id\" FROM \"banned_users\""));
	}

	#[test]
	fn test_select_multiple_unions() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("id").from("table1");

		let mut stmt2 = Query::select();
		stmt2.column("id").from("table2");

		let mut stmt3 = Query::select();
		stmt3.column("id").from("table3");

		stmt1.union(stmt2);
		stmt1.union_all(stmt3);

		let (sql, _values) = builder.build_select(&stmt1);
		assert!(sql.contains("SELECT \"id\" FROM \"table1\""));
		assert!(sql.contains("UNION SELECT \"id\" FROM \"table2\""));
		assert!(sql.contains("UNION ALL SELECT \"id\" FROM \"table3\""));
	}

	#[test]
	fn test_select_exists_subquery() {
		use crate::expr::Expr;

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		// Main query
		stmt.column("name").from("users");

		// Subquery
		let mut subquery = Query::select();
		subquery
			.column("id")
			.from("orders")
			.and_where(Expr::col(("orders", "user_id")).eq(Expr::col(("users", "id"))));

		// Add EXISTS condition
		stmt.and_where(Expr::exists(subquery));

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("SELECT \"name\" FROM \"users\""));
		assert!(sql.contains("WHERE"));
		assert!(sql.contains("EXISTS"));
		assert!(sql.contains("SELECT \"id\" FROM \"orders\""));
	}

	#[test]
	fn test_select_in_subquery() {
		use crate::expr::Expr;

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		// Main query
		stmt.column("name").from("users");

		// Subquery
		let mut subquery = Query::select();
		subquery.column("user_id").from("premium_users");

		// Add IN condition
		stmt.and_where(Expr::col("id").in_subquery(subquery));

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("SELECT \"name\" FROM \"users\""));
		assert!(sql.contains("WHERE"));
		assert!(sql.contains("\"id\""));
		assert!(sql.contains("IN"));
		assert!(sql.contains("SELECT \"user_id\" FROM \"premium_users\""));
	}

	#[test]
	fn test_select_not_exists_subquery() {
		use crate::expr::Expr;

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		// Main query
		stmt.column("email").from("users");

		// Subquery
		let mut subquery = Query::select();
		subquery
			.column("id")
			.from("banned_users")
			.and_where(Expr::col(("banned_users", "user_id")).eq(Expr::col(("users", "id"))));

		// Add NOT EXISTS condition
		stmt.and_where(Expr::not_exists(subquery));

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("SELECT \"email\" FROM \"users\""));
		assert!(sql.contains("WHERE"));
		assert!(sql.contains("NOT EXISTS"));
		assert!(sql.contains("SELECT \"id\" FROM \"banned_users\""));
	}

	// --- Phase 5: Subquery Edge Case Tests ---

	#[test]
	fn test_not_in_subquery() {
		let builder = PostgresQueryBuilder::new();

		let mut subquery = Query::select();
		subquery
			.column("user_id")
			.from("blocked_users")
			.and_where(Expr::col("reason").eq("spam"));

		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("id").not_in_subquery(subquery));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("NOT IN"));
		assert!(sql.contains("SELECT \"user_id\" FROM \"blocked_users\""));
		assert!(sql.contains("\"reason\" = $"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_subquery_in_select_list() {
		let builder = PostgresQueryBuilder::new();

		let mut subquery = Query::select();
		subquery
			.expr(Expr::col("count"))
			.from("order_counts")
			.and_where(Expr::col(("order_counts", "user_id")).eq(Expr::col(("users", "id"))));

		let mut stmt = Query::select();
		stmt.column("name")
			.expr(Expr::subquery(subquery))
			.from("users");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("\"name\""));
		assert!(sql.contains("(SELECT \"count\" FROM \"order_counts\""));
		assert!(sql.contains("\"order_counts\".\"user_id\" = \"users\".\"id\""));
	}

	#[test]
	fn test_multiple_exists_conditions() {
		let builder = PostgresQueryBuilder::new();

		let mut sub1 = Query::select();
		sub1.column("id")
			.from("orders")
			.and_where(Expr::col(("orders", "user_id")).eq(Expr::col(("users", "id"))));

		let mut sub2 = Query::select();
		sub2.column("id")
			.from("reviews")
			.and_where(Expr::col(("reviews", "user_id")).eq(Expr::col(("users", "id"))));

		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::exists(sub1))
			.and_where(Expr::exists(sub2));

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("EXISTS (SELECT \"id\" FROM \"orders\""));
		assert!(sql.contains("EXISTS (SELECT \"id\" FROM \"reviews\""));
	}

	#[test]
	fn test_nested_subquery() {
		let builder = PostgresQueryBuilder::new();

		let mut inner_subquery = Query::select();
		inner_subquery
			.column("department_id")
			.from("top_departments")
			.and_where(Expr::col("revenue").gt(1000000));

		let mut outer_subquery = Query::select();
		outer_subquery
			.column("id")
			.from("employees")
			.and_where(Expr::col("department_id").in_subquery(inner_subquery));

		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("employee_id").in_subquery(outer_subquery));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("IN (SELECT \"id\" FROM \"employees\""));
		assert!(sql.contains("IN (SELECT \"department_id\" FROM \"top_departments\""));
		assert!(sql.contains("\"revenue\" > $"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_subquery_with_complex_where() {
		let builder = PostgresQueryBuilder::new();

		let mut subquery = Query::select();
		subquery
			.column("product_id")
			.from("inventory")
			.and_where(Expr::col("quantity").gt(0))
			.and_where(Expr::col("warehouse").eq("main"))
			.and_where(Expr::col("status").eq("available"));

		let mut stmt = Query::select();
		stmt.column("name")
			.column("price")
			.from("products")
			.and_where(Expr::col("id").in_subquery(subquery))
			.and_where(Expr::col("active").eq(true));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("IN (SELECT \"product_id\" FROM \"inventory\""));
		assert!(sql.contains("\"quantity\" > $"));
		assert!(sql.contains("\"warehouse\" = $"));
		assert!(sql.contains("\"status\" = $"));
		assert!(sql.contains("\"active\" = $"));
		assert_eq!(values.len(), 4); // 0, "main", "available", true
	}

	// --- Phase 5: NULL Handling Tests ---

	#[test]
	fn test_where_is_null() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("deleted_at").is_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"deleted_at\" IS"));
		assert!(sql.to_uppercase().contains("NULL"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_where_is_not_null() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("email").is_not_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"email\" IS NOT"));
		assert!(sql.to_uppercase().contains("NULL"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_is_null_combined_with_other_conditions() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("active").eq(true))
			.and_where(Expr::col("deleted_at").is_null())
			.and_where(Expr::col("email").is_not_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"active\" = $"));
		assert!(sql.contains("\"deleted_at\" IS"));
		assert!(sql.contains("\"email\" IS NOT"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_is_null_with_join() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("users", "name"))
			.from("users")
			.left_join(
				"profiles",
				Expr::col(("users", "id")).eq(Expr::col(("profiles", "user_id"))),
			)
			.and_where(Expr::col(("profiles", "id")).is_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LEFT JOIN \"profiles\""));
		assert!(sql.contains("\"profiles\".\"id\" IS"));
		assert_eq!(values.len(), 0);
	}

	// --- Phase 5: Complex WHERE Clause Tests ---
	#[test]
	fn test_where_or_condition() {
		use crate::expr::Condition;

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("users").cond_where(
			Condition::any()
				.add(Expr::col("status").eq("active"))
				.add(Expr::col("status").eq("pending")),
		);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"status\" = $"));
		assert!(sql.contains(" OR "));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_where_between() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("products")
			.and_where(Expr::col("price").between(100, 500));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"price\" BETWEEN $"));
		assert!(sql.contains("AND $"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_where_not_between() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("products")
			.and_where(Expr::col("price").not_between(0, 10));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"price\" NOT BETWEEN $"));
		assert!(sql.contains("AND $"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_where_like() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("email").like("%@gmail.com"));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"email\" LIKE $"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_where_in_values() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("role").is_in(vec!["admin", "moderator", "editor"]));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("\"role\" IN"));
		assert_eq!(values.len(), 3);
	}

	#[test]
	fn test_insert_with_null_value() {
		use crate::value::Value;

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(vec!["name", "email", "phone"])
			.values(vec![
				Value::String(Some(Box::new("John".to_string()))),
				Value::String(Some(Box::new("john@example.com".to_string()))),
				Value::String(None),
			])
			.unwrap();

		let (sql, values) = builder.build_insert(&stmt);
		assert!(sql.contains("INSERT INTO \"users\""));
		assert!(sql.contains("\"name\""));
		assert!(sql.contains("\"email\""));
		assert!(sql.contains("\"phone\""));
		assert_eq!(values.len(), 3);
	}

	#[test]
	fn test_select_with_single_cte() {
		let builder = PostgresQueryBuilder::new();

		// Create CTE query
		let mut cte_query = Query::select();
		cte_query
			.column("id")
			.column("name")
			.from("employees")
			.and_where(Expr::col("department").eq("Engineering"));

		// Main query using the CTE
		let mut stmt = Query::select();
		stmt.with_cte("eng_employees", cte_query)
			.column("name")
			.from("eng_employees");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH"));
		assert!(sql.contains("\"eng_employees\""));
		assert!(sql.contains("AS"));
		assert!(sql.contains("SELECT \"id\", \"name\" FROM \"employees\""));
		assert!(sql.contains("SELECT \"name\" FROM \"eng_employees\""));
	}

	#[test]
	fn test_select_with_multiple_ctes() {
		let builder = PostgresQueryBuilder::new();

		// First CTE
		let mut cte1 = Query::select();
		cte1.column("id")
			.column("name")
			.from("employees")
			.and_where(Expr::col("department").eq("Engineering"));

		// Second CTE
		let mut cte2 = Query::select();
		cte2.column("id")
			.column("name")
			.from("employees")
			.and_where(Expr::col("department").eq("Sales"));

		// Main query using both CTEs
		let mut stmt = Query::select();
		stmt.with_cte("eng_emp", cte1)
			.with_cte("sales_emp", cte2)
			.column("name")
			.from("eng_emp");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH"));
		assert!(sql.contains("\"eng_emp\""));
		assert!(sql.contains("\"sales_emp\""));
		assert!(sql.contains("AS"));
		// Both CTEs should be present, and there should be a comma between them
		assert!(sql.contains("\"eng_emp\" AS"));
		assert!(sql.contains("\"sales_emp\" AS"));
	}

	#[test]
	fn test_select_with_recursive_cte() {
		let builder = PostgresQueryBuilder::new();

		// Recursive CTE for organizational hierarchy
		let mut cte_query = Query::select();
		cte_query
			.column("id")
			.column("name")
			.column("manager_id")
			.from("employees");

		// Main query using recursive CTE
		let mut stmt = Query::select();
		stmt.with_recursive_cte("employee_hierarchy", cte_query)
			.column("name")
			.from("employee_hierarchy");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH RECURSIVE"));
		assert!(sql.contains("\"employee_hierarchy\""));
		assert!(sql.contains("AS"));
		assert!(sql.contains("SELECT \"id\", \"name\", \"manager_id\" FROM \"employees\""));
		assert!(sql.contains("SELECT \"name\" FROM \"employee_hierarchy\""));
	}

	// Window function tests

	#[test]
	fn test_window_row_number_with_partition_and_order() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("department").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("salary".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::row_number().over(window))
			.column("name")
			.from("employees");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT ROW_NUMBER() OVER ( PARTITION BY "department" ORDER BY "salary" DESC ), "name" FROM "employees""#
		);
	}

	#[test]
	fn test_window_row_number_order_only() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("id".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::row_number().over(window)).from("users");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT ROW_NUMBER() OVER ( ORDER BY "id" ASC ) FROM "users""#
		);
	}

	#[test]
	fn test_window_rank_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("score".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::rank().over(window))
			.column("name")
			.from("students");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT RANK() OVER ( ORDER BY "score" DESC ), "name" FROM "students""#
		);
	}

	#[test]
	fn test_window_rank_with_partition() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("class").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("score".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::rank().over(window))
			.column("name")
			.from("students");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT RANK() OVER ( PARTITION BY "class" ORDER BY "score" DESC ), "name" FROM "students""#
		);
	}

	#[test]
	fn test_window_dense_rank_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("points".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::dense_rank().over(window))
			.column("player")
			.from("scores");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT DENSE_RANK() OVER ( ORDER BY "points" DESC ), "player" FROM "scores""#
		);
	}

	#[test]
	fn test_window_dense_rank_with_partition() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("league").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("points".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::dense_rank().over(window))
			.column("player")
			.from("scores");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT DENSE_RANK() OVER ( PARTITION BY "league" ORDER BY "points" DESC ), "player" FROM "scores""#
		);
	}

	#[test]
	fn test_window_ntile_four_buckets() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("salary".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::ntile(4).over(window))
			.column("name")
			.from("employees");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT NTILE($1) OVER ( ORDER BY "salary" ASC ), "name" FROM "employees""#
		);
	}

	#[test]
	fn test_window_ntile_custom_buckets() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("department").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("salary".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::ntile(3).over(window))
			.column("name")
			.from("employees");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT NTILE($1) OVER ( PARTITION BY "department" ORDER BY "salary" DESC ), "name" FROM "employees""#
		);
	}

	#[test]
	fn test_window_lead_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("date".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::lead(Expr::col("price").into_simple_expr(), None, None).over(window))
			.column("date")
			.from("stocks");

		let (sql, values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT LEAD("price") OVER ( ORDER BY "date" ASC ), "date" FROM "stocks""#
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_window_lead_with_offset_and_default() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("ticker").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("date".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(
			Expr::lead(
				Expr::col("price").into_simple_expr(),
				Some(2),
				Some(0.0.into()),
			)
			.over(window),
		)
		.column("date")
		.from("stocks");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LEAD"));
		assert!(sql.contains("OVER"));
		assert!(sql.contains(r#"PARTITION BY "ticker""#));
		assert!(sql.contains(r#"ORDER BY "date" ASC"#));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_window_lag_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("month".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::lag(Expr::col("revenue").into_simple_expr(), None, None).over(window))
			.column("month")
			.from("sales");

		let (sql, values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT LAG("revenue") OVER ( ORDER BY "month" ASC ), "month" FROM "sales""#
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_window_lag_with_offset_and_default() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("product").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("month".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(
			Expr::lag(
				Expr::col("revenue").into_simple_expr(),
				Some(3),
				Some(0.0.into()),
			)
			.over(window),
		)
		.column("month")
		.from("sales");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LAG"));
		assert!(sql.contains("OVER"));
		assert!(sql.contains(r#"PARTITION BY "product""#));
		assert!(sql.contains(r#"ORDER BY "month" ASC"#));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_window_first_value() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("category").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("price".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::first_value(Expr::col("name").into_simple_expr()).over(window))
			.column("name")
			.from("products");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT FIRST_VALUE("name") OVER ( PARTITION BY "category" ORDER BY "price" ASC ), "name" FROM "products""#
		);
	}

	#[test]
	fn test_window_last_value() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("category").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("price".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::last_value(Expr::col("name").into_simple_expr()).over(window))
			.column("name")
			.from("products");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT LAST_VALUE("name") OVER ( PARTITION BY "category" ORDER BY "price" DESC ), "name" FROM "products""#
		);
	}

	#[test]
	fn test_window_nth_value() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("department").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("salary".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::nth_value(Expr::col("name").into_simple_expr(), 2).over(window))
			.column("name")
			.from("employees");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("NTH_VALUE"));
		assert!(sql.contains(r#"PARTITION BY "department""#));
		assert!(sql.contains(r#"ORDER BY "salary" DESC"#));
		assert_eq!(values.len(), 1); // The "2" parameter
	}

	#[test]
	fn test_window_row_number_multiple_partition_columns() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![
				Expr::col("country").into_simple_expr(),
				Expr::col("city").into_simple_expr(),
			],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("population".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::row_number().over(window))
			.column("name")
			.from("cities");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT ROW_NUMBER() OVER ( PARTITION BY "country", "city" ORDER BY "population" DESC ), "name" FROM "cities""#
		);
	}

	#[test]
	fn test_window_ntile_with_partition() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("region").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("revenue".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::ntile(5).over(window))
			.column("store_name")
			.from("stores");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT NTILE($1) OVER ( PARTITION BY "region" ORDER BY "revenue" DESC ), "store_name" FROM "stores""#
		);
	}

	#[test]
	fn test_window_lead_with_offset_no_default() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("quarter".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::lead(Expr::col("sales").into_simple_expr(), Some(2), None).over(window))
			.column("quarter")
			.from("quarterly_sales");

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT LEAD("sales", $1) OVER ( ORDER BY "quarter" ASC ), "quarter" FROM "quarterly_sales""#
		);
	}

	#[test]
	fn test_window_lag_with_different_offset() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col("sensor_id").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("timestamp".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::lag(Expr::col("reading").into_simple_expr(), Some(5), None).over(window))
			.column("timestamp")
			.from("sensor_data");

		let (sql, values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT LAG("reading", $1) OVER ( PARTITION BY "sensor_id" ORDER BY "timestamp" ASC ), "timestamp" FROM "sensor_data""#
		);
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_window_multiple_functions_in_query() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window1 = WindowStatement {
			partition_by: vec![Expr::col("department").into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("salary".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		let window2 = WindowStatement {
			partition_by: vec![],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::Column("hire_date".into_iden()),
				order: Order::Asc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.expr(Expr::row_number().over(window1))
			.expr(Expr::rank().over(window2))
			.column("name")
			.from("employees");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(
			sql.contains(
				r#"ROW_NUMBER() OVER ( PARTITION BY "department" ORDER BY "salary" DESC )"#
			)
		);
		assert!(sql.contains(r#"RANK() OVER ( ORDER BY "hire_date" ASC )"#));
		assert!(sql.contains(r#""name""#));
		assert!(sql.contains(r#"FROM "employees""#));
	}

	// JOIN enhancement tests

	#[test]
	fn test_join_three_tables() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("users", "name"))
			.column(("orders", "order_date"))
			.column(("products", "product_name"))
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			)
			.inner_join(
				"products",
				Expr::col(("orders", "product_id")).eq(Expr::col(("products", "id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert_eq!(
			sql,
			r#"SELECT "users"."name", "orders"."order_date", "products"."product_name" FROM "users" INNER JOIN "orders" ON "users"."id" = "orders"."user_id" INNER JOIN "products" ON "orders"."product_id" = "products"."id""#
		);
	}

	#[test]
	fn test_self_join() {
		use crate::types::TableRef;

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("e1", "name"))
			.column(("e2", "name"))
			.from(TableRef::table_alias("employees", "e1"))
			.inner_join(
				TableRef::table_alias("employees", "e2"),
				Expr::col(("e1", "manager_id")).eq(Expr::col(("e2", "id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains(r#"FROM "employees" AS "e1""#));
		assert!(sql.contains(r#"INNER JOIN "employees" AS "e2""#));
		assert!(sql.contains(r#"ON "e1"."manager_id" = "e2"."id""#));
	}

	#[test]
	fn test_join_complex_conditions() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.from("orders").left_join(
			"customers",
			Expr::col(("orders", "customer_id"))
				.eq(Expr::col(("customers", "id")))
				.and(Expr::col(("customers", "active")).eq(true))
				.and(
					Expr::col(("orders", "created_at"))
						.gt(Expr::col(("customers", "registered_at"))),
				),
		);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LEFT JOIN \"customers\""));
		assert!(sql.contains("\"orders\".\"customer_id\" = \"customers\".\"id\""));
		assert!(sql.contains("AND \"customers\".\"active\" = $"));
		assert!(sql.contains("AND \"orders\".\"created_at\" > \"customers\".\"registered_at\""));
		assert_eq!(values.len(), 1); // true value
	}

	#[test]
	fn test_join_with_subquery_in_condition() {
		let builder = PostgresQueryBuilder::new();

		let mut subquery = Query::select();
		subquery.expr(Expr::col("max_id")).from("user_stats");

		let mut stmt = Query::select();
		stmt.from("users").inner_join(
			"profiles",
			Expr::col(("users", "id"))
				.eq(Expr::col(("profiles", "user_id")))
				.and(Expr::col(("users", "id")).in_subquery(subquery)),
		);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("INNER JOIN \"profiles\""));
		assert!(sql.contains("\"users\".\"id\" = \"profiles\".\"user_id\""));
		assert!(sql.contains("IN"));
		assert!(sql.contains("SELECT \"max_id\" FROM \"user_stats\""));
	}

	#[test]
	fn test_multiple_left_joins() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("users", "name"))
			.column(("profiles", "bio"))
			.column(("addresses", "city"))
			.column(("phone_numbers", "number"))
			.from("users")
			.left_join(
				"profiles",
				Expr::col(("users", "id")).eq(Expr::col(("profiles", "user_id"))),
			)
			.left_join(
				"addresses",
				Expr::col(("users", "id")).eq(Expr::col(("addresses", "user_id"))),
			)
			.left_join(
				"phone_numbers",
				Expr::col(("users", "id")).eq(Expr::col(("phone_numbers", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("LEFT JOIN \"profiles\""));
		assert!(sql.contains("LEFT JOIN \"addresses\""));
		assert!(sql.contains("LEFT JOIN \"phone_numbers\""));
	}

	#[test]
	fn test_mixed_join_types() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("users", "name"))
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			)
			.left_join(
				"reviews",
				Expr::col(("orders", "id")).eq(Expr::col(("reviews", "order_id"))),
			)
			.right_join(
				"refunds",
				Expr::col(("orders", "id")).eq(Expr::col(("refunds", "order_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("INNER JOIN \"orders\""));
		assert!(sql.contains("LEFT JOIN \"reviews\""));
		assert!(sql.contains("RIGHT JOIN \"refunds\""));
	}

	#[test]
	fn test_join_with_group_by() {
		use crate::expr::SimpleExpr;
		use crate::types::{BinOper, ColumnRef, IntoIden};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		let count_expr = SimpleExpr::FunctionCall(
			"COUNT".into_iden(),
			vec![SimpleExpr::Column(ColumnRef::Asterisk)],
		);

		stmt.column(("users", "name"))
			.expr(count_expr.clone())
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			)
			.group_by(("users", "name"))
			.and_having(SimpleExpr::Binary(
				Box::new(count_expr),
				BinOper::GreaterThan,
				Box::new(SimpleExpr::Value(5.into())),
			));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("INNER JOIN \"orders\""));
		assert!(sql.contains("GROUP BY \"users\".\"name\""));
		assert!(sql.contains("HAVING"));
		assert!(sql.contains("COUNT(*) > $"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_join_with_window_function() {
		use crate::types::{IntoIden, Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();

		let window = WindowStatement {
			partition_by: vec![Expr::col(("departments", "name")).into_simple_expr()],
			order_by: vec![OrderExpr {
				expr: OrderExprKind::TableColumn("employees".into_iden(), "salary".into_iden()),
				order: Order::Desc,
				nulls: None,
			}],
			frame: None,
		};

		stmt.column(("employees", "name"))
			.expr(Expr::row_number().over(window))
			.from("employees")
			.inner_join(
				"departments",
				Expr::col(("employees", "department_id")).eq(Expr::col(("departments", "id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("INNER JOIN \"departments\""));
		assert!(sql.contains("ROW_NUMBER() OVER"));
		assert!(sql.contains(r#"PARTITION BY "departments"."name""#));
	}

	#[test]
	fn test_four_table_join() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("users", "name"))
			.column(("orders", "order_date"))
			.column(("products", "product_name"))
			.column(("categories", "category_name"))
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			)
			.inner_join(
				"products",
				Expr::col(("orders", "product_id")).eq(Expr::col(("products", "id"))),
			)
			.inner_join(
				"categories",
				Expr::col(("products", "category_id")).eq(Expr::col(("categories", "id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("FROM \"users\""));
		assert!(sql.contains("INNER JOIN \"orders\""));
		assert!(sql.contains("INNER JOIN \"products\""));
		assert!(sql.contains("INNER JOIN \"categories\""));
	}

	#[test]
	fn test_join_with_cte() {
		use crate::types::TableRef;

		let builder = PostgresQueryBuilder::new();

		let mut cte = Query::select();
		cte.column("user_id")
			.expr(Expr::col("total"))
			.from("order_totals")
			.and_where(Expr::col("total").gt(1000));

		let mut stmt = Query::select();
		stmt.with_cte("high_value_customers", cte)
			.column(("users", "name"))
			.column(("hvc", "total"))
			.from("users")
			.inner_join(
				TableRef::table_alias("high_value_customers", "hvc"),
				Expr::col(("users", "id")).eq(Expr::col(("hvc", "user_id"))),
			);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH \"high_value_customers\" AS"));
		assert!(sql.contains("INNER JOIN \"high_value_customers\" AS \"hvc\""));
		assert_eq!(values.len(), 1); // 1000
	}

	#[test]
	fn test_cte_with_where_and_params() {
		let builder = PostgresQueryBuilder::new();

		let mut cte_query = Query::select();
		cte_query
			.column("id")
			.column("total")
			.from("orders")
			.and_where(Expr::col("status").eq("completed"))
			.and_where(Expr::col("amount").gt(1000));

		let mut stmt = Query::select();
		stmt.with_cte("large_orders", cte_query)
			.column("id")
			.column("total")
			.from("large_orders");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH"));
		assert!(sql.contains(r#""large_orders" AS"#));
		assert!(sql.contains(r#""status" = $"#));
		assert!(sql.contains(r#""amount" > $"#));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_cte_used_in_join() {
		use crate::types::TableRef;

		let builder = PostgresQueryBuilder::new();

		let mut cte_query = Query::select();
		cte_query
			.column("user_id")
			.column("order_count")
			.from("orders")
			.group_by("user_id");

		let mut stmt = Query::select();
		stmt.with_cte("user_orders", cte_query)
			.column(("users", "name"))
			.column(("uo", "order_count"))
			.from("users")
			.inner_join(
				TableRef::table_alias("user_orders", "uo"),
				Expr::col(("users", "id")).eq(Expr::col(("uo", "user_id"))),
			);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH"));
		assert!(sql.contains(r#""user_orders" AS"#));
		assert!(sql.contains(r#"INNER JOIN "user_orders" AS "uo""#));
		assert!(sql.contains(r#""users"."id" = "uo"."user_id""#));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_cte_with_aggregation() {
		use crate::expr::SimpleExpr;
		use crate::types::{ColumnRef, IntoIden};

		let builder = PostgresQueryBuilder::new();

		let mut cte_query = Query::select();
		cte_query
			.column("category")
			.expr(SimpleExpr::FunctionCall(
				"COUNT".into_iden(),
				vec![SimpleExpr::Column(ColumnRef::Asterisk)],
			))
			.expr(SimpleExpr::FunctionCall(
				"SUM".into_iden(),
				vec![SimpleExpr::Column(ColumnRef::column("price"))],
			))
			.from("products")
			.group_by("category");

		let mut stmt = Query::select();
		stmt.with_cte("category_stats", cte_query)
			.column("category")
			.from("category_stats");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH"));
		assert!(sql.contains(r#""category_stats" AS"#));
		assert!(sql.contains("COUNT(*)"));
		assert!(sql.contains(r#"SUM("price")"#));
		assert!(sql.contains(r#"GROUP BY "category""#));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_cte_with_subquery() {
		let builder = PostgresQueryBuilder::new();

		let mut sub = Query::select();
		sub.column("user_id").from("vip_users");

		let mut cte_query = Query::select();
		cte_query
			.column("id")
			.column("total")
			.from("orders")
			.and_where(Expr::col("user_id").in_subquery(sub))
			.and_where(Expr::col("status").eq("shipped"));

		let mut stmt = Query::select();
		stmt.with_cte("vip_orders", cte_query)
			.column("id")
			.column("total")
			.from("vip_orders");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH"));
		assert!(sql.contains(r#""vip_orders" AS"#));
		assert!(sql.contains("IN"));
		assert!(sql.contains(r#"SELECT "user_id" FROM "vip_users""#));
		assert!(sql.contains(r#""status" = $"#));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_multiple_recursive_and_regular_ctes() {
		let builder = PostgresQueryBuilder::new();

		// Regular CTE
		let mut regular_cte = Query::select();
		regular_cte
			.column("id")
			.column("name")
			.from("departments")
			.and_where(Expr::col("active").eq(true));

		// Recursive CTE
		let mut recursive_cte = Query::select();
		recursive_cte
			.column("id")
			.column("name")
			.column("parent_id")
			.from("categories");

		// Main query
		let mut stmt = Query::select();
		stmt.with_cte("active_depts", regular_cte)
			.with_recursive_cte("category_tree", recursive_cte)
			.column("name")
			.from("category_tree");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WITH RECURSIVE"));
		assert!(sql.contains(r#""active_depts" AS"#));
		assert!(sql.contains(r#""category_tree" AS"#));
		assert!(sql.contains(r#""active" = $"#));
		assert!(sql.contains(r#"FROM "category_tree""#));
		assert_eq!(values.len(), 1);
	}

	// CASE expression tests

	#[test]
	fn test_case_simple_when_else() {
		let builder = PostgresQueryBuilder::new();

		let case_expr = Expr::case()
			.when(Expr::col("status").eq("active"), "Active")
			.else_result("Inactive");

		let mut stmt = Query::select();
		stmt.expr_as(case_expr, "status_label").from("users");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("CASE"));
		assert!(sql.contains("WHEN"));
		assert!(sql.contains(r#""status" = $"#));
		assert!(sql.contains("THEN"));
		assert!(sql.contains("ELSE"));
		assert!(sql.contains("END"));
		assert!(sql.contains(r#"AS "status_label""#));
		assert_eq!(values.len(), 3);
	}

	#[test]
	fn test_case_multiple_when_clauses() {
		let builder = PostgresQueryBuilder::new();

		let case_expr = Expr::case()
			.when(Expr::col("score").gte(90), "A")
			.when(Expr::col("score").gte(80), "B")
			.when(Expr::col("score").gte(70), "C")
			.else_result("F");

		let mut stmt = Query::select();
		stmt.expr_as(case_expr, "grade").from("students");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("CASE"));
		// Verify multiple WHEN clauses
		let when_count = sql.matches("WHEN").count();
		assert_eq!(when_count, 3);
		let then_count = sql.matches("THEN").count();
		assert_eq!(then_count, 3);
		assert!(sql.contains("ELSE"));
		assert!(sql.contains("END"));
		// 3 score comparisons + 3 THEN values + 1 ELSE value = 7
		assert_eq!(values.len(), 7);
	}

	#[test]
	fn test_case_without_else() {
		let builder = PostgresQueryBuilder::new();

		let case_expr = Expr::case()
			.when(Expr::col("type").eq("admin"), "Administrator")
			.when(Expr::col("type").eq("user"), "Regular User")
			.build();

		let mut stmt = Query::select();
		stmt.expr_as(case_expr, "type_label").from("accounts");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("CASE"));
		assert!(sql.contains("WHEN"));
		assert!(sql.contains("THEN"));
		assert!(!sql.contains("ELSE"));
		assert!(sql.contains("END"));
		assert_eq!(values.len(), 4);
	}

	#[test]
	fn test_case_in_where_clause() {
		let builder = PostgresQueryBuilder::new();

		let case_expr = Expr::case()
			.when(Expr::col("role").eq("admin"), 1)
			.else_result(0);

		let mut stmt = Query::select();
		stmt.column("name").from("users").and_where(case_expr.eq(1));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WHERE"));
		assert!(sql.contains("CASE"));
		assert!(sql.contains("WHEN"));
		assert!(sql.contains("END"));
		assert!(values.len() >= 3);
	}

	#[test]
	fn test_case_in_order_by() {
		let builder = PostgresQueryBuilder::new();

		let case_expr = Expr::case()
			.when(Expr::col("priority").eq("high"), 1)
			.when(Expr::col("priority").eq("medium"), 2)
			.else_result(3);

		let mut stmt = Query::select();
		stmt.column("name")
			.column("priority")
			.from("tasks")
			.order_by_expr(case_expr, crate::types::Order::Asc);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("ORDER BY"));
		assert!(sql.contains("CASE"));
		assert!(sql.contains("WHEN"));
		assert!(sql.contains("END"));
		assert!(sql.contains("ASC"));
		assert_eq!(values.len(), 5);
	}

	// ORDER BY / LIMIT edge case tests

	#[test]
	fn test_order_by_multiple_columns_mixed() {
		let builder = PostgresQueryBuilder::new();

		let mut stmt = Query::select();
		stmt.column("name")
			.column("age")
			.column("score")
			.from("students")
			.order_by("name", crate::types::Order::Asc)
			.order_by("age", crate::types::Order::Desc)
			.order_by("score", crate::types::Order::Asc);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("ORDER BY"));
		assert!(sql.contains(r#""name" ASC"#));
		assert!(sql.contains(r#""age" DESC"#));
		assert!(sql.contains(r#""score" ASC"#));
	}

	#[test]
	fn test_order_by_nulls_first() {
		use crate::types::{IntoColumnRef, NullOrdering, OrderExpr, OrderExprKind};

		let builder = PostgresQueryBuilder::new();

		let mut stmt = Query::select();
		stmt.column("name").column("created_at").from("events");
		stmt.orders.push(OrderExpr {
			expr: OrderExprKind::Expr(Box::new(SimpleExpr::Column("created_at".into_column_ref()))),
			order: crate::types::Order::Desc,
			nulls: Some(NullOrdering::First),
		});

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("ORDER BY"));
		assert!(sql.contains("DESC"));
		assert!(sql.contains("NULLS FIRST"));
	}

	#[test]
	fn test_order_by_nulls_last() {
		use crate::types::{IntoColumnRef, NullOrdering, OrderExpr, OrderExprKind};

		let builder = PostgresQueryBuilder::new();

		let mut stmt = Query::select();
		stmt.column("name").column("updated_at").from("posts");
		stmt.orders.push(OrderExpr {
			expr: OrderExprKind::Expr(Box::new(SimpleExpr::Column("updated_at".into_column_ref()))),
			order: crate::types::Order::Asc,
			nulls: Some(NullOrdering::Last),
		});

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("ORDER BY"));
		assert!(sql.contains("ASC"));
		assert!(sql.contains("NULLS LAST"));
	}

	#[test]
	fn test_limit_without_offset() {
		let builder = PostgresQueryBuilder::new();

		let mut stmt = Query::select();
		stmt.column("id").from("items").limit(5);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LIMIT"));
		assert!(!sql.contains("OFFSET"));
		assert_eq!(values.len(), 1);
	}

	// Arithmetic / string operation tests

	#[test]
	fn test_arithmetic_add_sub() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("products");
		stmt.and_where(Expr::col("price").add(10i32).gt(100i32));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains(r#""price" + $1"#));
		assert!(sql.contains("> $2"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_arithmetic_mul_div_mod() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("items");
		stmt.and_where(
			Expr::col("quantity")
				.mul(Expr::col("unit_price"))
				.gt(1000i32),
		);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains(r#""quantity" * "unit_price""#));
		assert!(sql.contains("> $1"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_like_ilike_pattern() {
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("users");
		stmt.and_where(Expr::col("email").like("%@example.com"));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains(r#""email" LIKE $1"#));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_pg_concat_operator() {
		use crate::types::{BinOper, IntoColumnRef, PgBinOper};
		let builder = PostgresQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.expr(SimpleExpr::Binary(
			Box::new(SimpleExpr::Column("first_name".into_column_ref())),
			BinOper::PgOperator(PgBinOper::Concatenate),
			Box::new(SimpleExpr::Column("last_name".into_column_ref())),
		));
		stmt.from("users");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains(r#""first_name" || "last_name""#));
	}

	// DCL (Data Control Language) Tests

	#[test]
	fn test_grant_single_privilege_on_table() {
		use crate::dcl::{GrantStatement, Grantee, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Select)
			.on_table("users")
			.to("app_user");

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(sql, r#"GRANT SELECT ON TABLE "users" TO "app_user""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_multiple_privileges() {
		use crate::dcl::{GrantStatement, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privileges(vec![
				Privilege::Select,
				Privilege::Insert,
				Privilege::Update,
			])
			.on_table("users")
			.to("app_user");

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(
			sql,
			r#"GRANT SELECT, INSERT, UPDATE ON TABLE "users" TO "app_user""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_multiple_objects() {
		use crate::dcl::{GrantStatement, ObjectType, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Select)
			.object_type(ObjectType::Table)
			.object("users")
			.object("posts")
			.to("app_user");

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(
			sql,
			r#"GRANT SELECT ON TABLE "users", "posts" TO "app_user""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_multiple_grantees() {
		use crate::dcl::{GrantStatement, Grantee, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Select)
			.on_table("users")
			.grantee(Grantee::role("app_user"))
			.grantee(Grantee::role("readonly_user"));

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(
			sql,
			r#"GRANT SELECT ON TABLE "users" TO "app_user", "readonly_user""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_with_grant_option() {
		use crate::dcl::{GrantStatement, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Select)
			.on_table("users")
			.to("app_user")
			.with_grant_option(true);

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(
			sql,
			r#"GRANT SELECT ON TABLE "users" TO "app_user" WITH GRANT OPTION"#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_with_granted_by() {
		use crate::dcl::{GrantStatement, Grantee, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Select)
			.on_table("users")
			.to("app_user")
			.granted_by(Grantee::role("admin"));

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(
			sql,
			r#"GRANT SELECT ON TABLE "users" TO "app_user" GRANTED BY "admin""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_on_database() {
		use crate::dcl::{GrantStatement, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Create)
			.on_database("mydb")
			.to("app_user");

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(sql, r#"GRANT CREATE ON DATABASE "mydb" TO "app_user""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_on_schema() {
		use crate::dcl::{GrantStatement, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Usage)
			.on_schema("public")
			.to("app_user");

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(sql, r#"GRANT USAGE ON SCHEMA "public" TO "app_user""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_on_sequence() {
		use crate::dcl::{GrantStatement, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Usage)
			.on_sequence("user_id_seq")
			.to("app_user");

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(
			sql,
			r#"GRANT USAGE ON SEQUENCE "user_id_seq" TO "app_user""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_all_privileges() {
		use crate::dcl::{GrantStatement, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::All)
			.on_table("users")
			.to("admin");

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(sql, r#"GRANT ALL PRIVILEGES ON TABLE "users" TO "admin""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_to_public() {
		use crate::dcl::{GrantStatement, Grantee, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Select)
			.on_table("public_data")
			.grantee(Grantee::Public);

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(sql, r#"GRANT SELECT ON TABLE "public_data" TO PUBLIC"#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_to_current_user() {
		use crate::dcl::{GrantStatement, Grantee, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privilege(Privilege::Select)
			.on_table("users")
			.grantee(Grantee::CurrentUser);

		let (sql, values) = builder.build_grant(&stmt);
		assert_eq!(sql, r#"GRANT SELECT ON TABLE "users" TO CURRENT_USER"#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_grant_complex() {
		use crate::dcl::{GrantStatement, Grantee, Privilege};

		let builder = PostgresQueryBuilder::new();
		let stmt = GrantStatement::new()
			.privileges(vec![
				Privilege::Select,
				Privilege::Insert,
				Privilege::Update,
			])
			.on_table("users")
			.on_table("posts")
			.grantee(Grantee::role("app_user"))
			.grantee(Grantee::role("readonly_user"))
			.with_grant_option(true)
			.granted_by(Grantee::role("admin"));

		let (sql, values) = builder.build_grant(&stmt);
		assert!(sql.starts_with("GRANT SELECT, INSERT, UPDATE ON TABLE"));
		assert!(sql.contains(r#""users", "posts""#));
		assert!(sql.contains(r#"TO "app_user", "readonly_user""#));
		assert!(sql.contains("WITH GRANT OPTION"));
		assert!(sql.contains(r#"GRANTED BY "admin""#));
		assert!(values.is_empty());
	}

	// REVOKE tests

	#[test]
	fn test_revoke_single_privilege() {
		use crate::dcl::{Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::Insert)
			.from_table("users")
			.from("app_user");

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(sql, r#"REVOKE INSERT ON TABLE "users" FROM "app_user""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_multiple_privileges() {
		use crate::dcl::{Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privileges(vec![
				Privilege::Select,
				Privilege::Insert,
				Privilege::Update,
			])
			.from_table("users")
			.from("app_user");

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(
			sql,
			r#"REVOKE SELECT, INSERT, UPDATE ON TABLE "users" FROM "app_user""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_with_cascade() {
		use crate::dcl::{Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::All)
			.from_table("users")
			.from("app_user")
			.cascade(true);

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(
			sql,
			r#"REVOKE ALL PRIVILEGES ON TABLE "users" FROM "app_user" CASCADE"#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_grant_option_for() {
		use crate::dcl::{Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::Select)
			.from_table("users")
			.from("app_user")
			.grant_option_for(true);

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(
			sql,
			r#"REVOKE GRANT OPTION FOR SELECT ON TABLE "users" FROM "app_user""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_from_database() {
		use crate::dcl::{Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::Create)
			.from_database("mydb")
			.from("app_user");

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(sql, r#"REVOKE CREATE ON DATABASE "mydb" FROM "app_user""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_from_schema() {
		use crate::dcl::{Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::Usage)
			.from_schema("public")
			.from("app_user");

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(sql, r#"REVOKE USAGE ON SCHEMA "public" FROM "app_user""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_from_sequence() {
		use crate::dcl::{Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::Usage)
			.from_sequence("user_id_seq")
			.from("app_user");

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(
			sql,
			r#"REVOKE USAGE ON SEQUENCE "user_id_seq" FROM "app_user""#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_from_public() {
		use crate::dcl::{Grantee, Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::Select)
			.from_table("public_data")
			.grantee(Grantee::Public);

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(sql, r#"REVOKE SELECT ON TABLE "public_data" FROM PUBLIC"#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_from_current_user() {
		use crate::dcl::{Grantee, Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privilege(Privilege::Select)
			.from_table("users")
			.grantee(Grantee::CurrentUser);

		let (sql, values) = builder.build_revoke(&stmt);
		assert_eq!(sql, r#"REVOKE SELECT ON TABLE "users" FROM CURRENT_USER"#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_revoke_complex() {
		use crate::dcl::{Grantee, Privilege, RevokeStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = RevokeStatement::new()
			.privileges(vec![Privilege::Select, Privilege::Insert])
			.from_table("users")
			.from_table("posts")
			.grantee(Grantee::role("app_user"))
			.grantee(Grantee::role("readonly_user"))
			.cascade(true);

		let (sql, values) = builder.build_revoke(&stmt);
		assert!(sql.starts_with("REVOKE SELECT, INSERT ON TABLE"));
		assert!(sql.contains(r#""users", "posts""#));
		assert!(sql.contains(r#"FROM "app_user", "readonly_user""#));
		assert!(sql.contains("CASCADE"));
		assert!(values.is_empty());
	}

	#[test]
	fn test_create_role_simple() {
		use crate::dcl::CreateRoleStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = CreateRoleStatement::new().role("developer");

		let (sql, values) = builder.build_create_role(&stmt);
		assert_eq!(sql, r#"CREATE ROLE "developer""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_create_role_with_login() {
		use crate::dcl::{CreateRoleStatement, RoleAttribute};

		let builder = PostgresQueryBuilder::new();
		let stmt = CreateRoleStatement::new()
			.role("app_user")
			.attribute(RoleAttribute::Login)
			.attribute(RoleAttribute::Password("secret".to_string()));

		let (sql, values) = builder.build_create_role(&stmt);
		assert_eq!(
			sql,
			r#"CREATE ROLE "app_user" WITH LOGIN PASSWORD 'secret'"#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_create_role_with_multiple_attributes() {
		use crate::dcl::{CreateRoleStatement, RoleAttribute};

		let builder = PostgresQueryBuilder::new();
		let stmt = CreateRoleStatement::new()
			.role("superuser")
			.attribute(RoleAttribute::SuperUser)
			.attribute(RoleAttribute::CreateDb)
			.attribute(RoleAttribute::CreateRole)
			.attribute(RoleAttribute::ConnectionLimit(10));

		let (sql, values) = builder.build_create_role(&stmt);
		assert_eq!(
			sql,
			r#"CREATE ROLE "superuser" WITH SUPERUSER CREATEDB CREATEROLE CONNECTION LIMIT 10"#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_drop_role_simple() {
		use crate::dcl::DropRoleStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = DropRoleStatement::new().role("old_role");

		let (sql, values) = builder.build_drop_role(&stmt);
		assert_eq!(sql, r#"DROP ROLE "old_role""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_drop_role_if_exists() {
		use crate::dcl::DropRoleStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = DropRoleStatement::new().role("old_role").if_exists(true);

		let (sql, values) = builder.build_drop_role(&stmt);
		assert_eq!(sql, r#"DROP ROLE IF EXISTS "old_role""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_drop_role_multiple() {
		use crate::dcl::DropRoleStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = DropRoleStatement::new()
			.role("role1")
			.role("role2")
			.role("role3");

		let (sql, values) = builder.build_drop_role(&stmt);
		assert_eq!(sql, r#"DROP ROLE "role1", "role2", "role3""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_alter_role_with_attributes() {
		use crate::dcl::{AlterRoleStatement, RoleAttribute};

		let builder = PostgresQueryBuilder::new();
		let stmt = AlterRoleStatement::new()
			.role("developer")
			.attribute(RoleAttribute::NoLogin)
			.attribute(RoleAttribute::ConnectionLimit(5));

		let (sql, values) = builder.build_alter_role(&stmt);
		assert_eq!(
			sql,
			r#"ALTER ROLE "developer" WITH NOLOGIN CONNECTION LIMIT 5"#
		);
		assert!(values.is_empty());
	}

	#[test]
	fn test_alter_role_rename_to() {
		use crate::dcl::AlterRoleStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = AlterRoleStatement::new()
			.role("old_name")
			.rename_to("new_name");

		let (sql, values) = builder.build_alter_role(&stmt);
		assert_eq!(sql, r#"ALTER ROLE "old_name" RENAME TO "new_name""#);
		assert!(values.is_empty());
	}

	// CREATE USER tests
	#[test]
	fn test_create_user_basic() {
		use crate::dcl::CreateUserStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = CreateUserStatement::new().user("app_user");

		let (sql, values) = builder.build_create_user(&stmt);
		assert_eq!(sql, r#"CREATE ROLE "app_user" WITH LOGIN"#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_create_user_with_password() {
		use crate::dcl::{CreateUserStatement, RoleAttribute};

		let builder = PostgresQueryBuilder::new();
		let stmt = CreateUserStatement::new()
			.user("app_user")
			.attribute(RoleAttribute::Password("secret".to_string()));

		let (sql, values) = builder.build_create_user(&stmt);
		assert_eq!(
			sql,
			r#"CREATE ROLE "app_user" WITH LOGIN PASSWORD 'secret'"#
		);
		assert!(values.is_empty());
	}

	// DROP USER tests
	#[test]
	fn test_drop_user_basic() {
		use crate::dcl::DropUserStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = DropUserStatement::new().user("app_user");

		let (sql, values) = builder.build_drop_user(&stmt);
		assert_eq!(sql, r#"DROP ROLE "app_user""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_drop_user_if_exists() {
		use crate::dcl::DropUserStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = DropUserStatement::new().user("app_user").if_exists(true);

		let (sql, values) = builder.build_drop_user(&stmt);
		assert_eq!(sql, r#"DROP ROLE IF EXISTS "app_user""#);
		assert!(values.is_empty());
	}

	// ALTER USER tests
	#[test]
	fn test_alter_user_basic() {
		use crate::dcl::{AlterUserStatement, RoleAttribute};

		let builder = PostgresQueryBuilder::new();
		let stmt = AlterUserStatement::new()
			.user("app_user")
			.attribute(RoleAttribute::Password("new_secret".to_string()));

		let (sql, values) = builder.build_alter_user(&stmt);
		assert_eq!(sql, r#"ALTER ROLE "app_user" WITH PASSWORD 'new_secret'"#);
		assert!(values.is_empty());
	}

	// RENAME USER panic test
	#[test]
	#[should_panic(expected = "RENAME USER is not supported by PostgreSQL")]
	fn test_rename_user_panics() {
		use crate::dcl::RenameUserStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = RenameUserStatement::new().rename("old", "new");

		builder.build_rename_user(&stmt);
	}

	// SET ROLE tests
	#[test]
	fn test_set_role_named() {
		use crate::dcl::{RoleTarget, SetRoleStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = SetRoleStatement::new().role(RoleTarget::Named("admin".to_string()));

		let (sql, values) = builder.build_set_role(&stmt);
		assert_eq!(sql, r#"SET ROLE "admin""#);
		assert!(values.is_empty());
	}

	#[test]
	fn test_set_role_none() {
		use crate::dcl::{RoleTarget, SetRoleStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = SetRoleStatement::new().role(RoleTarget::None);

		let (sql, values) = builder.build_set_role(&stmt);
		assert_eq!(sql, "SET ROLE NONE");
		assert!(values.is_empty());
	}

	#[test]
	#[should_panic(expected = "SET ROLE ALL is not supported by PostgreSQL")]
	fn test_set_role_all_panics() {
		use crate::dcl::{RoleTarget, SetRoleStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = SetRoleStatement::new().role(RoleTarget::All);

		builder.build_set_role(&stmt);
	}

	// RESET ROLE test
	#[test]
	fn test_reset_role() {
		use crate::dcl::ResetRoleStatement;

		let builder = PostgresQueryBuilder::new();
		let stmt = ResetRoleStatement::new();

		let (sql, values) = builder.build_reset_role(&stmt);
		assert_eq!(sql, "RESET ROLE");
		assert!(values.is_empty());
	}

	// SET DEFAULT ROLE panic test
	#[test]
	#[should_panic(expected = "SET DEFAULT ROLE is not supported by PostgreSQL")]
	fn test_set_default_role_panics() {
		use crate::dcl::{DefaultRoleSpec, SetDefaultRoleStatement};

		let builder = PostgresQueryBuilder::new();
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::All)
			.user("app_user");

		builder.build_set_default_role(&stmt);
	}
}
