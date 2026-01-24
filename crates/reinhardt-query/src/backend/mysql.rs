//! MySQL query builder backend
//!
//! This module implements the SQL generation backend for MySQL.

use super::{QueryBuilder, SqlWriter};
use crate::{
	expr::{Condition, SimpleExpr},
	query::{
		AlterIndexStatement, AlterTableOperation, AlterTableStatement, CreateIndexStatement,
		CreateTableStatement, CreateTriggerStatement, CreateViewStatement, DeleteStatement,
		DropIndexStatement, DropTableStatement, DropTriggerStatement, DropViewStatement,
		InsertStatement, ReindexStatement, SelectStatement, TruncateTableStatement,
		UpdateStatement,
	},
	types::{BinOper, ColumnRef, TableRef},
	value::Values,
};

/// MySQL query builder
///
/// This struct implements SQL generation for MySQL, using the following conventions:
/// - Identifiers: Backticks (`` `table_name` ``)
/// - Placeholders: Question marks (`?`)
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::backend::{MySqlQueryBuilder, QueryBuilder};
/// use reinhardt_query::prelude::*;
///
/// let builder = MySqlQueryBuilder::new();
/// let stmt = Query::select()
///     .column("id")
///     .from("users");
///
/// let (sql, values) = builder.build_select(&stmt);
/// // sql: SELECT `id` FROM `users`
/// ```
///
/// # Limitations
///
/// MySQL has the following limitations compared to PostgreSQL:
/// - RETURNING clause is not supported (will panic if used)
/// - FULL OUTER JOIN is not supported (will panic if used in future implementations)
#[derive(Debug, Clone, Default)]
pub struct MySqlQueryBuilder;

impl MySqlQueryBuilder {
	/// Create a new MySQL query builder
	pub fn new() -> Self {
		Self
	}

	/// Escape an identifier for MySQL
	///
	/// MySQL uses backticks for identifiers.
	///
	/// # Arguments
	///
	/// * `ident` - The identifier to escape
	///
	/// # Returns
	///
	/// The escaped identifier (e.g., `` `user` ``)
	fn escape_iden(&self, ident: &str) -> String {
		// Escape backticks within the identifier
		let escaped = ident.replace('`', "``");
		format!("`{}`", escaped)
	}

	/// Format a placeholder for MySQL
	///
	/// MySQL uses question mark placeholders (`?`).
	///
	/// # Arguments
	///
	/// * `_index` - The parameter index (ignored for MySQL)
	///
	/// # Returns
	///
	/// The placeholder string (always `?`)
	#[allow(clippy::unused_self)]
	fn placeholder(&self, _index: usize) -> String {
		"?".to_string()
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
				writer.push_value(value.clone(), |_i| self.placeholder(0));
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

				// MySQL uses ? placeholders, no adjustment needed
				writer.push(&subquery_sql);
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
		use crate::types::{JoinOn, JoinType};

		// MySQL does not support FULL OUTER JOIN
		if matches!(join.join, JoinType::FullOuterJoin) {
			panic!(
				"MySQL does not support FULL OUTER JOIN. Use LEFT JOIN and RIGHT JOIN with UNION instead."
			);
		}

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

	/// Write a window statement (PARTITION BY ... ORDER BY ... frame_clause)
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

	/// Write a frame clause (ROWS/RANGE BETWEEN ... AND ...)
	fn write_frame_clause(&self, writer: &mut SqlWriter, frame: &crate::types::FrameClause) {
		use crate::types::FrameType;

		// Frame type (ROWS, RANGE)
		match frame.frame_type {
			FrameType::Rows => writer.push_keyword("ROWS"),
			FrameType::Range => writer.push_keyword("RANGE"),
			FrameType::Groups => {
				panic!("MySQL does not support GROUPS frame type. Use ROWS or RANGE instead.");
			}
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

impl QueryBuilder for MySqlQueryBuilder {
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

				// MySQL uses ? placeholders, no adjustment needed
				w.push(&cte_sql);
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
					// SELECT ALL - explicit but not required in MySQL
				}
				SelectDistinct::Distinct => {
					writer.push_keyword("DISTINCT");
					writer.push_space();
				}
				SelectDistinct::DistinctRow => {
					writer.push_keyword("DISTINCTROW");
					writer.push_space();
				}
				SelectDistinct::DistinctOn(_cols) => {
					panic!("MySQL does not support DISTINCT ON. Use DISTINCT instead.");
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

		// WINDOW clause (named windows)
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
			writer.push_value(limit.clone(), |_i| self.placeholder(0));
		}

		// OFFSET clause
		if let Some(offset) = &stmt.offset {
			writer.push_keyword("OFFSET");
			writer.push_space();
			writer.push_value(offset.clone(), |_i| self.placeholder(0));
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
					panic!("MySQL does not support INTERSECT. Use UNION with DISTINCT instead.");
				}
				UnionType::Except => {
					panic!("MySQL does not support EXCEPT. Use LEFT JOIN with IS NULL instead.");
				}
			}
			writer.push_space();

			// Recursively build the union query
			let (union_sql, union_values) = self.build_select(union_stmt);

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
					w2.push_value(value.clone(), |_i| self.placeholder(0));
				});
				w.push(")");
			});
		}

		// RETURNING clause - NOT SUPPORTED in MySQL
		if stmt.returning.is_some() {
			panic!("MySQL does not support RETURNING clause. Use LAST_INSERT_ID() instead.");
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
				w.push_value(value.clone(), |_i| self.placeholder(0));
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

		// RETURNING clause - NOT SUPPORTED in MySQL
		if stmt.returning.is_some() {
			panic!("MySQL does not support RETURNING clause.");
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

		// RETURNING clause - NOT SUPPORTED in MySQL
		if stmt.returning.is_some() {
			panic!("MySQL does not support RETURNING clause.");
		}

		writer.finish()
	}

	fn build_create_table(&self, stmt: &CreateTableStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// CREATE TABLE
		writer.push("CREATE TABLE");
		writer.push_space();

		// IF NOT EXISTS clause
		if stmt.if_not_exists {
			writer.push_keyword("IF NOT EXISTS");
			writer.push_space();
		}

		// Table name
		if let Some(table) = &stmt.table {
			self.write_table_ref(&mut writer, table);
		}

		writer.push_space();
		writer.push("(");

		// Column definitions
		let mut first = true;
		for column in &stmt.columns {
			if !first {
				writer.push(", ");
			}
			first = false;

			// Column name
			writer.push_identifier(&column.name.to_string(), |s| self.escape_iden(s));
			writer.push_space();

			// Column type
			if let Some(col_type) = &column.column_type {
				writer.push(&self.column_type_to_sql(col_type));
			}

			// NOT NULL
			if column.not_null {
				writer.push(" NOT NULL");
			}

			// AUTO_INCREMENT
			if column.auto_increment {
				writer.push(" AUTO_INCREMENT");
			}

			// UNIQUE
			if column.unique {
				writer.push(" UNIQUE");
			}

			// PRIMARY KEY
			if column.primary_key {
				writer.push(" PRIMARY KEY");
			}

			// DEFAULT
			if let Some(default) = &column.default {
				writer.push(" DEFAULT ");
				self.write_simple_expr(&mut writer, default);
			}

			// CHECK constraint
			if let Some(check) = &column.check {
				writer.push(" CHECK (");
				self.write_simple_expr(&mut writer, check);
				writer.push(")");
			}

			// COMMENT
			if let Some(comment) = &column.comment {
				writer.push(" COMMENT ");
				writer.push_value(comment.clone().into(), |_| "?".to_string());
			}
		}

		// Table constraints
		for constraint in &stmt.constraints {
			writer.push(", ");
			self.write_table_constraint(&mut writer, constraint);
		}

		writer.push(")");

		writer.finish()
	}

	fn build_alter_table(&self, stmt: &AlterTableStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// ALTER TABLE table_name
		writer.push("ALTER TABLE");
		writer.push_space();
		if let Some(table) = &stmt.table {
			self.write_table_ref(&mut writer, table);
		}

		// Process operations
		let mut first = true;
		for operation in &stmt.operations {
			if !first {
				writer.push(",");
			}
			first = false;
			writer.push_space();

			match operation {
				AlterTableOperation::AddColumn(column_def) => {
					writer.push("ADD COLUMN");
					writer.push_space();
					writer.push_identifier(&column_def.name.to_string(), |s| self.escape_iden(s));
					writer.push_space();
					if let Some(col_type) = &column_def.column_type {
						writer.push(&self.column_type_to_sql(col_type));
					}
					if column_def.not_null {
						writer.push(" NOT NULL");
					}
					if column_def.unique {
						writer.push(" UNIQUE");
					}
					if column_def.auto_increment {
						writer.push(" AUTO_INCREMENT");
					}
					if let Some(default) = &column_def.default {
						writer.push(" DEFAULT ");
						self.write_simple_expr(&mut writer, default);
					}
					if let Some(check) = &column_def.check {
						writer.push(" CHECK (");
						self.write_simple_expr(&mut writer, check);
						writer.push(")");
					}
					if let Some(comment) = &column_def.comment {
						writer.push(" COMMENT ");
						writer.push_value(comment.clone().into(), |_| "?".to_string());
					}
				}
				AlterTableOperation::DropColumn { name, if_exists } => {
					writer.push("DROP COLUMN");
					writer.push_space();
					if *if_exists {
						writer.push("IF EXISTS");
						writer.push_space();
					}
					writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
				}
				AlterTableOperation::RenameColumn { old, new } => {
					// MySQL 8.0+ supports RENAME COLUMN
					writer.push("RENAME COLUMN");
					writer.push_space();
					writer.push_identifier(&old.to_string(), |s| self.escape_iden(s));
					writer.push_space();
					writer.push("TO");
					writer.push_space();
					writer.push_identifier(&new.to_string(), |s| self.escape_iden(s));
				}
				AlterTableOperation::ModifyColumn(column_def) => {
					// MySQL uses MODIFY COLUMN instead of ALTER COLUMN
					writer.push("MODIFY COLUMN");
					writer.push_space();
					writer.push_identifier(&column_def.name.to_string(), |s| self.escape_iden(s));
					writer.push_space();
					if let Some(col_type) = &column_def.column_type {
						writer.push(&self.column_type_to_sql(col_type));
					}
					if column_def.not_null {
						writer.push(" NOT NULL");
					}
					if let Some(default) = &column_def.default {
						writer.push(" DEFAULT ");
						self.write_simple_expr(&mut writer, default);
					}
				}
				AlterTableOperation::AddConstraint(constraint) => {
					writer.push("ADD ");
					self.write_table_constraint(&mut writer, constraint);
				}
				AlterTableOperation::DropConstraint { name, if_exists } => {
					writer.push("DROP CONSTRAINT");
					writer.push_space();
					if *if_exists {
						writer.push("IF EXISTS");
						writer.push_space();
					}
					writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
				}
				AlterTableOperation::RenameTable(new_name) => {
					writer.push("RENAME TO");
					writer.push_space();
					writer.push_identifier(&new_name.to_string(), |s| self.escape_iden(s));
				}
			}
		}

		writer.finish()
	}

	fn build_drop_table(&self, stmt: &DropTableStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// DROP TABLE
		writer.push("DROP TABLE");
		writer.push_space();

		// IF EXISTS clause
		if stmt.if_exists {
			writer.push_keyword("IF EXISTS");
			writer.push_space();
		}

		// Table names
		writer.push_list(&stmt.tables, ", ", |w, table_ref| {
			self.write_table_ref(w, table_ref);
		});

		// Note: MySQL does not support CASCADE/RESTRICT for DROP TABLE

		writer.finish()
	}

	fn build_create_index(&self, stmt: &CreateIndexStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// CREATE UNIQUE INDEX IF NOT EXISTS
		writer.push("CREATE");
		writer.push_space();
		if stmt.unique {
			writer.push_keyword("UNIQUE");
			writer.push_space();
		}
		writer.push_keyword("INDEX");
		writer.push_space();

		// IF NOT EXISTS (MySQL 5.7.4+)
		if stmt.if_not_exists {
			writer.push_keyword("IF NOT EXISTS");
			writer.push_space();
		}

		// Index name
		if let Some(name) = &stmt.name {
			writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
			writer.push_space();
		}

		// ON table
		writer.push_keyword("ON");
		writer.push_space();
		if let Some(table) = &stmt.table {
			self.write_table_ref(&mut writer, table);
		}
		writer.push_space();

		// (column1 ASC, column2 DESC, ...)
		writer.push("(");
		let mut first = true;
		for col in &stmt.columns {
			if !first {
				writer.push(", ");
			}
			first = false;
			writer.push_identifier(&col.name.to_string(), |s| self.escape_iden(s));
			if let Some(order) = &col.order {
				writer.push_space();
				match order {
					crate::types::Order::Asc => writer.push("ASC"),
					crate::types::Order::Desc => writer.push("DESC"),
				}
			}
		}
		writer.push(")");

		// USING method (MySQL: BTREE, HASH, FULLTEXT)
		if let Some(method) = &stmt.using {
			writer.push_space();
			writer.push_keyword("USING");
			writer.push_space();
			writer.push(self.index_method_to_sql(method));
		}

		// WHERE clause is NOT supported in MySQL - ignore stmt.where

		writer.finish()
	}

	fn build_drop_index(&self, stmt: &DropIndexStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// DROP INDEX index_name ON table_name
		writer.push("DROP INDEX");
		writer.push_space();

		// Index name
		if let Some(name) = &stmt.name {
			writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
		}

		// ON table_name (required in MySQL)
		if let Some(table) = &stmt.table {
			writer.push_space();
			writer.push_keyword("ON");
			writer.push_space();
			self.write_table_ref(&mut writer, table);
		}

		writer.finish()
	}

	fn build_create_view(&self, stmt: &CreateViewStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// MySQL does not support MATERIALIZED views
		if stmt.materialized {
			panic!("MySQL does not support MATERIALIZED views");
		}

		writer.push("CREATE");

		if stmt.or_replace {
			writer.push_keyword("OR REPLACE");
		}

		writer.push_keyword("VIEW");

		if stmt.if_not_exists {
			writer.push_keyword("IF NOT EXISTS");
		}

		if let Some(name) = &stmt.name {
			writer.push_space();
			writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
		}

		if !stmt.columns.is_empty() {
			writer.push_space();
			writer.push("(");
			writer.push_list(stmt.columns.iter(), ", ", |w, col| {
				w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
			});
			writer.push(")");
		}

		writer.push_keyword("AS");

		if let Some(select) = &stmt.select {
			let (select_sql, select_values) = self.build_select(select);
			writer.push_space();
			writer.push(&select_sql);
			writer.append_values(&select_values);
		}

		writer.finish()
	}

	fn build_drop_view(&self, stmt: &DropViewStatement) -> (String, Values) {
		let mut writer = SqlWriter::new();

		// MySQL does not support MATERIALIZED views
		if stmt.materialized {
			panic!("MySQL does not support MATERIALIZED views");
		}

		// MySQL does not support CASCADE/RESTRICT for DROP VIEW
		if stmt.cascade || stmt.restrict {
			panic!("MySQL does not support CASCADE/RESTRICT for DROP VIEW");
		}

		writer.push("DROP");
		writer.push_keyword("VIEW");

		if stmt.if_exists {
			writer.push_keyword("IF EXISTS");
		}

		writer.push_space();
		writer.push_list(stmt.names.iter(), ", ", |w, name| {
			w.push_identifier(&name.to_string(), |s| self.escape_iden(s));
		});

		writer.finish()
	}

	fn build_truncate_table(&self, stmt: &TruncateTableStatement) -> (String, Values) {
		// MySQL does not support truncating multiple tables in a single statement
		if stmt.tables.len() > 1 {
			panic!(
				"MySQL does not support truncating multiple tables in a single TRUNCATE statement"
			);
		}

		// MySQL does not support RESTART IDENTITY, CASCADE, or RESTRICT for TRUNCATE TABLE
		if stmt.restart_identity {
			panic!("MySQL does not support RESTART IDENTITY for TRUNCATE TABLE");
		}
		if stmt.cascade {
			panic!("MySQL does not support CASCADE for TRUNCATE TABLE");
		}
		if stmt.restrict {
			panic!("MySQL does not support RESTRICT for TRUNCATE TABLE");
		}

		let mut writer = SqlWriter::new();

		// TRUNCATE TABLE
		writer.push("TRUNCATE TABLE");
		writer.push_space();

		// Table name (single table only)
		if let Some(table_ref) = stmt.tables.first() {
			self.write_table_ref(&mut writer, table_ref);
		}

		writer.finish()
	}

	fn build_create_trigger(&self, stmt: &CreateTriggerStatement) -> (String, Values) {
		use crate::types::{TriggerBody, TriggerEvent, TriggerOrder, TriggerScope, TriggerTiming};

		// MySQL only supports a single event per trigger
		if stmt.events.len() > 1 {
			panic!("MySQL does not support multiple events in a single trigger");
		}

		// MySQL does not support INSTEAD OF triggers
		if matches!(stmt.timing, Some(TriggerTiming::InsteadOf)) {
			panic!("MySQL does not support INSTEAD OF triggers");
		}

		// MySQL only supports FOR EACH ROW
		if matches!(stmt.scope, Some(TriggerScope::Statement)) {
			panic!("MySQL only supports FOR EACH ROW triggers");
		}

		// MySQL does not support WHEN clause
		if stmt.when_condition.is_some() {
			panic!("MySQL does not support WHEN clause in triggers");
		}

		let mut writer = SqlWriter::new();

		// CREATE TRIGGER
		writer.push("CREATE TRIGGER");

		// Trigger name
		if let Some(name) = &stmt.name {
			writer.push_space();
			writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
		}

		// Timing: BEFORE / AFTER
		if let Some(timing) = stmt.timing {
			writer.push_space();
			match timing {
				TriggerTiming::Before => writer.push("BEFORE"),
				TriggerTiming::After => writer.push("AFTER"),
				TriggerTiming::InsteadOf => unreachable!(), // Already checked above
			}
		}

		// Event: INSERT / UPDATE / DELETE (single event only)
		if let Some(event) = stmt.events.first() {
			writer.push_space();
			match event {
				TriggerEvent::Insert => writer.push("INSERT"),
				TriggerEvent::Update { columns } => {
					writer.push("UPDATE");
					if columns.is_some() {
						panic!("MySQL does not support UPDATE OF columns syntax");
					}
				}
				TriggerEvent::Delete => writer.push("DELETE"),
			}
		}

		// ON table
		writer.push_keyword("ON");
		if let Some(table) = &stmt.table {
			writer.push_space();
			self.write_table_ref(&mut writer, table);
		}

		// FOR EACH ROW
		writer.push_keyword("FOR EACH ROW");

		// FOLLOWS / PRECEDES (MySQL-specific)
		if let Some(order) = &stmt.order {
			writer.push_space();
			match order {
				TriggerOrder::Follows(trigger_name) => {
					writer.push("FOLLOWS ");
					writer.push_identifier(trigger_name.as_str(), |s| self.escape_iden(s));
				}
				TriggerOrder::Precedes(trigger_name) => {
					writer.push("PRECEDES ");
					writer.push_identifier(trigger_name.as_str(), |s| self.escape_iden(s));
				}
			}
		}

		// BEGIN ... END block
		if let Some(body) = &stmt.body {
			writer.push_space();
			match body {
				TriggerBody::Single(sql) => {
					writer.push("BEGIN ");
					writer.push(sql.as_str());
					writer.push("; END");
				}
				TriggerBody::Multiple(statements) => {
					writer.push("BEGIN ");
					for (i, stmt) in statements.iter().enumerate() {
						if i > 0 {
							writer.push(" ");
						}
						writer.push(stmt);
						writer.push(";");
					}
					writer.push(" END");
				}
				TriggerBody::PostgresFunction(_) => {
					panic!("MySQL does not support EXECUTE FUNCTION syntax");
				}
			}
		}

		writer.finish()
	}

	fn build_drop_trigger(&self, stmt: &DropTriggerStatement) -> (String, Values) {
		// MySQL requires table name for DROP TRIGGER
		if stmt.table.is_none() {
			panic!("MySQL requires table name (ON table) for DROP TRIGGER");
		}

		// MySQL does not support CASCADE/RESTRICT
		if stmt.cascade || stmt.restrict {
			panic!("MySQL does not support CASCADE/RESTRICT for DROP TRIGGER");
		}

		let mut writer = SqlWriter::new();

		// DROP TRIGGER
		writer.push("DROP TRIGGER");

		// IF EXISTS
		if stmt.if_exists {
			writer.push_keyword("IF EXISTS");
		}

		// table.trigger_name (MySQL syntax)
		if let Some(table) = &stmt.table {
			writer.push_space();
			self.write_table_ref(&mut writer, table);
			writer.push(".");
		}

		// Trigger name
		if let Some(name) = &stmt.name {
			writer.push_identifier(&name.to_string(), |s| self.escape_iden(s));
		}

		writer.finish()
	}

	fn build_alter_index(&self, stmt: &AlterIndexStatement) -> (String, Values) {
		use crate::types::Iden;

		// MySQL does not support SET TABLESPACE for indexes
		if stmt.set_tablespace.is_some() {
			panic!("MySQL does not support SET TABLESPACE for indexes");
		}

		// MySQL requires table name for RENAME INDEX
		let table = stmt.table.as_ref().expect("MySQL requires table name for ALTER INDEX RENAME");

		// MySQL only supports RENAME INDEX via ALTER TABLE
		if let Some(ref new_name) = stmt.rename_to {
			let mut writer = SqlWriter::new();

			// ALTER TABLE
			writer.push_keyword("ALTER TABLE");
			writer.push_space();
			writer.push_identifier(&Iden::to_string(table.as_ref()), |s| self.escape_iden(s));

			// RENAME INDEX
			writer.push_space();
			writer.push_keyword("RENAME INDEX");
			writer.push_space();

			// Old index name
			if let Some(ref name) = stmt.name {
				writer.push_identifier(&Iden::to_string(name.as_ref()), |s| self.escape_iden(s));
			} else {
				panic!("ALTER INDEX requires an index name");
			}

			// TO new_name
			writer.push_space();
			writer.push_keyword("TO");
			writer.push_space();
			writer.push_identifier(&Iden::to_string(new_name.as_ref()), |s| self.escape_iden(s));

			writer.finish()
		} else {
			panic!("MySQL ALTER INDEX only supports RENAME operation");
		}
	}

	fn build_reindex(&self, _stmt: &ReindexStatement) -> (String, Values) {
		panic!("MySQL does not support REINDEX. Use OPTIMIZE TABLE or DROP/CREATE INDEX instead.");
	}

	fn escape_identifier(&self, ident: &str) -> String {
		self.escape_iden(ident)
	}

	fn format_placeholder(&self, index: usize) -> String {
		self.placeholder(index)
	}
}

// Helper methods for CREATE TABLE
impl MySqlQueryBuilder {
	/// Convert ColumnType to MySQL SQL type string
	fn column_type_to_sql(&self, col_type: &crate::types::ColumnType) -> String {
		use crate::types::ColumnType;
		use ColumnType::*;

		match col_type {
			Char(len) => format!("CHAR({})", len.unwrap_or(1)),
			String(len) => format!("VARCHAR({})", len.unwrap_or(255)),
			Text => "TEXT".to_string(),
			TinyInteger => "TINYINT".to_string(),
			SmallInteger => "SMALLINT".to_string(),
			Integer => "INT".to_string(),
			BigInteger => "BIGINT".to_string(),
			Float => "FLOAT".to_string(),
			Double => "DOUBLE".to_string(),
			Decimal(precision) => {
				if let Some((p, s)) = precision {
					format!("DECIMAL({}, {})", p, s)
				} else {
					"DECIMAL".to_string()
				}
			}
			Boolean => "TINYINT(1)".to_string(),
			Date => "DATE".to_string(),
			Time => "TIME".to_string(),
			DateTime => "DATETIME".to_string(),
			Timestamp => "TIMESTAMP".to_string(),
			TimestampWithTimeZone => "TIMESTAMP".to_string(), // MySQL TIMESTAMP handles timezone
			Binary(len) => {
				if let Some(l) = len {
					format!("BLOB({})", l)
				} else {
					"BLOB".to_string()
				}
			}
			VarBinary(len) => format!("VARBINARY({})", len),
			Blob => "BLOB".to_string(),
			Uuid => "CHAR(36)".to_string(), // UUID as CHAR(36) in MySQL
			Json => "JSON".to_string(),
			JsonBinary => "JSON".to_string(), // MySQL JSON is binary
			Array(_) => "JSON".to_string(),   // MySQL doesn't have ARRAY, use JSON
			Custom(name) => name.clone(),
		}
	}

	/// Write table constraint to SQL writer
	fn write_table_constraint(
		&self,
		writer: &mut SqlWriter,
		constraint: &crate::types::TableConstraint,
	) {
		use crate::types::TableConstraint;
		use TableConstraint::*;

		match constraint {
			PrimaryKey { name, columns } => {
				if let Some(constraint_name) = name {
					writer.push_keyword("CONSTRAINT");
					writer.push_space();
					writer.push_identifier(&constraint_name.to_string(), |s| self.escape_iden(s));
					writer.push_space();
				}
				writer.push_keyword("PRIMARY KEY");
				writer.push(" (");
				writer.push_list(columns, ", ", |w, col| {
					w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
				});
				writer.push(")");
			}
			ForeignKey {
				name,
				columns,
				ref_table,
				ref_columns,
				on_delete,
				on_update,
			} => {
				if let Some(constraint_name) = name {
					writer.push_keyword("CONSTRAINT");
					writer.push_space();
					writer.push_identifier(&constraint_name.to_string(), |s| self.escape_iden(s));
					writer.push_space();
				}
				writer.push_keyword("FOREIGN KEY");
				writer.push(" (");
				writer.push_list(columns, ", ", |w, col| {
					w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
				});
				writer.push(")");
				writer.push_space();
				writer.push_keyword("REFERENCES");
				writer.push_space();
				self.write_table_ref(writer, ref_table);
				writer.push(" (");
				writer.push_list(ref_columns, ", ", |w, col| {
					w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
				});
				writer.push(")");

				if let Some(action) = on_delete {
					writer.push_space();
					writer.push_keyword("ON DELETE");
					writer.push_space();
					writer.push_keyword(self.foreign_key_action_to_sql(action));
				}

				if let Some(action) = on_update {
					writer.push_space();
					writer.push_keyword("ON UPDATE");
					writer.push_space();
					writer.push_keyword(self.foreign_key_action_to_sql(action));
				}
			}
			Unique { name, columns } => {
				if let Some(constraint_name) = name {
					writer.push_keyword("CONSTRAINT");
					writer.push_space();
					writer.push_identifier(&constraint_name.to_string(), |s| self.escape_iden(s));
					writer.push_space();
				}
				writer.push_keyword("UNIQUE");
				writer.push(" (");
				writer.push_list(columns, ", ", |w, col| {
					w.push_identifier(&col.to_string(), |s| self.escape_iden(s));
				});
				writer.push(")");
			}
			Check { name, expr } => {
				if let Some(constraint_name) = name {
					writer.push_keyword("CONSTRAINT");
					writer.push_space();
					writer.push_identifier(&constraint_name.to_string(), |s| self.escape_iden(s));
					writer.push_space();
				}
				writer.push_keyword("CHECK");
				writer.push(" (");
				self.write_simple_expr(writer, expr);
				writer.push(")");
			}
		}
	}

	/// Convert ForeignKeyAction to SQL keyword
	fn foreign_key_action_to_sql(&self, action: &crate::types::ForeignKeyAction) -> &'static str {
		use crate::types::ForeignKeyAction;
		use ForeignKeyAction::*;

		match action {
			Cascade => "CASCADE",
			Restrict => "RESTRICT",
			SetNull => "SET NULL",
			SetDefault => "SET DEFAULT",
			NoAction => "NO ACTION",
		}
	}

	fn index_method_to_sql(&self, method: &crate::query::IndexMethod) -> &'static str {
		use crate::query::IndexMethod;
		match method {
			IndexMethod::BTree => "BTREE",
			IndexMethod::Hash => "HASH",
			IndexMethod::FullText => "FULLTEXT",
			// MySQL doesn't support GIST, GIN, BRIN, Spatial - use BTREE as default
			IndexMethod::Gist | IndexMethod::Gin | IndexMethod::Brin | IndexMethod::Spatial => {
				"BTREE"
			}
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
		let builder = MySqlQueryBuilder::new();
		assert_eq!(builder.escape_identifier("user"), "`user`");
		assert_eq!(builder.escape_identifier("table_name"), "`table_name`");
	}

	#[test]
	fn test_escape_identifier_with_backticks() {
		let builder = MySqlQueryBuilder::new();
		assert_eq!(builder.escape_identifier("user`name"), "`user``name`");
	}

	#[test]
	fn test_format_placeholder() {
		let builder = MySqlQueryBuilder::new();
		assert_eq!(builder.format_placeholder(1), "?");
		assert_eq!(builder.format_placeholder(2), "?");
		assert_eq!(builder.format_placeholder(10), "?");
	}

	#[test]
	fn test_select_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("id").column("name").from("users");

		let (sql, values) = builder.build_select(&stmt);
		assert_eq!(sql, "SELECT `id`, `name` FROM `users`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_select_asterisk() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.from("users");

		let (sql, values) = builder.build_select(&stmt);
		assert_eq!(sql, "SELECT * FROM `users`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_select_with_where() {
		let builder = MySqlQueryBuilder::new();
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
		let builder = MySqlQueryBuilder::new();
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
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(["name", "email"])
			.values_panic(["Alice", "alice@example.com"]);

		let (sql, values) = builder.build_insert(&stmt);
		assert_eq!(sql, "INSERT INTO `users` (`name`, `email`) VALUES (?, ?)");
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_insert_multiple_rows() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(["name", "email"])
			.values_panic(["Alice", "alice@example.com"])
			.values_panic(["Bob", "bob@example.com"]);

		let (sql, values) = builder.build_insert(&stmt);
		assert_eq!(
			sql,
			"INSERT INTO `users` (`name`, `email`) VALUES (?, ?), (?, ?)"
		);
		assert_eq!(values.len(), 4);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support RETURNING clause")]
	fn test_insert_with_returning_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::insert();
		stmt.into_table("users")
			.columns(["name"])
			.values_panic(["Alice"])
			.returning(["id", "created_at"]);

		let _ = builder.build_insert(&stmt);
	}

	#[test]
	fn test_update_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::update();
		stmt.table("users")
			.value("name", "Alice")
			.value("email", "alice@example.com");

		let (sql, values) = builder.build_update(&stmt);
		assert_eq!(sql, "UPDATE `users` SET `name` = ?, `email` = ?");
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_update_with_where() {
		let builder = MySqlQueryBuilder::new();
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
	#[should_panic(expected = "MySQL does not support RETURNING clause")]
	fn test_update_with_returning_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::update();
		stmt.table("users")
			.value("active", false)
			.and_where(Expr::col("id").eq(1))
			.returning(["id", "updated_at"]);

		let _ = builder.build_update(&stmt);
	}

	#[test]
	fn test_delete_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::delete();
		stmt.from_table("users")
			.and_where(Expr::col("active").eq(false));

		let (sql, values) = builder.build_delete(&stmt);
		assert!(sql.contains("DELETE FROM"));
		assert!(sql.contains("`users`"));
		assert!(sql.contains("WHERE"));
		assert_eq!(values.len(), 1); // false
	}

	#[test]
	fn test_delete_no_where() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::delete();
		stmt.from_table("users");

		let (sql, values) = builder.build_delete(&stmt);
		assert_eq!(sql, "DELETE FROM `users`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support RETURNING clause")]
	fn test_delete_with_returning_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::delete();
		stmt.from_table("users")
			.and_where(Expr::col("id").eq(1))
			.returning(["id", "name"]);

		let _ = builder.build_delete(&stmt);
	}

	// JOIN tests

	#[test]
	fn test_inner_join_simple() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.from("users")
			.inner_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("FROM `users`"));
		assert!(sql.contains("INNER JOIN `orders`"));
		assert!(sql.contains("ON `users`.`id` = `orders`.`user_id`"));
	}

	#[test]
	fn test_left_join() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("profiles.bio")
			.from("users")
			.left_join(
				"profiles",
				Expr::col(("users", "id")).eq(Expr::col(("profiles", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("LEFT JOIN `profiles`"));
		assert!(sql.contains("ON `users`.`id` = `profiles`.`user_id`"));
	}

	#[test]
	fn test_right_join() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.from("users")
			.right_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("RIGHT JOIN `orders`"));
		assert!(sql.contains("ON `users`.`id` = `orders`.`user_id`"));
	}

	#[test]
	#[should_panic(expected = "MySQL does not support FULL OUTER JOIN")]
	fn test_full_outer_join_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("orders.amount")
			.from("users")
			.full_outer_join(
				"orders",
				Expr::col(("users", "id")).eq(Expr::col(("orders", "user_id"))),
			);

		let _ = builder.build_select(&stmt);
	}

	#[test]
	fn test_cross_join() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("users.name")
			.column("roles.title")
			.from("users")
			.cross_join("roles");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("CROSS JOIN `roles`"));
		assert!(!sql.contains("ON"));
	}

	#[test]
	fn test_multiple_joins() {
		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("INNER JOIN `orders`"));
		assert!(sql.contains("INNER JOIN `products`"));
		assert!(sql.contains("`users`.`id` = `orders`.`user_id`"));
		assert!(sql.contains("`orders`.`product_id` = `products`.`id`"));
	}

	#[test]
	fn test_join_with_complex_condition() {
		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("INNER JOIN `orders`"));
		assert!(sql.contains("ON"));
		assert!(sql.contains("`users`.`id` = `orders`.`user_id`"));
		assert!(sql.contains("AND"));
		assert!(sql.contains("`orders`.`status` = ?"));
		assert_eq!(values.len(), 1);
	}

	// GROUP BY / HAVING tests

	#[test]
	fn test_group_by_single_column() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("category")
			.from("products")
			.group_by("category");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("GROUP BY `category`"));
	}

	#[test]
	fn test_group_by_multiple_columns() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("category")
			.column("brand")
			.from("products")
			.group_by("category")
			.group_by("brand");

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("GROUP BY `category`, `brand`"));
	}

	#[test]
	fn test_group_by_with_count() {
		use crate::expr::SimpleExpr;
		use crate::types::{ColumnRef, IntoIden};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("GROUP BY `category`"));
	}

	#[test]
	fn test_having_simple() {
		use crate::expr::SimpleExpr;
		use crate::types::{BinOper, IntoIden};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("GROUP BY `category`"));
		assert!(sql.contains("HAVING"));
		assert!(sql.contains("COUNT(*)"));
		assert!(sql.contains(">"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_group_by_having_with_sum() {
		use crate::expr::SimpleExpr;
		use crate::types::{BinOper, ColumnRef, IntoIden};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("SUM(`amount`)"));
		assert!(sql.contains("GROUP BY `user_id`"));
		assert!(sql.contains("HAVING"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_select_distinct() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("category").from("products").distinct();

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.starts_with("SELECT DISTINCT"));
		assert!(sql.contains("`category`"));
		assert!(sql.contains("FROM `products`"));
	}

	#[test]
	fn test_select_distinctrow() {
		use crate::query::SelectDistinct;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("id").column("name").from("users");
		stmt.distinct = Some(SelectDistinct::DistinctRow);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("SELECT DISTINCTROW"));
		assert!(sql.contains("`id`"));
		assert!(sql.contains("`name`"));
	}

	#[test]
	#[should_panic(expected = "MySQL does not support DISTINCT ON")]
	fn test_select_distinct_on_panics() {
		use crate::query::SelectDistinct;
		use crate::types::ColumnRef;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("products");
		stmt.distinct = Some(SelectDistinct::DistinctOn(vec![ColumnRef::column(
			"category",
		)]));

		let _ = builder.build_select(&stmt);
	}

	#[test]
	fn test_select_union() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("id").from("users");

		let mut stmt2 = Query::select();
		stmt2.column("id").from("customers");

		stmt1.union(stmt2);

		let (sql, _values) = builder.build_select(&stmt1);
		assert!(sql.contains("SELECT `id` FROM `users`"));
		assert!(sql.contains("UNION SELECT `id` FROM `customers`"));
	}

	#[test]
	fn test_select_union_all() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("name").from("products");

		let mut stmt2 = Query::select();
		stmt2.column("name").from("archived_products");

		stmt1.union_all(stmt2);

		let (sql, _values) = builder.build_select(&stmt1);
		assert!(sql.contains("SELECT `name` FROM `products`"));
		assert!(sql.contains("UNION ALL SELECT `name` FROM `archived_products`"));
	}

	#[test]
	#[should_panic(expected = "MySQL does not support INTERSECT")]
	fn test_select_intersect_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("email").from("subscribers");

		let mut stmt2 = Query::select();
		stmt2.column("email").from("customers");

		stmt1.intersect(stmt2);

		let _ = builder.build_select(&stmt1);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support EXCEPT")]
	fn test_select_except_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt1 = Query::select();
		stmt1.column("id").from("all_users");

		let mut stmt2 = Query::select();
		stmt2.column("id").from("banned_users");

		stmt1.except(stmt2);

		let _ = builder.build_select(&stmt1);
	}

	// --- Phase 5: Subquery Edge Case Tests ---

	#[test]
	fn test_not_in_subquery() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("SELECT `user_id` FROM `blocked_users`"));
		assert!(sql.contains("`reason` = ?"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_subquery_in_select_list() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`name`"));
		assert!(sql.contains("(SELECT `count` FROM `order_counts`"));
		assert!(sql.contains("`order_counts`.`user_id` = `users`.`id`"));
	}

	#[test]
	fn test_multiple_exists_conditions() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("EXISTS (SELECT `id` FROM `orders`"));
		assert!(sql.contains("EXISTS (SELECT `id` FROM `reviews`"));
	}

	#[test]
	fn test_nested_subquery() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("IN (SELECT `id` FROM `employees`"));
		assert!(sql.contains("IN (SELECT `department_id` FROM `top_departments`"));
		assert!(sql.contains("`revenue` > ?"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_subquery_with_complex_where() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("IN (SELECT `product_id` FROM `inventory`"));
		assert!(sql.contains("`quantity` > ?"));
		assert!(sql.contains("`warehouse` = ?"));
		assert!(sql.contains("`status` = ?"));
		assert!(sql.contains("`active` = ?"));
		assert_eq!(values.len(), 4); // 0, "main", "available", true
	}

	// --- Phase 5: NULL Handling Tests ---

	#[test]
	fn test_where_is_null() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("deleted_at").is_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`deleted_at` IS"));
		assert!(sql.to_uppercase().contains("NULL"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_where_is_not_null() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("email").is_not_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`email` IS NOT"));
		assert!(sql.to_uppercase().contains("NULL"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_is_null_combined_with_other_conditions() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("active").eq(true))
			.and_where(Expr::col("deleted_at").is_null())
			.and_where(Expr::col("email").is_not_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`active` = ?"));
		assert!(sql.contains("`deleted_at` IS"));
		assert!(sql.contains("`email` IS NOT"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_is_null_with_join() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("users", "name"))
			.from("users")
			.left_join(
				"profiles",
				Expr::col(("users", "id")).eq(Expr::col(("profiles", "user_id"))),
			)
			.and_where(Expr::col(("profiles", "id")).is_null());

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LEFT JOIN `profiles`"));
		assert!(sql.contains("`profiles`.`id` IS"));
		assert_eq!(values.len(), 0);
	}

	// --- Phase 5: Complex WHERE Clause Tests ---
	#[test]
	fn test_where_or_condition() {
		use crate::expr::Condition;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("users").cond_where(
			Condition::any()
				.add(Expr::col("status").eq("active"))
				.add(Expr::col("status").eq("pending")),
		);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`status` = ?"));
		assert!(sql.contains(" OR "));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_where_between() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("products")
			.and_where(Expr::col("price").between(100, 500));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`price` BETWEEN ?"));
		assert!(sql.contains("AND ?"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_where_not_between() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("products")
			.and_where(Expr::col("price").not_between(0, 10));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`price` NOT BETWEEN ?"));
		assert!(sql.contains("AND ?"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_where_like() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("email").like("%@gmail.com"));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`email` LIKE ?"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_where_in_values() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name")
			.from("users")
			.and_where(Expr::col("role").is_in(vec!["admin", "moderator", "editor"]));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`role` IN"));
		assert_eq!(values.len(), 3);
	}

	#[test]
	fn test_insert_with_null_value() {
		use crate::value::Value;

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("INSERT INTO `users`"));
		assert!(sql.contains("`name`"));
		assert!(sql.contains("`email`"));
		assert!(sql.contains("`phone`"));
		assert_eq!(values.len(), 3);
	}

	#[test]
	fn test_select_with_single_cte() {
		// Note: CTE (WITH clause) is supported in MySQL 8.0+
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`eng_employees`"));
		assert!(sql.contains("AS"));
		assert!(sql.contains("SELECT `id`, `name` FROM `employees`"));
		assert!(sql.contains("SELECT `name` FROM `eng_employees`"));
	}

	#[test]
	fn test_select_with_multiple_ctes() {
		// Note: CTE (WITH clause) is supported in MySQL 8.0+
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`eng_emp`"));
		assert!(sql.contains("`sales_emp`"));
		assert!(sql.contains("AS"));
		// Both CTEs should be present
		assert!(sql.contains("`eng_emp` AS"));
		assert!(sql.contains("`sales_emp` AS"));
	}

	#[test]
	fn test_select_with_recursive_cte() {
		// Note: Recursive CTE is supported in MySQL 8.0+
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`employee_hierarchy`"));
		assert!(sql.contains("AS"));
		assert!(sql.contains("SELECT `id`, `name`, `manager_id` FROM `employees`"));
		assert!(sql.contains("SELECT `name` FROM `employee_hierarchy`"));
	}

	// Window function tests

	#[test]
	fn test_window_row_number_with_partition_and_order() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			r#"SELECT ROW_NUMBER() OVER ( PARTITION BY `department` ORDER BY `salary` DESC ), `name` FROM `employees`"#
		);
	}

	#[test]
	fn test_window_rank_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			r#"SELECT RANK() OVER ( ORDER BY `score` DESC ), `name` FROM `students`"#
		);
	}

	#[test]
	fn test_window_dense_rank_with_partition() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			r#"SELECT DENSE_RANK() OVER ( PARTITION BY `league` ORDER BY `points` DESC ), `player` FROM `scores`"#
		);
	}

	#[test]
	fn test_window_ntile_four_buckets() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			r#"SELECT NTILE(?) OVER ( ORDER BY `salary` ASC ), `name` FROM `employees`"#
		);
	}

	#[test]
	fn test_window_lead_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			r#"SELECT LEAD(`price`) OVER ( ORDER BY `date` ASC ), `date` FROM `stocks`"#
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_window_lag_with_offset_and_default() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
				Some(2),
				Some(0.0.into()),
			)
			.over(window),
		)
		.column("month")
		.from("sales");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LAG"));
		assert!(sql.contains("OVER"));
		assert!(sql.contains("PARTITION BY `product`"));
		assert!(sql.contains("ORDER BY `month` ASC"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_window_first_value() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			r#"SELECT FIRST_VALUE(`name`) OVER ( PARTITION BY `category` ORDER BY `price` ASC ), `name` FROM `products`"#
		);
	}

	#[test]
	fn test_window_last_value() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			r#"SELECT LAST_VALUE(`name`) OVER ( PARTITION BY `category` ORDER BY `price` DESC ), `name` FROM `products`"#
		);
	}

	#[test]
	fn test_window_nth_value() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("PARTITION BY `department`"));
		assert!(sql.contains("ORDER BY `salary` DESC"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_window_row_number_order_only() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT ROW_NUMBER() OVER ( ORDER BY `id` ASC ) FROM `users`"
		);
	}

	#[test]
	fn test_window_rank_order_only() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT RANK() OVER ( ORDER BY `score` DESC ), `name` FROM `students`"
		);
	}

	#[test]
	fn test_window_rank_with_partition() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT RANK() OVER ( PARTITION BY `class` ORDER BY `score` DESC ), `name` FROM `students`"
		);
	}

	#[test]
	fn test_window_dense_rank_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT DENSE_RANK() OVER ( ORDER BY `points` DESC ), `player` FROM `scores`"
		);
	}

	#[test]
	fn test_window_ntile_custom_buckets() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT NTILE(?) OVER ( PARTITION BY `department` ORDER BY `salary` DESC ), `name` FROM `employees`"
		);
	}

	#[test]
	fn test_window_lead_with_offset_and_default() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("PARTITION BY `ticker`"));
		assert!(sql.contains("ORDER BY `date` ASC"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_window_lag_basic() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT LAG(`revenue`) OVER ( ORDER BY `month` ASC ), `month` FROM `sales`"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_window_lag_with_different_offset() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("PARTITION BY `product`"));
		assert!(sql.contains("ORDER BY `month` ASC"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_window_first_value_with_partition() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT FIRST_VALUE(`name`) OVER ( PARTITION BY `category` ORDER BY `price` ASC ), `name` FROM `products`"
		);
	}

	#[test]
	fn test_window_last_value_with_partition() {
		use crate::types::{Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
			"SELECT LAST_VALUE(`name`) OVER ( PARTITION BY `category` ORDER BY `price` DESC ), `name` FROM `products`"
		);
	}

	// --- Phase 5: JOIN Enhancement Tests ---

	#[test]
	fn test_join_three_tables() {
		let builder = MySqlQueryBuilder::new();
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
			"SELECT `users`.`name`, `orders`.`order_date`, `products`.`product_name` FROM `users` INNER JOIN `orders` ON `users`.`id` = `orders`.`user_id` INNER JOIN `products` ON `orders`.`product_id` = `products`.`id`"
		);
	}

	#[test]
	fn test_self_join() {
		use crate::types::TableRef;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column(("e1", "name"))
			.column(("e2", "name"))
			.from(TableRef::table_alias("employees", "e1"))
			.inner_join(
				TableRef::table_alias("employees", "e2"),
				Expr::col(("e1", "manager_id")).eq(Expr::col(("e2", "id"))),
			);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("FROM `employees` AS `e1`"));
		assert!(sql.contains("INNER JOIN `employees` AS `e2`"));
		assert!(sql.contains("ON `e1`.`manager_id` = `e2`.`id`"));
	}

	#[test]
	fn test_join_complex_conditions() {
		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("LEFT JOIN `customers`"));
		assert!(sql.contains("`orders`.`customer_id` = `customers`.`id`"));
		assert!(sql.contains("AND `customers`.`active` = ?"));
		assert!(sql.contains("AND `orders`.`created_at` > `customers`.`registered_at`"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_join_with_subquery_in_condition() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("INNER JOIN `profiles`"));
		assert!(sql.contains("`users`.`id` = `profiles`.`user_id`"));
		assert!(sql.contains("IN"));
		assert!(sql.contains("SELECT `max_id` FROM `user_stats`"));
	}

	#[test]
	fn test_multiple_left_joins() {
		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("LEFT JOIN `profiles`"));
		assert!(sql.contains("LEFT JOIN `addresses`"));
		assert!(sql.contains("LEFT JOIN `phone_numbers`"));
	}

	#[test]
	fn test_mixed_join_types() {
		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("INNER JOIN `orders`"));
		assert!(sql.contains("LEFT JOIN `reviews`"));
		assert!(sql.contains("RIGHT JOIN `refunds`"));
	}

	#[test]
	fn test_join_with_group_by() {
		use crate::expr::SimpleExpr;
		use crate::types::{BinOper, ColumnRef, IntoIden};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("INNER JOIN `orders`"));
		assert!(sql.contains("GROUP BY `users`.`name`"));
		assert!(sql.contains("HAVING"));
		assert!(sql.contains("COUNT(*) > ?"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_join_with_window_function() {
		use crate::types::{IntoIden, Order, OrderExpr, OrderExprKind, WindowStatement};

		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("INNER JOIN `departments`"));
		assert!(sql.contains("ROW_NUMBER() OVER"));
		assert!(sql.contains("PARTITION BY `departments`.`name`"));
	}

	#[test]
	fn test_four_table_join() {
		let builder = MySqlQueryBuilder::new();
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
		assert!(sql.contains("FROM `users`"));
		assert!(sql.contains("INNER JOIN `orders`"));
		assert!(sql.contains("INNER JOIN `products`"));
		assert!(sql.contains("INNER JOIN `categories`"));
	}

	#[test]
	fn test_join_with_cte() {
		use crate::types::TableRef;

		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("WITH `high_value_customers` AS"));
		assert!(sql.contains("INNER JOIN `high_value_customers` AS `hvc`"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_cte_with_where_and_params() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`large_orders` AS"));
		assert!(sql.contains("`status` = ?"));
		assert!(sql.contains("`amount` > ?"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_cte_used_in_join() {
		use crate::types::TableRef;

		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`user_orders` AS"));
		assert!(sql.contains("INNER JOIN `user_orders` AS `uo`"));
		assert!(sql.contains("`users`.`id` = `uo`.`user_id`"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_cte_with_aggregation() {
		use crate::expr::SimpleExpr;
		use crate::types::{ColumnRef, IntoIden};

		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`category_stats` AS"));
		assert!(sql.contains("COUNT(*)"));
		assert!(sql.contains("SUM(`price`)"));
		assert!(sql.contains("GROUP BY `category`"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_cte_with_subquery() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`vip_orders` AS"));
		assert!(sql.contains("IN"));
		assert!(sql.contains("SELECT `user_id` FROM `vip_users`"));
		assert!(sql.contains("`status` = ?"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_multiple_recursive_and_regular_ctes() {
		let builder = MySqlQueryBuilder::new();

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
		assert!(sql.contains("`active_depts` AS"));
		assert!(sql.contains("`category_tree` AS"));
		assert!(sql.contains("`active` = ?"));
		assert!(sql.contains("FROM `category_tree`"));
		assert_eq!(values.len(), 1);
	}

	// CASE expression tests

	#[test]
	fn test_case_simple_when_else() {
		let builder = MySqlQueryBuilder::new();

		let case_expr = Expr::case()
			.when(Expr::col("status").eq("active"), "Active")
			.else_result("Inactive");

		let mut stmt = Query::select();
		stmt.expr_as(case_expr, "status_label").from("users");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("CASE"));
		assert!(sql.contains("WHEN"));
		assert!(sql.contains("`status` = ?"));
		assert!(sql.contains("THEN"));
		assert!(sql.contains("ELSE"));
		assert!(sql.contains("END"));
		assert!(sql.contains("AS `status_label`"));
		assert_eq!(values.len(), 3);
	}

	#[test]
	fn test_case_multiple_when_clauses() {
		let builder = MySqlQueryBuilder::new();

		let case_expr = Expr::case()
			.when(Expr::col("score").gte(90), "A")
			.when(Expr::col("score").gte(80), "B")
			.when(Expr::col("score").gte(70), "C")
			.else_result("F");

		let mut stmt = Query::select();
		stmt.expr_as(case_expr, "grade").from("students");

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("CASE"));
		let when_count = sql.matches("WHEN").count();
		assert_eq!(when_count, 3);
		let then_count = sql.matches("THEN").count();
		assert_eq!(then_count, 3);
		assert!(sql.contains("ELSE"));
		assert!(sql.contains("END"));
		assert_eq!(values.len(), 7);
	}

	#[test]
	fn test_case_without_else() {
		let builder = MySqlQueryBuilder::new();

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
		let builder = MySqlQueryBuilder::new();

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
		let builder = MySqlQueryBuilder::new();

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
	fn test_order_by_desc_multiple() {
		let builder = MySqlQueryBuilder::new();

		let mut stmt = Query::select();
		stmt.column("name")
			.column("created_at")
			.from("posts")
			.order_by("created_at", crate::types::Order::Desc)
			.order_by("name", crate::types::Order::Asc);

		let (sql, _values) = builder.build_select(&stmt);
		assert!(sql.contains("ORDER BY"));
		assert!(sql.contains("`created_at` DESC"));
		assert!(sql.contains("`name` ASC"));
	}

	#[test]
	fn test_limit_zero() {
		let builder = MySqlQueryBuilder::new();

		let mut stmt = Query::select();
		stmt.column("id").from("items").limit(0);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("LIMIT"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_combined_where_order_limit_offset() {
		let builder = MySqlQueryBuilder::new();

		let mut stmt = Query::select();
		stmt.column("id")
			.column("name")
			.from("users")
			.and_where(Expr::col("active").eq(true))
			.order_by("name", crate::types::Order::Asc)
			.limit(10)
			.offset(20);

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("WHERE"));
		assert!(sql.contains("`active` = ?"));
		assert!(sql.contains("ORDER BY"));
		assert!(sql.contains("`name` ASC"));
		assert!(sql.contains("LIMIT"));
		assert!(sql.contains("OFFSET"));
		assert_eq!(values.len(), 3);
	}

	// Arithmetic / string operation tests

	#[test]
	fn test_arithmetic_in_where() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("products");
		stmt.and_where(Expr::col("price").sub(Expr::col("discount")).gt(50i32));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`price` - `discount`"));
		assert!(sql.contains("> ?"));
		assert_eq!(values.len(), 1);
	}

	#[test]
	fn test_like_not_like() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("users");
		stmt.and_where(Expr::col("name").like("John%"));
		stmt.and_where(Expr::col("email").not_like("%spam%"));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`name` LIKE ?"));
		assert!(sql.contains("`email` NOT LIKE ?"));
		assert_eq!(values.len(), 2);
	}

	#[test]
	fn test_between_values() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::select();
		stmt.column("name").from("products");
		stmt.and_where(Expr::col("price").between(10i32, 100i32));

		let (sql, values) = builder.build_select(&stmt);
		assert!(sql.contains("`price` BETWEEN ? AND ?"));
		assert_eq!(values.len(), 2);
	}

	// DDL Tests

	#[test]
	fn test_drop_table_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_table();
		stmt.table("users");

		let (sql, values) = builder.build_drop_table(&stmt);
		assert_eq!(sql, "DROP TABLE `users`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_drop_table_if_exists() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_table();
		stmt.table("users").if_exists();

		let (sql, values) = builder.build_drop_table(&stmt);
		assert_eq!(sql, "DROP TABLE IF EXISTS `users`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_drop_table_multiple() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_table();
		stmt.table("users").table("posts");

		let (sql, values) = builder.build_drop_table(&stmt);
		assert_eq!(sql, "DROP TABLE `users`, `posts`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_drop_index_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_index();
		stmt.name("idx_email").table("users");

		let (sql, values) = builder.build_drop_index(&stmt);
		assert_eq!(sql, "DROP INDEX `idx_email` ON `users`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_drop_index_if_exists_not_supported() {
		// Note: MySQL supports IF EXISTS for DROP INDEX, but showing it works
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_index();
		stmt.name("idx_email").table("users").if_exists();

		let (sql, values) = builder.build_drop_index(&stmt);
		// Note: Our implementation doesn't add IF EXISTS for MySQL (not standard)
		assert_eq!(sql, "DROP INDEX `idx_email` ON `users`");
		assert_eq!(values.len(), 0);
	}

	// CREATE TABLE tests

	#[test]
	fn test_create_table_basic() {
		use crate::types::{ColumnDef, ColumnType};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_table();
		stmt.table("users");
		stmt.columns.push(ColumnDef {
			name: "id".into_iden(),
			column_type: Some(ColumnType::Integer),
			not_null: false,
			unique: false,
			primary_key: false,
			auto_increment: false,
			default: None,
			check: None,
			comment: None,
		});
		stmt.columns.push(ColumnDef {
			name: "name".into_iden(),
			column_type: Some(ColumnType::String(Some(255))),
			not_null: false,
			unique: false,
			primary_key: false,
			auto_increment: false,
			default: None,
			check: None,
			comment: None,
		});

		let (sql, values) = builder.build_create_table(&stmt);
		assert!(sql.contains("CREATE TABLE `users`"));
		assert!(sql.contains("`id` INT"));
		assert!(sql.contains("`name` VARCHAR(255)"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_table_with_auto_increment() {
		use crate::types::{ColumnDef, ColumnType};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_table();
		stmt.table("users");
		stmt.columns.push(ColumnDef {
			name: "id".into_iden(),
			column_type: Some(ColumnType::Integer),
			not_null: false,
			unique: false,
			primary_key: true,
			auto_increment: true,
			default: None,
			check: None,
			comment: None,
		});

		let (sql, values) = builder.build_create_table(&stmt);
		assert!(sql.contains("`id` INT AUTO_INCREMENT PRIMARY KEY"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_table_with_foreign_key() {
		use crate::types::{
			ColumnDef, ColumnType, ForeignKeyAction, IntoTableRef, TableConstraint,
		};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_table();
		stmt.table("posts");
		stmt.columns.push(ColumnDef {
			name: "id".into_iden(),
			column_type: Some(ColumnType::Integer),
			not_null: false,
			unique: false,
			primary_key: true,
			auto_increment: false,
			default: None,
			check: None,
			comment: None,
		});
		stmt.columns.push(ColumnDef {
			name: "user_id".into_iden(),
			column_type: Some(ColumnType::Integer),
			not_null: false,
			unique: false,
			primary_key: false,
			auto_increment: false,
			default: None,
			check: None,
			comment: None,
		});
		stmt.constraints.push(TableConstraint::ForeignKey {
			name: Some("fk_user".into_iden()),
			columns: vec!["user_id".into_iden()],
			ref_table: "users".into_table_ref(),
			ref_columns: vec!["id".into_iden()],
			on_delete: Some(ForeignKeyAction::Cascade),
			on_update: Some(ForeignKeyAction::Restrict),
		});

		let (sql, values) = builder.build_create_table(&stmt);
		assert!(sql.contains("CONSTRAINT `fk_user` FOREIGN KEY (`user_id`)"));
		assert!(sql.contains("REFERENCES `users` (`id`)"));
		assert!(sql.contains("ON DELETE CASCADE"));
		assert!(sql.contains("ON UPDATE RESTRICT"));
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_index_basic() {
		use crate::query::IndexColumn;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_index();
		stmt.name("idx_users_email");
		stmt.table("users");
		stmt.columns.push(IndexColumn {
			name: "email".into_iden(),
			order: None,
		});

		let (sql, values) = builder.build_create_index(&stmt);
		assert_eq!(sql, "CREATE INDEX `idx_users_email` ON `users` (`email`)");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_index_unique() {
		use crate::query::IndexColumn;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_index();
		stmt.name("idx_users_username");
		stmt.table("users");
		stmt.unique = true;
		stmt.columns.push(IndexColumn {
			name: "username".into_iden(),
			order: None,
		});

		let (sql, values) = builder.build_create_index(&stmt);
		assert_eq!(
			sql,
			"CREATE UNIQUE INDEX `idx_users_username` ON `users` (`username`)"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_index_if_not_exists() {
		use crate::query::IndexColumn;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_index();
		stmt.name("idx_users_email");
		stmt.table("users");
		stmt.if_not_exists = true;
		stmt.columns.push(IndexColumn {
			name: "email".into_iden(),
			order: None,
		});

		let (sql, values) = builder.build_create_index(&stmt);
		assert_eq!(
			sql,
			"CREATE INDEX IF NOT EXISTS `idx_users_email` ON `users` (`email`)"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_index_with_order() {
		use crate::query::IndexColumn;
		use crate::types::Order;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_index();
		stmt.name("idx_users_created");
		stmt.table("users");
		stmt.columns.push(IndexColumn {
			name: "created_at".into_iden(),
			order: Some(Order::Desc),
		});

		let (sql, values) = builder.build_create_index(&stmt);
		assert_eq!(
			sql,
			"CREATE INDEX `idx_users_created` ON `users` (`created_at` DESC)"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_index_multiple_columns() {
		use crate::query::IndexColumn;
		use crate::types::Order;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_index();
		stmt.name("idx_users_name");
		stmt.table("users");
		stmt.columns.push(IndexColumn {
			name: "last_name".into_iden(),
			order: Some(Order::Asc),
		});
		stmt.columns.push(IndexColumn {
			name: "first_name".into_iden(),
			order: Some(Order::Asc),
		});

		let (sql, values) = builder.build_create_index(&stmt);
		assert_eq!(
			sql,
			"CREATE INDEX `idx_users_name` ON `users` (`last_name` ASC, `first_name` ASC)"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_index_with_using_btree() {
		use crate::query::{IndexColumn, IndexMethod};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_index();
		stmt.name("idx_users_id");
		stmt.table("users");
		stmt.using = Some(IndexMethod::BTree);
		stmt.columns.push(IndexColumn {
			name: "id".into_iden(),
			order: None,
		});

		let (sql, values) = builder.build_create_index(&stmt);
		assert_eq!(
			sql,
			"CREATE INDEX `idx_users_id` ON `users` (`id`) USING BTREE"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_create_index_with_using_fulltext() {
		use crate::query::{IndexColumn, IndexMethod};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_index();
		stmt.name("idx_posts_content");
		stmt.table("posts");
		stmt.using = Some(IndexMethod::FullText);
		stmt.columns.push(IndexColumn {
			name: "content".into_iden(),
			order: None,
		});

		let (sql, values) = builder.build_create_index(&stmt);
		assert_eq!(
			sql,
			"CREATE INDEX `idx_posts_content` ON `posts` (`content`) USING FULLTEXT"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_table_add_column() {
		use crate::query::AlterTableOperation;
		use crate::types::{ColumnDef, ColumnType};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::alter_table();
		stmt.table("users");
		stmt.operations
			.push(AlterTableOperation::AddColumn(ColumnDef {
				name: "age".into_iden(),
				column_type: Some(ColumnType::Integer),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				check: None,
				comment: None,
			}));

		let (sql, values) = builder.build_alter_table(&stmt);
		assert_eq!(sql, "ALTER TABLE `users` ADD COLUMN `age` INT");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_table_drop_column() {
		use crate::query::AlterTableOperation;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::alter_table();
		stmt.table("users");
		stmt.operations.push(AlterTableOperation::DropColumn {
			name: "age".into_iden(),
			if_exists: false,
		});

		let (sql, values) = builder.build_alter_table(&stmt);
		assert_eq!(sql, "ALTER TABLE `users` DROP COLUMN `age`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_table_rename_column() {
		use crate::query::AlterTableOperation;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::alter_table();
		stmt.table("users");
		stmt.operations.push(AlterTableOperation::RenameColumn {
			old: "email".into_iden(),
			new: "email_address".into_iden(),
		});

		let (sql, values) = builder.build_alter_table(&stmt);
		assert_eq!(
			sql,
			"ALTER TABLE `users` RENAME COLUMN `email` TO `email_address`"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_table_modify_column_type() {
		use crate::query::AlterTableOperation;
		use crate::types::{ColumnDef, ColumnType};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::alter_table();
		stmt.table("users");
		stmt.operations
			.push(AlterTableOperation::ModifyColumn(ColumnDef {
				name: "age".into_iden(),
				column_type: Some(ColumnType::BigInteger),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				check: None,
				comment: None,
			}));

		let (sql, values) = builder.build_alter_table(&stmt);
		assert_eq!(sql, "ALTER TABLE `users` MODIFY COLUMN `age` BIGINT");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_table_add_constraint() {
		use crate::query::AlterTableOperation;
		use crate::types::TableConstraint;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::alter_table();
		stmt.table("users");
		stmt.operations.push(AlterTableOperation::AddConstraint(
			TableConstraint::Unique {
				name: Some("unique_email".into_iden()),
				columns: vec!["email".into_iden()],
			},
		));

		let (sql, values) = builder.build_alter_table(&stmt);
		assert_eq!(
			sql,
			"ALTER TABLE `users` ADD CONSTRAINT `unique_email` UNIQUE (`email`)"
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_table_drop_constraint() {
		use crate::query::AlterTableOperation;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::alter_table();
		stmt.table("users");
		stmt.operations.push(AlterTableOperation::DropConstraint {
			name: "unique_email".into_iden(),
			if_exists: false,
		});

		let (sql, values) = builder.build_alter_table(&stmt);
		assert_eq!(sql, "ALTER TABLE `users` DROP CONSTRAINT `unique_email`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_alter_table_rename_table() {
		use crate::query::AlterTableOperation;

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::alter_table();
		stmt.table("users");
		stmt.operations
			.push(AlterTableOperation::RenameTable("accounts".into_iden()));

		let (sql, values) = builder.build_alter_table(&stmt);
		assert_eq!(sql, "ALTER TABLE `users` RENAME TO `accounts`");
		assert_eq!(values.len(), 0);
	}

	// VIEW tests

	#[test]
	fn test_create_view_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_view();
		stmt.name("user_view".into_iden());
		let mut select = Query::select();
		select.column("id").column("name").from("users");
		stmt.as_select(select);

		let (sql, _values) = builder.build_create_view(&stmt);
		assert!(sql.contains("CREATE VIEW"));
		assert!(sql.contains("`user_view`"));
		assert!(sql.contains("AS"));
		assert!(sql.contains("SELECT `id`, `name` FROM `users`"));
	}

	#[test]
	fn test_create_view_or_replace() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_view();
		stmt.name("user_view".into_iden()).or_replace();
		let mut select = Query::select();
		select.column("id").from("users");
		stmt.as_select(select);

		let (sql, _values) = builder.build_create_view(&stmt);
		assert!(sql.contains("CREATE OR REPLACE VIEW"));
		assert!(sql.contains("`user_view`"));
	}

	#[test]
	fn test_create_view_if_not_exists() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_view();
		stmt.name("user_view".into_iden()).if_not_exists();
		let mut select = Query::select();
		select.column("id").from("users");
		stmt.as_select(select);

		let (sql, _values) = builder.build_create_view(&stmt);
		assert!(sql.contains("CREATE VIEW IF NOT EXISTS"));
		assert!(sql.contains("`user_view`"));
	}

	#[test]
	#[should_panic(expected = "MySQL does not support MATERIALIZED views")]
	fn test_create_view_materialized_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_view();
		stmt.name("user_view".into_iden()).materialized(true);
		let mut select = Query::select();
		select.column("id").from("users");
		stmt.as_select(select);

		let _ = builder.build_create_view(&stmt);
	}

	#[test]
	fn test_create_view_with_columns() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_view();
		stmt.name("user_view".into_iden())
			.columns(vec!["user_id".into_iden(), "user_name".into_iden()]);
		let mut select = Query::select();
		select.column("id").column("name").from("users");
		stmt.as_select(select);

		let (sql, _values) = builder.build_create_view(&stmt);
		assert!(sql.contains("`user_view` (`user_id`, `user_name`)"));
	}

	#[test]
	fn test_drop_view_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_view();
		stmt.names(vec!["user_view".into_iden()]);

		let (sql, values) = builder.build_drop_view(&stmt);
		assert_eq!(sql, "DROP VIEW `user_view`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	fn test_drop_view_if_exists() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_view();
		stmt.names(vec!["user_view".into_iden()]).if_exists();

		let (sql, values) = builder.build_drop_view(&stmt);
		assert_eq!(sql, "DROP VIEW IF EXISTS `user_view`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support CASCADE/RESTRICT for DROP VIEW")]
	fn test_drop_view_cascade_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_view();
		stmt.names(vec!["user_view".into_iden()]).cascade();

		let _ = builder.build_drop_view(&stmt);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support MATERIALIZED views")]
	fn test_drop_view_materialized_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_view();
		stmt.names(vec!["user_view".into_iden()]).materialized(true);

		let _ = builder.build_drop_view(&stmt);
	}

	#[test]
	fn test_drop_view_multiple() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_view();
		stmt.names(vec![
			"view1".into_iden(),
			"view2".into_iden(),
			"view3".into_iden(),
		]);

		let (sql, values) = builder.build_drop_view(&stmt);
		assert_eq!(sql, "DROP VIEW `view1`, `view2`, `view3`");
		assert_eq!(values.len(), 0);
	}

	// TRUNCATE TABLE tests

	#[test]
	fn test_truncate_table_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::truncate_table();
		stmt.table("users");

		let (sql, values) = builder.build_truncate_table(&stmt);
		assert_eq!(sql, "TRUNCATE TABLE `users`");
		assert_eq!(values.len(), 0);
	}

	#[test]
	#[should_panic(
		expected = "MySQL does not support truncating multiple tables in a single TRUNCATE statement"
	)]
	fn test_truncate_table_multiple_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::truncate_table();
		stmt.table("users").table("posts");

		let _ = builder.build_truncate_table(&stmt);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support RESTART IDENTITY for TRUNCATE TABLE")]
	fn test_truncate_table_restart_identity_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::truncate_table();
		stmt.table("users").restart_identity();

		let _ = builder.build_truncate_table(&stmt);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support CASCADE for TRUNCATE TABLE")]
	fn test_truncate_table_cascade_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::truncate_table();
		stmt.table("users").cascade();

		let _ = builder.build_truncate_table(&stmt);
	}

	#[test]
	fn test_create_trigger_basic() {
		use crate::types::{TriggerBody, TriggerEvent, TriggerScope, TriggerTiming};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_trigger();
		stmt.name("update_timestamp")
			.timing(TriggerTiming::Before)
			.event(TriggerEvent::Update { columns: None })
			.on_table("users")
			.for_each(TriggerScope::Row)
			.body(TriggerBody::single("SET NEW.updated_at = NOW()"));

		let (sql, values) = builder.build_create_trigger(&stmt);
		assert_eq!(
			sql,
			r#"CREATE TRIGGER `update_timestamp` BEFORE UPDATE ON `users` FOR EACH ROW BEGIN SET NEW.updated_at = NOW(); END"#
		);
		assert_eq!(values.len(), 0);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support multiple events in a single trigger")]
	fn test_create_trigger_multiple_events_panics() {
		use crate::types::{TriggerEvent, TriggerScope, TriggerTiming};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_trigger();
		stmt.name("audit")
			.timing(TriggerTiming::After)
			.event(TriggerEvent::Insert)
			.event(TriggerEvent::Update { columns: None })
			.on_table("users")
			.for_each(TriggerScope::Row)
			.execute_function("audit_log");

		let _ = builder.build_create_trigger(&stmt);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support INSTEAD OF triggers")]
	fn test_create_trigger_instead_of_panics() {
		use crate::types::{TriggerBody, TriggerEvent, TriggerScope, TriggerTiming};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_trigger();
		stmt.name("view_insert")
			.timing(TriggerTiming::InsteadOf)
			.event(TriggerEvent::Insert)
			.on_table("view_name")
			.for_each(TriggerScope::Row)
			.body(TriggerBody::single(
				"INSERT INTO base_table VALUES (NEW.id)",
			));

		let _ = builder.build_create_trigger(&stmt);
	}

	#[test]
	#[should_panic(expected = "MySQL only supports FOR EACH ROW triggers")]
	fn test_create_trigger_for_statement_panics() {
		use crate::types::{TriggerBody, TriggerEvent, TriggerScope, TriggerTiming};

		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::create_trigger();
		stmt.name("audit")
			.timing(TriggerTiming::After)
			.event(TriggerEvent::Insert)
			.on_table("users")
			.for_each(TriggerScope::Statement)
			.body(TriggerBody::single("INSERT INTO audit_log VALUES (NOW())"));

		let _ = builder.build_create_trigger(&stmt);
	}

	#[test]
	fn test_drop_trigger_basic() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_trigger();
		stmt.name("update_timestamp").on_table("users");

		let (sql, values) = builder.build_drop_trigger(&stmt);
		assert_eq!(sql, r#"DROP TRIGGER `users`.`update_timestamp`"#);
		assert_eq!(values.len(), 0);
	}

	#[test]
	#[should_panic(expected = "MySQL requires table name (ON table) for DROP TRIGGER")]
	fn test_drop_trigger_no_table_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_trigger();
		stmt.name("update_timestamp");

		let _ = builder.build_drop_trigger(&stmt);
	}

	#[test]
	#[should_panic(expected = "MySQL does not support CASCADE/RESTRICT for DROP TRIGGER")]
	fn test_drop_trigger_cascade_panics() {
		let builder = MySqlQueryBuilder::new();
		let mut stmt = Query::drop_trigger();
		stmt.name("update_timestamp").on_table("users").cascade();

		let _ = builder.build_drop_trigger(&stmt);
	}
}
