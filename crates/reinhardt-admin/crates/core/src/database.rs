//! Database integration for admin operations
//!
//! This module provides database access layer for admin CRUD operations,
//! integrating with reinhardt-orm's QuerySet API.

use crate::{AdminError, AdminResult};
use async_trait::async_trait;
use reinhardt_db::orm::{
	DatabaseConnection, Filter, FilterCondition, FilterOperator, FilterValue, Model,
};
use reinhardt_di::{DiResult, Injectable, InjectionContext};
use sea_query::{
	Alias, Asterisk, Condition, Expr, ExprTrait, Order, PostgresQueryBuilder, Query as SeaQuery,
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

impl Model for AdminRecord {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"admin_records"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, pk: Self::PrimaryKey) {
		self.id = Some(pk);
	}
}

/// Convert FilterValue to sea_query::Value
fn filter_value_to_sea_value(v: &FilterValue) -> sea_query::Value {
	match v {
		FilterValue::String(s) => s.clone().into(),
		FilterValue::Integer(i) | FilterValue::Int(i) => (*i).into(),
		FilterValue::Float(f) => (*f).into(),
		FilterValue::Boolean(b) | FilterValue::Bool(b) => (*b).into(),
		FilterValue::Null => sea_query::Value::Int(None),
		FilterValue::Array(_) => sea_query::Value::String(None),
		FilterValue::FieldRef(_) => {
			todo!("FieldRef to sea_query::Value conversion not implemented")
		}
		FilterValue::Expression(_) => {
			todo!("Expression to sea_query::Value conversion not implemented")
		}
	}
}

/// Build a SimpleExpr from a single Filter
fn build_single_filter_expr(filter: &Filter) -> Option<sea_query::SimpleExpr> {
	let col = Expr::col(Alias::new(&filter.field));

	let expr = match (&filter.operator, &filter.value) {
		(FilterOperator::Eq, FilterValue::Null) => col.is_null(),
		(FilterOperator::Ne, FilterValue::Null) => col.is_not_null(),
		(FilterOperator::Eq, v) => col.eq(filter_value_to_sea_value(v)),
		(FilterOperator::Ne, v) => col.ne(filter_value_to_sea_value(v)),
		(FilterOperator::Gt, v) => col.gt(filter_value_to_sea_value(v)),
		(FilterOperator::Gte, v) => col.gte(filter_value_to_sea_value(v)),
		(FilterOperator::Lt, v) => col.lt(filter_value_to_sea_value(v)),
		(FilterOperator::Lte, v) => col.lte(filter_value_to_sea_value(v)),
		(FilterOperator::Contains, FilterValue::String(s)) => col.like(format!("%{}%", s)),
		(FilterOperator::StartsWith, FilterValue::String(s)) => col.like(format!("{}%", s)),
		(FilterOperator::EndsWith, FilterValue::String(s)) => col.like(format!("%{}", s)),
		(FilterOperator::In, FilterValue::String(s)) => {
			let values: Vec<sea_query::Value> =
				s.split(',').map(|v| v.trim().to_string().into()).collect();
			col.is_in(values)
		}
		(FilterOperator::NotIn, FilterValue::String(s)) => {
			let values: Vec<sea_query::Value> =
				s.split(',').map(|v| v.trim().to_string().into()).collect();
			col.is_not_in(values)
		}
		_ => return None, // Skip unsupported combinations
	};

	Some(expr)
}

/// Build sea-query Condition from filters (AND logic only)
fn build_filter_condition(filters: &[Filter]) -> Option<Condition> {
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

/// Build sea-query Condition from FilterCondition (supports AND/OR logic)
///
/// This function recursively processes FilterCondition to build complex
/// query conditions with nested AND/OR logic.
fn build_composite_filter_condition(filter_condition: &FilterCondition) -> Option<Condition> {
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
				if let Some(sub_cond) = build_composite_filter_condition(cond) {
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
				if let Some(sub_cond) = build_composite_filter_condition(cond) {
					or_condition = or_condition.add(sub_cond);
				}
			}
			Some(or_condition)
		}
		FilterCondition::Not(inner) => {
			build_composite_filter_condition(inner).map(|inner_cond| inner_cond.not())
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
/// use reinhardt_admin_core::AdminDatabase;
/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model};
/// use std::sync::Arc;
/// use serde::{Serialize, Deserialize};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
/// let db = AdminDatabase::new(conn);
///
/// // List items with filters
/// let items = db.list::<User>("users", vec![], 0, 50).await?;
/// # Ok(())
/// # }
///
/// // Placeholder User type for example
/// #[derive(Clone, Serialize, Deserialize)]
/// struct User {
///     id: Option<i64>,
///     name: String,
/// }
///
/// impl Model for User {
///     type PrimaryKey = i64;
///     fn table_name() -> &'static str { "users" }
///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
/// }
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
	/// use reinhardt_admin_core::AdminDatabase;
	/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model, Filter, FilterOperator, FilterValue};
	/// use std::sync::Arc;
	/// use serde::{Serialize, Deserialize};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let filters = vec![
	///     Filter::new("is_active".to_string(), FilterOperator::Eq, FilterValue::Boolean(true))
	/// ];
	///
	/// let items = db.list::<User>("users", filters, 0, 50).await?;
	/// # Ok(())
	/// # }
	///
	/// #[derive(Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
	/// }
	/// ```
	pub async fn list<M: Model>(
		&self,
		table_name: &str,
		filters: Vec<Filter>,
		offset: u64,
		limit: u64,
	) -> AdminResult<Vec<HashMap<String, serde_json::Value>>> {
		let mut query = SeaQuery::select()
			.from(Alias::new(table_name))
			.column(Asterisk)
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
		let mut query = SeaQuery::select()
			.from(Alias::new(table_name))
			.column(Asterisk)
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
		let mut query = SeaQuery::select()
			.from(Alias::new(table_name))
			.expr(Expr::cust("COUNT(*)"))
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
	/// use reinhardt_admin_core::AdminDatabase;
	/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model};
	/// use std::sync::Arc;
	/// use serde::{Serialize, Deserialize};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let item = db.get::<User>("users", "id", "1").await?;
	/// # Ok(())
	/// # }
	///
	/// #[derive(Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
	/// }
	/// ```
	pub async fn get<M: Model>(
		&self,
		table_name: &str,
		pk_field: &str,
		id: &str,
	) -> AdminResult<Option<HashMap<String, serde_json::Value>>> {
		let query = SeaQuery::select()
			.from(Alias::new(table_name))
			.column(Asterisk)
			.and_where(Expr::col(Alias::new(pk_field)).eq(id))
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
	/// use reinhardt_admin_core::AdminDatabase;
	/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model};
	/// use std::sync::Arc;
	/// use std::collections::HashMap;
	/// use serde::{Serialize, Deserialize};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let mut data = HashMap::new();
	/// data.insert("name".to_string(), serde_json::json!("Alice"));
	/// data.insert("email".to_string(), serde_json::json!("alice@example.com"));
	///
	/// db.create::<User>("users", data).await?;
	/// # Ok(())
	/// # }
	///
	/// #[derive(Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
	/// }
	/// ```
	pub async fn create<M: Model>(
		&self,
		table_name: &str,
		data: HashMap<String, serde_json::Value>,
	) -> AdminResult<u64> {
		let mut query = SeaQuery::insert()
			.into_table(Alias::new(table_name))
			.to_owned();

		// Build column and value lists
		let mut columns = Vec::new();
		let mut values = Vec::new();

		for (key, value) in data {
			columns.push(Alias::new(&key));

			let sea_value = match value {
				serde_json::Value::String(s) => sea_query::Value::String(Some(s)),
				serde_json::Value::Number(n) => {
					if let Some(i) = n.as_i64() {
						sea_query::Value::BigInt(Some(i))
					} else if let Some(f) = n.as_f64() {
						sea_query::Value::Double(Some(f))
					} else {
						sea_query::Value::String(Some(n.to_string()))
					}
				}
				serde_json::Value::Bool(b) => sea_query::Value::Bool(Some(b)),
				serde_json::Value::Null => sea_query::Value::Int(None),
				_ => sea_query::Value::String(Some(value.to_string())),
			};
			values.push(sea_value);
		}

		// Convert Values to Exprs for sea-query v1.0
		let expr_values: Vec<sea_query::SimpleExpr> =
			values.into_iter().map(|v| v.into()).collect();
		query.columns(columns).values(expr_values).unwrap();

		let sql = query.to_string(PostgresQueryBuilder);
		let affected = self
			.connection
			.execute(&sql, vec![])
			.await
			.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

		Ok(affected)
	}

	/// Update an existing item
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_core::AdminDatabase;
	/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model};
	/// use std::sync::Arc;
	/// use std::collections::HashMap;
	/// use serde::{Serialize, Deserialize};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let mut data = HashMap::new();
	/// data.insert("name".to_string(), serde_json::json!("Alice Updated"));
	///
	/// db.update::<User>("users", "id", "1", data).await?;
	/// # Ok(())
	/// # }
	///
	/// #[derive(Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
	/// }
	/// ```
	pub async fn update<M: Model>(
		&self,
		table_name: &str,
		pk_field: &str,
		id: &str,
		data: HashMap<String, serde_json::Value>,
	) -> AdminResult<u64> {
		let mut query = SeaQuery::update().table(Alias::new(table_name)).to_owned();

		// Build SET clauses
		for (key, value) in data {
			let sea_value = match value {
				serde_json::Value::String(s) => sea_query::Value::String(Some(s)),
				serde_json::Value::Number(n) => {
					if let Some(i) = n.as_i64() {
						sea_query::Value::BigInt(Some(i))
					} else if let Some(f) = n.as_f64() {
						sea_query::Value::Double(Some(f))
					} else {
						sea_query::Value::String(Some(n.to_string()))
					}
				}
				serde_json::Value::Bool(b) => sea_query::Value::Bool(Some(b)),
				serde_json::Value::Null => sea_query::Value::Int(None),
				_ => sea_query::Value::String(Some(value.to_string())),
			};
			query.value(Alias::new(&key), sea_value);
		}

		query.and_where(Expr::col(Alias::new(pk_field)).eq(id));

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
	/// use reinhardt_admin_core::AdminDatabase;
	/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model};
	/// use std::sync::Arc;
	/// use serde::{Serialize, Deserialize};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// db.delete::<User>("users", "id", "1").await?;
	/// # Ok(())
	/// # }
	///
	/// #[derive(Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
	/// }
	/// ```
	pub async fn delete<M: Model>(
		&self,
		table_name: &str,
		pk_field: &str,
		id: &str,
	) -> AdminResult<u64> {
		let query = SeaQuery::delete()
			.from_table(Alias::new(table_name))
			.and_where(Expr::col(Alias::new(pk_field)).eq(id))
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
	/// use reinhardt_admin_core::AdminDatabase;
	/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model};
	/// use std::sync::Arc;
	/// use serde::{Serialize, Deserialize};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
	/// db.bulk_delete::<User>("users", "id", ids).await?;
	/// # Ok(())
	/// # }
	///
	/// #[derive(Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
	/// }
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
	/// use reinhardt_admin_core::AdminDatabase;
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

		let query = SeaQuery::delete()
			.from_table(Alias::new(table_name))
			.and_where(Expr::col(Alias::new(pk_field)).is_in(ids))
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
	/// use reinhardt_admin_core::AdminDatabase;
	/// use reinhardt_db::orm::{DatabaseConnection, DatabaseBackend, Model, Filter, FilterOperator, FilterValue};
	/// use std::sync::Arc;
	/// use serde::{Serialize, Deserialize};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let conn = DatabaseConnection::connect("postgres://localhost/test").await?;
	/// let db = AdminDatabase::new(conn);
	///
	/// let filters = vec![
	///     Filter::new("is_active".to_string(), FilterOperator::Eq, FilterValue::Boolean(true))
	/// ];
	///
	/// let count = db.count::<User>("users", filters).await?;
	/// # Ok(())
	/// # }
	///
	/// #[derive(Clone, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, pk: Self::PrimaryKey) { self.id = Some(pk); }
	/// }
	/// ```
	pub async fn count<M: Model>(
		&self,
		table_name: &str,
		filters: Vec<Filter>,
	) -> AdminResult<u64> {
		let mut query = SeaQuery::select()
			.from(Alias::new(table_name))
			.expr(Expr::cust("COUNT(*)"))
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
/// The implementation resolves Arc<AdminDatabase> from the container
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
	use reinhardt_test::fixtures::mock_connection;
	use rstest::*;

	// Mock User model for testing
	#[derive(Clone, serde::Serialize, serde::Deserialize)]
	struct User {
		id: i64,
		name: String,
	}

	impl Model for User {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"users"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			Some(&self.id)
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
		let query = SeaQuery::select()
			.from(Alias::new("users"))
			.column(Asterisk)
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
		let query = SeaQuery::select()
			.from(Alias::new("users"))
			.column(Asterisk)
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
		let query = SeaQuery::select()
			.from(Alias::new("users"))
			.column(Asterisk)
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
		let query = SeaQuery::select()
			.from(Alias::new("users"))
			.column(Asterisk)
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
}
