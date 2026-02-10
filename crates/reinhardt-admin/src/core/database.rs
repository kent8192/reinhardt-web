//! Database integration for admin operations
//!
//! This module provides database access layer for admin CRUD operations,
//! integrating with reinhardt-orm's QuerySet API.

use crate::types::{AdminError, AdminResult};
use async_trait::async_trait;
use reinhardt_db::orm::{
	DatabaseConnection, Filter, FilterCondition, FilterOperator, FilterValue, Model,
};
use reinhardt_di::{DiResult, Injectable, InjectionContext};
use reinhardt_query::prelude::{
	Alias, ColumnRef, Condition, Expr, ExprTrait, IntoValue, Order, PostgresQueryBuilder, Query,
	QueryStatementBuilder, SimpleExpr, Value,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Dummy record type for admin panel CRUD operations
///
/// This type exists solely to satisfy the `<M: Model>` generic constraint
/// in `AdminDatabase` methods. The admin panel operates on dynamic data
/// (serde_json::Value), not statically-typed models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminRecord {
	pub id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AdminRecordFields {
	pub id: reinhardt_db::orm::query_fields::Field<AdminRecord, Option<i64>>,
}

impl Default for AdminRecordFields {
	fn default() -> Self {
		Self::new()
	}
}

impl AdminRecordFields {
	pub fn new() -> Self {
		Self {
			id: reinhardt_db::orm::query_fields::Field::new(vec!["id".to_string()]),
		}
	}
}

impl reinhardt_db::orm::FieldSelector for AdminRecordFields {
	fn with_alias(mut self, alias: &str) -> Self {
		self.id = self.id.with_alias(alias);
		self
	}
}

impl Model for AdminRecord {
	type PrimaryKey = i64;
	type Fields = AdminRecordFields;

	fn table_name() -> &'static str {
		"admin_records"
	}

	fn new_fields() -> Self::Fields {
		AdminRecordFields::new()
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, pk: Self::PrimaryKey) {
		self.id = Some(pk);
	}
}

/// Convert FilterValue to Value
#[doc(hidden)]
pub fn filter_value_to_sea_value(v: &FilterValue) -> Value {
	match v {
		FilterValue::String(s) => s.clone().into(),
		FilterValue::Integer(i) | FilterValue::Int(i) => (*i).into(),
		FilterValue::Float(f) => (*f).into(),
		FilterValue::Boolean(b) | FilterValue::Bool(b) => (*b).into(),
		FilterValue::Null => Value::Int(None),
		FilterValue::Array(_) => Value::String(None),
		FilterValue::FieldRef(f) => {
			// FieldRef generates column reference, not scalar value.
			// For Value context, return field name as string.
			// Proper handling is in build_single_filter_expr().
			Value::String(Some(Box::new(f.field.clone())))
		}
		FilterValue::Expression(expr) => {
			// Expression generates SQL expression, not scalar value.
			// For Value context, return SQL string representation.
			// Proper handling is in build_single_filter_expr().
			Value::String(Some(Box::new(expr.to_sql())))
		}
		FilterValue::OuterRef(outer) => {
			// OuterRef generates outer query reference, not scalar value.
			// For Value context, return field name as string.
			// Proper handling is in build_single_filter_expr().
			Value::String(Some(Box::new(outer.field.clone())))
		}
	}
}

/// Build a SimpleExpr from a single Filter
#[doc(hidden)]
pub fn build_single_filter_expr(filter: &Filter) -> Option<SimpleExpr> {
	let col = Expr::col(Alias::new(&filter.field));

	let expr = match (&filter.operator, &filter.value) {
		// Null handling (must come before generic patterns)
		(FilterOperator::Eq, FilterValue::Null) => col.is_null(),
		(FilterOperator::Ne, FilterValue::Null) => col.is_not_null(),

		// FieldRef: Column-to-column comparisons
		(FilterOperator::Eq, FilterValue::FieldRef(f)) => col.eq(Expr::col(Alias::new(&f.field))),
		(FilterOperator::Ne, FilterValue::FieldRef(f)) => col.ne(Expr::col(Alias::new(&f.field))),
		(FilterOperator::Gt, FilterValue::FieldRef(f)) => col.gt(Expr::col(Alias::new(&f.field))),
		(FilterOperator::Gte, FilterValue::FieldRef(f)) => col.gte(Expr::col(Alias::new(&f.field))),
		(FilterOperator::Lt, FilterValue::FieldRef(f)) => col.lt(Expr::col(Alias::new(&f.field))),
		(FilterOperator::Lte, FilterValue::FieldRef(f)) => col.lte(Expr::col(Alias::new(&f.field))),

		// OuterRef: Correlated subquery references (use custom SQL)
		(FilterOperator::Eq, FilterValue::OuterRef(outer)) => {
			Expr::cust(format!("\"{}\" = {}", filter.field, outer.to_sql())).into()
		}
		(FilterOperator::Ne, FilterValue::OuterRef(outer)) => {
			Expr::cust(format!("\"{}\" <> {}", filter.field, outer.to_sql())).into()
		}
		(FilterOperator::Gt, FilterValue::OuterRef(outer)) => {
			Expr::cust(format!("\"{}\" > {}", filter.field, outer.to_sql())).into()
		}
		(FilterOperator::Gte, FilterValue::OuterRef(outer)) => {
			Expr::cust(format!("\"{}\" >= {}", filter.field, outer.to_sql())).into()
		}
		(FilterOperator::Lt, FilterValue::OuterRef(outer)) => {
			Expr::cust(format!("\"{}\" < {}", filter.field, outer.to_sql())).into()
		}
		(FilterOperator::Lte, FilterValue::OuterRef(outer)) => {
			Expr::cust(format!("\"{}\" <= {}", filter.field, outer.to_sql())).into()
		}

		// Expression: Arithmetic expressions (use custom SQL for simplicity)
		(FilterOperator::Eq, FilterValue::Expression(expr)) => col.eq(Expr::cust(expr.to_sql())),
		(FilterOperator::Ne, FilterValue::Expression(expr)) => col.ne(Expr::cust(expr.to_sql())),
		(FilterOperator::Gt, FilterValue::Expression(expr)) => col.gt(Expr::cust(expr.to_sql())),
		(FilterOperator::Gte, FilterValue::Expression(expr)) => col.gte(Expr::cust(expr.to_sql())),
		(FilterOperator::Lt, FilterValue::Expression(expr)) => col.lt(Expr::cust(expr.to_sql())),
		(FilterOperator::Lte, FilterValue::Expression(expr)) => col.lte(Expr::cust(expr.to_sql())),

		// Generic scalar value patterns
		(FilterOperator::Eq, v) => col.eq(filter_value_to_sea_value(v)),
		(FilterOperator::Ne, v) => col.ne(filter_value_to_sea_value(v)),
		(FilterOperator::Gt, v) => col.gt(filter_value_to_sea_value(v)),
		(FilterOperator::Gte, v) => col.gte(filter_value_to_sea_value(v)),
		(FilterOperator::Lt, v) => col.lt(filter_value_to_sea_value(v)),
		(FilterOperator::Lte, v) => col.lte(filter_value_to_sea_value(v)),

		// String-specific operators
		(FilterOperator::Contains, FilterValue::String(s)) => col.like(format!("%{}%", s)),
		(FilterOperator::StartsWith, FilterValue::String(s)) => col.like(format!("{}%", s)),
		(FilterOperator::EndsWith, FilterValue::String(s)) => col.like(format!("%{}", s)),
		(FilterOperator::In, FilterValue::String(s)) => {
			let values: Vec<Value> = s.split(',').map(|v| v.trim().into_value()).collect();
			col.is_in(values)
		}
		(FilterOperator::NotIn, FilterValue::String(s)) => {
			let values: Vec<Value> = s.split(',').map(|v| v.trim().into_value()).collect();
			col.is_not_in(values)
		}

		// Skip unsupported combinations
		_ => return None,
	};

	Some(expr)
}

/// Build Condition from filters (AND logic only)
#[doc(hidden)]
pub fn build_filter_condition(filters: &[Filter]) -> Option<Condition> {
	if filters.is_empty() {
		return None;
	}

	let mut condition = Condition::all();

	for filter in filters {
		if let Some(expr) = build_single_filter_expr(filter) {
			condition = condition.add(expr);
		}
	}

	Some(condition)
}

/// Maximum recursion depth for filter conditions to prevent stack overflow
#[doc(hidden)]
pub const MAX_FILTER_DEPTH: usize = 100;

/// Build Condition from FilterCondition (supports AND/OR logic)
///
/// This function recursively processes FilterCondition to build complex
/// query conditions with nested AND/OR logic.
///
/// # Stack Overflow Protection
///
/// To prevent stack overflow with deeply nested filter conditions, this function
/// limits recursion depth to `MAX_FILTER_DEPTH` (100 levels). If the depth limit
/// is exceeded, the function returns `None`.
#[doc(hidden)]
pub fn build_composite_filter_condition(filter_condition: &FilterCondition) -> Option<Condition> {
	build_composite_filter_condition_with_depth(filter_condition, 0)
}

/// Internal helper for building composite filter conditions with depth tracking
#[doc(hidden)]
pub fn build_composite_filter_condition_with_depth(
	filter_condition: &FilterCondition,
	depth: usize,
) -> Option<Condition> {
	// Prevent stack overflow by limiting recursion depth
	if depth >= MAX_FILTER_DEPTH {
		return None;
	}

	match filter_condition {
		FilterCondition::Single(filter) => {
			build_single_filter_expr(filter).map(|expr| Condition::all().add(expr))
		}
		FilterCondition::And(conditions) => {
			if conditions.is_empty() {
				return None;
			}
			let mut and_condition = Condition::all();
			for cond in conditions {
				if let Some(sub_cond) = build_composite_filter_condition_with_depth(cond, depth + 1)
				{
					and_condition = and_condition.add(sub_cond);
				}
			}
			Some(and_condition)
		}
		FilterCondition::Or(conditions) => {
			if conditions.is_empty() {
				return None;
			}
			let mut or_condition = Condition::any();
			for cond in conditions {
				if let Some(sub_cond) = build_composite_filter_condition_with_depth(cond, depth + 1)
				{
					or_condition = or_condition.add(sub_cond);
				}
			}
			Some(or_condition)
		}
		FilterCondition::Not(inner) => {
			build_composite_filter_condition_with_depth(inner, depth + 1)
				.map(|inner_cond| inner_cond.not())
		}
	}
}

/// Admin database interface
///
/// Provides CRUD operations for admin panel, leveraging reinhardt-orm.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
/// use reinhardt_db::orm::DatabaseConnection;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
/// let db = AdminDatabase::new(conn);
///
/// // List items with filters
/// let items = db.list::<AdminRecord>("admin_records", vec![], 0, 50).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AdminDatabase {
	connection: Arc<DatabaseConnection>,
}

impl AdminDatabase {
	/// Create a new admin database interface
	///
	/// This method accepts a DatabaseConnection directly without requiring `Arc` wrapping.
	/// The `Arc` wrapping is handled internally for you.
	pub fn new(connection: DatabaseConnection) -> Self {
		Self {
			connection: Arc::new(connection),
		}
	}

	/// Create a new admin database interface from an Arc-wrapped connection
	///
	/// This is provided for cases where you already have an `Arc<DatabaseConnection>`.
	/// In most cases, you should use `new()` instead.
	pub fn from_arc(connection: Arc<DatabaseConnection>) -> Self {
		Self { connection }
	}

	/// Get a reference to the underlying database connection
	pub fn connection(&self) -> &DatabaseConnection {
		&self.connection
	}

	/// Get a cloned Arc of the connection (for cases where you need ownership)
	///
	/// In most cases, you should use `connection()` instead to get a reference.
	pub fn connection_arc(&self) -> Arc<DatabaseConnection> {
		Arc::clone(&self.connection)
	}

	/// List items with filters, ordering, and pagination
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
	/// use reinhardt_db::orm::{DatabaseConnection, Filter, FilterOperator, FilterValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let filters = vec![
	///     Filter::new("is_active".to_string(), FilterOperator::Eq, FilterValue::Boolean(true))
	/// ];
	///
	/// let items = db.list::<AdminRecord>("admin_records", filters, 0, 50).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn list<M: Model>(
		&self,
		table_name: &str,
		filters: Vec<Filter>,
		offset: u64,
		limit: u64,
	) -> AdminResult<Vec<HashMap<String, serde_json::Value>>> {
		let mut query = Query::select()
			.from(Alias::new(table_name))
			.column(ColumnRef::Asterisk)
			.to_owned();

		// Apply filters using build_filter_condition helper
		if let Some(condition) = build_filter_condition(&filters) {
			query.cond_where(condition);
		}

		// Apply pagination
		query.limit(limit).offset(offset);

		// Execute query
		let sql = query.to_string(PostgresQueryBuilder);
		let rows = self
			.connection
			.query(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		// Convert QueryRow to HashMap
		Ok(rows
			.into_iter()
			.filter_map(|row| {
				// row.data is already a serde_json::Value, typically an Object
				if let serde_json::Value::Object(map) = row.data {
					Some(
						map.into_iter()
							.collect::<HashMap<String, serde_json::Value>>(),
					)
				} else {
					None
				}
			})
			.collect())
	}

	/// List items with composite filter conditions (supports AND/OR logic)
	///
	/// This method supports complex filter conditions using FilterCondition,
	/// which allows building nested AND/OR queries.
	///
	/// # Arguments
	///
	/// * `table_name` - The name of the table to query
	/// * `filter_condition` - Optional composite filter condition (AND/OR logic)
	/// * `additional_filters` - Additional simple filters to AND with the condition
	/// * `sort_by` - Optional sort field (prefix with "-" for descending, e.g., "created_at" or "-created_at")
	/// * `offset` - Number of items to skip for pagination
	/// * `limit` - Maximum number of items to return
	pub async fn list_with_condition<M: Model>(
		&self,
		table_name: &str,
		filter_condition: Option<&FilterCondition>,
		additional_filters: Vec<Filter>,
		sort_by: Option<&str>,
		offset: u64,
		limit: u64,
	) -> AdminResult<Vec<HashMap<String, serde_json::Value>>> {
		let mut query = Query::select()
			.from(Alias::new(table_name))
			.column(ColumnRef::Asterisk)
			.to_owned();

		// Build combined condition
		let mut combined = Condition::all();

		// Add composite filter condition (e.g., OR search across fields)
		if let Some(fc) = filter_condition
			&& let Some(cond) = build_composite_filter_condition(fc)
		{
			combined = combined.add(cond);
		}

		// Add simple filters (AND logic)
		if let Some(simple_cond) = build_filter_condition(&additional_filters) {
			combined = combined.add(simple_cond);
		}

		// Only add condition if we have actual filters
		if !additional_filters.is_empty() || filter_condition.is_some() {
			query.cond_where(combined);
		}

		// Apply sorting (if specified)
		if let Some(sort_str) = sort_by {
			let (field, is_desc) = if let Some(stripped) = sort_str.strip_prefix('-') {
				(stripped, true)
			} else {
				(sort_str, false)
			};

			let col = Alias::new(field);
			if is_desc {
				query.order_by(col, Order::Desc);
			} else {
				query.order_by(col, Order::Asc);
			}
		}

		// Apply pagination
		query.limit(limit).offset(offset);

		// Execute query
		let sql = query.to_string(PostgresQueryBuilder);
		let rows = self
			.connection
			.query(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		// Convert QueryRow to HashMap
		Ok(rows
			.into_iter()
			.filter_map(|row| {
				if let serde_json::Value::Object(map) = row.data {
					Some(
						map.into_iter()
							.collect::<HashMap<String, serde_json::Value>>(),
					)
				} else {
					None
				}
			})
			.collect())
	}

	/// Count items with composite filter conditions (supports AND/OR logic)
	///
	/// # Arguments
	///
	/// * `table_name` - The name of the table to query
	/// * `filter_condition` - Optional composite filter condition (AND/OR logic)
	/// * `additional_filters` - Additional simple filters to AND with the condition
	pub async fn count_with_condition<M: Model>(
		&self,
		table_name: &str,
		filter_condition: Option<&FilterCondition>,
		additional_filters: Vec<Filter>,
	) -> AdminResult<u64> {
		let mut query = Query::select()
			.from(Alias::new(table_name))
			.expr(Expr::cust("COUNT(*) AS count"))
			.to_owned();

		// Build combined condition
		let mut combined = Condition::all();

		// Add composite filter condition
		if let Some(fc) = filter_condition
			&& let Some(cond) = build_composite_filter_condition(fc)
		{
			combined = combined.add(cond);
		}

		// Add simple filters
		if let Some(simple_cond) = build_filter_condition(&additional_filters) {
			combined = combined.add(simple_cond);
		}

		// Only add condition if we have actual filters
		if !additional_filters.is_empty() || filter_condition.is_some() {
			query.cond_where(combined);
		}

		let sql = query.to_string(PostgresQueryBuilder);
		let row = self
			.connection
			.query_one(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		// Extract count from result
		let count = if let Some(count_value) = row.data.get("count") {
			count_value.as_i64().unwrap_or(0) as u64
		} else if let Some(obj) = row.data.as_object() {
			obj.values().next().and_then(|v| v.as_i64()).unwrap_or(0) as u64
		} else {
			0
		};

		Ok(count)
	}

	/// Get a single item by ID
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
	/// use reinhardt_db::orm::DatabaseConnection;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let item = db.get::<AdminRecord>("admin_records", "id", "1").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get<M: Model>(
		&self,
		table_name: &str,
		pk_field: &str,
		id: &str,
	) -> AdminResult<Option<HashMap<String, serde_json::Value>>> {
		// Convert id to appropriate type for WHERE clause
		let pk_value: Value = if let Ok(num_id) = id.parse::<i64>() {
			Value::BigInt(Some(num_id))
		} else {
			Value::String(Some(Box::new(id.to_string())))
		};

		let query = Query::select()
			.from(Alias::new(table_name))
			.column(ColumnRef::Asterisk)
			.and_where(Expr::col(Alias::new(pk_field)).eq(pk_value))
			.to_owned();

		let sql = query.to_string(PostgresQueryBuilder);
		let row = self
			.connection
			.query_optional(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		Ok(row.and_then(|r| {
			// r.data is already a serde_json::Value, typically an Object
			if let serde_json::Value::Object(map) = r.data {
				Some(
					map.into_iter()
						.collect::<HashMap<String, serde_json::Value>>(),
				)
			} else {
				None
			}
		}))
	}

	/// Create a new item
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
	/// use reinhardt_db::orm::DatabaseConnection;
	/// use std::collections::HashMap;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let mut data = HashMap::new();
	/// data.insert("name".to_string(), serde_json::json!("Alice"));
	/// data.insert("email".to_string(), serde_json::json!("alice@example.com"));
	///
	/// db.create::<AdminRecord>("admin_records", data).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn create<M: Model>(
		&self,
		table_name: &str,
		data: HashMap<String, serde_json::Value>,
	) -> AdminResult<u64> {
		let mut query = Query::insert()
			.into_table(Alias::new(table_name))
			.to_owned();

		// Build column and value lists
		let mut columns = Vec::new();
		let mut values = Vec::new();

		for (key, value) in data {
			columns.push(Alias::new(&key));

			let sea_value = match value {
				serde_json::Value::String(s) => Value::String(Some(Box::new(s))),
				serde_json::Value::Number(n) => {
					if let Some(i) = n.as_i64() {
						Value::BigInt(Some(i))
					} else if let Some(f) = n.as_f64() {
						Value::Double(Some(f))
					} else {
						Value::String(Some(Box::new(n.to_string())))
					}
				}
				serde_json::Value::Bool(b) => Value::Bool(Some(b)),
				serde_json::Value::Null => Value::Int(None),
				_ => Value::String(Some(Box::new(value.to_string()))),
			};
			values.push(sea_value);
		}

		// Pass values directly for reinhardt-query
		query.columns(columns).values(values).unwrap();

		// Add RETURNING clause to get the inserted ID
		query.returning([Alias::new("id")]);

		let sql = query.to_string(PostgresQueryBuilder);
		let row = self
			.connection
			.query_one(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		// Extract the ID from the returned row
		let id = if let Some(serde_json::Value::Number(n)) = row.data.get("id") {
			n.as_u64().unwrap_or(0)
		} else {
			0
		};

		Ok(id)
	}

	/// Update an existing item
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
	/// use reinhardt_db::orm::DatabaseConnection;
	/// use std::collections::HashMap;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let mut data = HashMap::new();
	/// data.insert("name".to_string(), serde_json::json!("Alice Updated"));
	///
	/// db.update::<AdminRecord>("admin_records", "id", "1", data).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn update<M: Model>(
		&self,
		table_name: &str,
		pk_field: &str,
		id: &str,
		data: HashMap<String, serde_json::Value>,
	) -> AdminResult<u64> {
		let mut query = Query::update().table(Alias::new(table_name)).to_owned();

		// Build SET clauses
		for (key, value) in data {
			let sea_value = match value {
				serde_json::Value::String(s) => Value::String(Some(Box::new(s))),
				serde_json::Value::Number(n) => {
					if let Some(i) = n.as_i64() {
						Value::BigInt(Some(i))
					} else if let Some(f) = n.as_f64() {
						Value::Double(Some(f))
					} else {
						Value::String(Some(Box::new(n.to_string())))
					}
				}
				serde_json::Value::Bool(b) => Value::Bool(Some(b)),
				serde_json::Value::Null => Value::Int(None),
				_ => Value::String(Some(Box::new(value.to_string()))),
			};
			query.value(Alias::new(&key), sea_value);
		}

		// Convert id to appropriate type for WHERE clause
		let pk_value: Value = if let Ok(num_id) = id.parse::<i64>() {
			Value::BigInt(Some(num_id))
		} else {
			Value::String(Some(Box::new(id.to_string())))
		};
		query.and_where(Expr::col(Alias::new(pk_field)).eq(pk_value));

		let sql = query.to_string(PostgresQueryBuilder);
		let affected = self
			.connection
			.execute(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		Ok(affected)
	}

	/// Delete an item by ID
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
	/// use reinhardt_db::orm::DatabaseConnection;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// db.delete::<AdminRecord>("admin_records", "id", "1").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn delete<M: Model>(
		&self,
		table_name: &str,
		pk_field: &str,
		id: &str,
	) -> AdminResult<u64> {
		// Convert id to appropriate type for WHERE clause
		let pk_value: Value = if let Ok(num_id) = id.parse::<i64>() {
			Value::BigInt(Some(num_id))
		} else {
			Value::String(Some(Box::new(id.to_string())))
		};

		let query = Query::delete()
			.from_table(Alias::new(table_name))
			.and_where(Expr::col(Alias::new(pk_field)).eq(pk_value))
			.to_owned();

		let sql = query.to_string(PostgresQueryBuilder);
		let affected = self
			.connection
			.execute(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		Ok(affected)
	}

	/// Delete multiple items by IDs (bulk delete)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
	/// use reinhardt_db::orm::DatabaseConnection;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
	/// db.bulk_delete::<AdminRecord>("admin_records", "id", ids).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn bulk_delete<M: Model>(
		&self,
		table_name: &str,
		pk_field: &str,
		ids: Vec<String>,
	) -> AdminResult<u64> {
		self.bulk_delete_by_table(table_name, pk_field, ids).await
	}

	/// Delete multiple items by IDs without requiring Model type parameter
	///
	/// This method provides a type-safe way to perform bulk deletions without
	/// requiring a Model type parameter. It's particularly useful for admin actions
	/// where the model type may not be known at compile time.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::AdminDatabase;
	/// use reinhardt_db::orm::DatabaseConnection;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
	/// db.bulk_delete_by_table("users", "id", ids).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn bulk_delete_by_table(
		&self,
		table_name: &str,
		pk_field: &str,
		ids: Vec<String>,
	) -> AdminResult<u64> {
		if ids.is_empty() {
			return Ok(0);
		}

		// Convert each id to appropriate type for WHERE clause
		let pk_values: Vec<Value> = ids
			.iter()
			.map(|id| {
				if let Ok(num_id) = id.parse::<i64>() {
					Value::BigInt(Some(num_id))
				} else {
					Value::String(Some(Box::new(id.to_string())))
				}
			})
			.collect();

		let query = Query::delete()
			.from_table(Alias::new(table_name))
			.and_where(Expr::col(Alias::new(pk_field)).is_in(pk_values))
			.to_owned();

		let sql = query.to_string(PostgresQueryBuilder);
		let affected = self
			.connection
			.execute(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		Ok(affected)
	}

	/// Count total items with optional filters
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::core::{AdminDatabase, AdminRecord};
	/// use reinhardt_db::orm::{DatabaseConnection, Filter, FilterOperator, FilterValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let filters = vec![
	///     Filter::new("is_active".to_string(), FilterOperator::Eq, FilterValue::Boolean(true))
	/// ];
	///
	/// let count = db.count::<AdminRecord>("admin_records", filters).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn count<M: Model>(
		&self,
		table_name: &str,
		filters: Vec<Filter>,
	) -> AdminResult<u64> {
		let mut query = Query::select()
			.from(Alias::new(table_name))
			.expr(Expr::cust("COUNT(*) AS count"))
			.to_owned();

		// Apply filters using build_filter_condition helper
		if let Some(condition) = build_filter_condition(&filters) {
			query.cond_where(condition);
		}

		let sql = query.to_string(PostgresQueryBuilder);
		let row = self
			.connection
			.query_one(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		// Extract count from result
		let count = if let Some(count_value) = row.data.get("count") {
			count_value.as_i64().unwrap_or(0) as u64
		} else if let Some(obj) = row.data.as_object() {
			// COUNT(*) result may be in the first column
			obj.values().next().and_then(|v| v.as_i64()).unwrap_or(0) as u64
		} else {
			0
		};

		Ok(count)
	}
}

/// Injectable trait implementation for AdminDatabase
///
/// This allows AdminDatabase to be injected via the DI container.
/// The implementation resolves `Arc<AdminDatabase>` from the container
/// and clones the inner value.
#[async_trait]
impl Injectable for AdminDatabase {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Resolve Arc<AdminDatabase> from the container and clone it
		ctx.resolve::<Self>().await.map(|arc| (*arc).clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_db::orm::DatabaseBackend;
	use reinhardt_db::orm::annotation::Expression;
	use reinhardt_db::orm::expressions::{F, OuterRef};
	use reinhardt_test::fixtures::mock_connection;
	use rstest::*;

	// Mock User model for testing
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct User {
		id: i64,
		name: String,
	}

	#[derive(Debug, Clone)]
	struct UserFields {
		pub id: reinhardt_db::orm::query_fields::Field<User, i64>,
		pub name: reinhardt_db::orm::query_fields::Field<User, String>,
	}

	impl UserFields {
		pub(crate) fn new() -> Self {
			Self {
				id: reinhardt_db::orm::query_fields::Field::new(vec!["id".to_string()]),
				name: reinhardt_db::orm::query_fields::Field::new(vec!["name".to_string()]),
			}
		}
	}

	impl reinhardt_db::orm::FieldSelector for UserFields {
		fn with_alias(mut self, alias: &str) -> Self {
			self.id = self.id.with_alias(alias);
			self.name = self.name.with_alias(alias);
			self
		}
	}

	impl Model for User {
		type PrimaryKey = i64;
		type Fields = UserFields;

		fn table_name() -> &'static str {
			"users"
		}

		fn new_fields() -> Self::Fields {
			UserFields::new()
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_admin_database_new(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		assert_eq!(db.connection().backend(), DatabaseBackend::Postgres);
	}

	#[rstest]
	#[tokio::test]
	async fn test_bulk_delete_empty(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let result = db.bulk_delete::<User>("users", "id", vec![]).await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_list_with_filters(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let filters = vec![Filter::new(
			"is_active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		)];

		let result = db.list::<User>("users", filters, 0, 50).await;
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_get_by_id(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let result = db.get::<User>("users", "id", "1").await;
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_create(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let mut data = HashMap::new();
		data.insert("name".to_string(), serde_json::json!("Alice"));
		data.insert("email".to_string(), serde_json::json!("alice@example.com"));

		let result = db.create::<User>("users", data).await;
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_update(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let mut data = HashMap::new();
		data.insert("name".to_string(), serde_json::json!("Alice Updated"));

		let result = db.update::<User>("users", "id", "1", data).await;
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_delete(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let result = db.delete::<User>("users", "id", "1").await;
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_count(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let filters = vec![];
		let result = db.count::<User>("users", filters).await;
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_bulk_delete_multiple_ids(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
		let result = db.bulk_delete::<User>("users", "id", ids).await;
		assert!(result.is_ok());
	}

	// ==================== build_composite_filter_condition tests ====================

	#[test]
	fn test_build_composite_single_condition() {
		let filter = Filter::new(
			"name".to_string(),
			FilterOperator::Eq,
			FilterValue::String("Alice".to_string()),
		);
		let condition = FilterCondition::Single(filter);

		let result = build_composite_filter_condition(&condition);

		assert!(result.is_some());
		// The condition should produce valid SQL when used
		let cond = result.unwrap();
		let query = Query::select()
			.from(Alias::new("users"))
			.column(ColumnRef::Asterisk)
			.cond_where(cond)
			.to_string(PostgresQueryBuilder);
		assert!(query.contains("\"name\""));
		assert!(query.contains("'Alice'"));
	}

	#[test]
	fn test_build_composite_or_condition() {
		let filter1 = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("Alice".to_string()),
		);
		let filter2 = Filter::new(
			"email".to_string(),
			FilterOperator::Contains,
			FilterValue::String("alice".to_string()),
		);

		let condition = FilterCondition::Or(vec![
			FilterCondition::Single(filter1),
			FilterCondition::Single(filter2),
		]);

		let result = build_composite_filter_condition(&condition);

		assert!(result.is_some());
		let cond = result.unwrap();
		let query = Query::select()
			.from(Alias::new("users"))
			.column(ColumnRef::Asterisk)
			.cond_where(cond)
			.to_string(PostgresQueryBuilder);
		// OR condition should produce SQL with OR keyword
		assert!(query.contains("\"name\""));
		assert!(query.contains("\"email\""));
		assert!(query.contains("OR"));
	}

	#[test]
	fn test_build_composite_and_condition() {
		let filter1 = Filter::new(
			"is_active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		);
		let filter2 = Filter::new(
			"is_staff".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		);

		let condition = FilterCondition::And(vec![
			FilterCondition::Single(filter1),
			FilterCondition::Single(filter2),
		]);

		let result = build_composite_filter_condition(&condition);

		assert!(result.is_some());
		let cond = result.unwrap();
		let query = Query::select()
			.from(Alias::new("users"))
			.column(ColumnRef::Asterisk)
			.cond_where(cond)
			.to_string(PostgresQueryBuilder);
		// AND condition should produce SQL with AND keyword
		assert!(query.contains("\"is_active\""));
		assert!(query.contains("\"is_staff\""));
		assert!(query.contains("AND"));
	}

	#[test]
	fn test_build_composite_nested_condition() {
		// Build: (name LIKE '%Alice%' OR email LIKE '%alice%') AND is_active = true
		let filter_name = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("Alice".to_string()),
		);
		let filter_email = Filter::new(
			"email".to_string(),
			FilterOperator::Contains,
			FilterValue::String("alice".to_string()),
		);
		let filter_active = Filter::new(
			"is_active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		);

		let or_condition = FilterCondition::Or(vec![
			FilterCondition::Single(filter_name),
			FilterCondition::Single(filter_email),
		]);

		let and_condition =
			FilterCondition::And(vec![or_condition, FilterCondition::Single(filter_active)]);

		let result = build_composite_filter_condition(&and_condition);

		assert!(result.is_some());
		let cond = result.unwrap();
		let query = Query::select()
			.from(Alias::new("users"))
			.column(ColumnRef::Asterisk)
			.cond_where(cond)
			.to_string(PostgresQueryBuilder);
		// Nested condition should contain both OR and AND
		assert!(query.contains("\"name\""));
		assert!(query.contains("\"email\""));
		assert!(query.contains("\"is_active\""));
		assert!(query.contains("OR"));
		assert!(query.contains("AND"));
	}

	#[test]
	fn test_build_composite_empty_or() {
		let condition = FilterCondition::Or(vec![]);

		let result = build_composite_filter_condition(&condition);

		// Empty OR should return None
		assert!(result.is_none());
	}

	#[test]
	fn test_build_composite_empty_and() {
		let condition = FilterCondition::And(vec![]);

		let result = build_composite_filter_condition(&condition);

		// Empty AND should return None
		assert!(result.is_none());
	}

	// ==================== list_with_condition / count_with_condition tests ====================

	#[rstest]
	#[tokio::test]
	async fn test_list_with_condition_or_search(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		// Build OR search condition: name LIKE '%Alice%' OR email LIKE '%alice%'
		let filter1 = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("Alice".to_string()),
		);
		let filter2 = Filter::new(
			"email".to_string(),
			FilterOperator::Contains,
			FilterValue::String("alice".to_string()),
		);
		let search_condition = FilterCondition::Or(vec![
			FilterCondition::Single(filter1),
			FilterCondition::Single(filter2),
		]);

		let result = db
			.list_with_condition::<User>("users", Some(&search_condition), vec![], None, 0, 50)
			.await;

		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_list_with_condition_and_additional(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		// Build combined condition: (name LIKE '%Alice%' OR email LIKE '%alice%') AND is_active = true
		let filter1 = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("Alice".to_string()),
		);
		let filter2 = Filter::new(
			"email".to_string(),
			FilterOperator::Contains,
			FilterValue::String("alice".to_string()),
		);
		let search_condition = FilterCondition::Or(vec![
			FilterCondition::Single(filter1),
			FilterCondition::Single(filter2),
		]);

		let additional = vec![Filter::new(
			"is_active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		)];

		let result = db
			.list_with_condition::<User>("users", Some(&search_condition), additional, None, 0, 50)
			.await;

		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_count_with_condition_or_search(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		// Build OR search condition
		let filter1 = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("Alice".to_string()),
		);
		let filter2 = Filter::new(
			"email".to_string(),
			FilterOperator::Contains,
			FilterValue::String("alice".to_string()),
		);
		let search_condition = FilterCondition::Or(vec![
			FilterCondition::Single(filter1),
			FilterCondition::Single(filter2),
		]);

		let result = db
			.count_with_condition::<User>("users", Some(&search_condition), vec![])
			.await;

		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_list_with_condition_none(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		// No filter condition - should return all items
		let result = db
			.list_with_condition::<User>("users", None, vec![], None, 0, 50)
			.await;

		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_list_with_condition_empty_additional(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		let filter = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("Alice".to_string()),
		);
		let search_condition = FilterCondition::Single(filter);

		// Empty additional filters
		let result = db
			.list_with_condition::<User>("users", Some(&search_condition), vec![], None, 0, 50)
			.await;

		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_count_with_condition_none(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		// No filter condition
		let result = db.count_with_condition::<User>("users", None, vec![]).await;

		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_count_with_condition_combined(mock_connection: DatabaseConnection) {
		let db = AdminDatabase::new(mock_connection);

		// Combined filter condition and additional filters
		let filter1 = Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("Alice".to_string()),
		);
		let search_condition = FilterCondition::Single(filter1);

		let additional = vec![Filter::new(
			"is_active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		)];

		let result = db
			.count_with_condition::<User>("users", Some(&search_condition), additional)
			.await;

		assert!(result.is_ok());
	}

	// ==================== FieldRef/OuterRef/Expression filter tests ====================

	#[test]
	fn test_build_single_filter_expr_field_ref_eq() {
		let filter = Filter::new(
			"price".to_string(),
			FilterOperator::Eq,
			FilterValue::FieldRef(F::new("discount_price")),
		);
		let result = build_single_filter_expr(&filter);
		assert!(result.is_some());

		let query = Query::select()
			.from(Alias::new("products"))
			.column(ColumnRef::Asterisk)
			.cond_where(Condition::all().add(result.unwrap()))
			.to_string(PostgresQueryBuilder);
		assert!(query.contains("\"price\""));
		assert!(query.contains("\"discount_price\""));
	}

	#[test]
	fn test_build_single_filter_expr_field_ref_gt() {
		let filter = Filter::new(
			"price".to_string(),
			FilterOperator::Gt,
			FilterValue::FieldRef(F::new("cost")),
		);
		let result = build_single_filter_expr(&filter);
		assert!(result.is_some());
	}

	#[test]
	fn test_build_single_filter_expr_field_ref_all_operators() {
		let operators = [
			FilterOperator::Eq,
			FilterOperator::Ne,
			FilterOperator::Gt,
			FilterOperator::Gte,
			FilterOperator::Lt,
			FilterOperator::Lte,
		];

		for op in operators {
			let filter = Filter::new(
				"field_a".to_string(),
				op.clone(),
				FilterValue::FieldRef(F::new("field_b")),
			);
			let result = build_single_filter_expr(&filter);
			assert!(
				result.is_some(),
				"FieldRef with {:?} should produce Some",
				op
			);
		}
	}

	#[test]
	fn test_build_single_filter_expr_outer_ref() {
		let filter = Filter::new(
			"author_id".to_string(),
			FilterOperator::Eq,
			FilterValue::OuterRef(OuterRef::new("authors.id")),
		);
		let result = build_single_filter_expr(&filter);
		assert!(result.is_some());

		let query = Query::select()
			.from(Alias::new("books"))
			.column(ColumnRef::Asterisk)
			.cond_where(Condition::all().add(result.unwrap()))
			.to_string(PostgresQueryBuilder);
		assert!(query.contains("author_id"));
		assert!(query.contains("authors.id"));
	}

	#[test]
	fn test_build_single_filter_expr_outer_ref_all_operators() {
		let operators = [
			FilterOperator::Eq,
			FilterOperator::Ne,
			FilterOperator::Gt,
			FilterOperator::Gte,
			FilterOperator::Lt,
			FilterOperator::Lte,
		];

		for op in operators {
			let filter = Filter::new(
				"child_id".to_string(),
				op.clone(),
				FilterValue::OuterRef(OuterRef::new("parent.id")),
			);
			let result = build_single_filter_expr(&filter);
			assert!(
				result.is_some(),
				"OuterRef with {:?} should produce Some",
				op
			);
		}
	}

	#[test]
	fn test_build_single_filter_expr_expression() {
		use reinhardt_db::orm::annotation::{AnnotationValue, Value};

		// Test: price > (cost * 2)
		let expr = Expression::Multiply(
			Box::new(AnnotationValue::Field(F::new("cost"))),
			Box::new(AnnotationValue::Value(Value::Int(2))),
		);
		let filter = Filter::new(
			"price".to_string(),
			FilterOperator::Gt,
			FilterValue::Expression(expr),
		);
		let result = build_single_filter_expr(&filter);
		assert!(result.is_some());
	}

	#[test]
	fn test_build_single_filter_expr_expression_all_operators() {
		use reinhardt_db::orm::annotation::{AnnotationValue, Value as OrmValue};

		let operators = [
			FilterOperator::Eq,
			FilterOperator::Ne,
			FilterOperator::Gt,
			FilterOperator::Gte,
			FilterOperator::Lt,
			FilterOperator::Lte,
		];

		for op in operators {
			let expr = Expression::Add(
				Box::new(AnnotationValue::Field(F::new("base"))),
				Box::new(AnnotationValue::Value(OrmValue::Int(10))),
			);
			let filter = Filter::new(
				"total".to_string(),
				op.clone(),
				FilterValue::Expression(expr),
			);
			let result = build_single_filter_expr(&filter);
			assert!(
				result.is_some(),
				"Expression with {:?} should produce Some",
				op
			);
		}
	}

	#[test]
	fn test_filter_value_to_sea_value_field_ref_fallback() {
		let value = FilterValue::FieldRef(F::new("test_field"));
		let sea_value = filter_value_to_sea_value(&value);

		// Should return string representation, not panic
		match sea_value {
			Value::String(Some(s)) => assert_eq!(s.as_str(), "test_field"),
			_ => panic!("Expected String value"),
		}
	}

	#[test]
	fn test_filter_value_to_sea_value_outer_ref_fallback() {
		let value = FilterValue::OuterRef(OuterRef::new("outer.field"));
		let sea_value = filter_value_to_sea_value(&value);

		// Should return string representation, not panic
		match sea_value {
			Value::String(Some(s)) => assert_eq!(s.as_str(), "outer.field"),
			_ => panic!("Expected String value"),
		}
	}

	#[test]
	fn test_filter_value_to_sea_value_expression_fallback() {
		use reinhardt_db::orm::annotation::{AnnotationValue, Value as OrmValue};

		let expr = Expression::Add(
			Box::new(AnnotationValue::Field(F::new("a"))),
			Box::new(AnnotationValue::Value(OrmValue::Int(1))),
		);
		let value = FilterValue::Expression(expr);
		let sea_value = filter_value_to_sea_value(&value);

		// Should return SQL string representation, not panic
		match sea_value {
			Value::String(Some(s)) => {
				assert!(s.contains("a"), "SQL should contain field name 'a'");
				assert!(s.contains("1"), "SQL should contain value '1'");
			}
			_ => panic!("Expected String value"),
		}
	}
}
