//! Database integration for admin operations
//!
//! This module provides database access layer for admin CRUD operations,
//! integrating with reinhardt-orm's QuerySet API.

use crate::{AdminError, AdminResult};
use reinhardt_orm::{DatabaseConnection, Filter, FilterOperator, FilterValue, Model};
use sea_query::{
	Alias, Asterisk, Condition, Expr, ExprTrait, PostgresQueryBuilder, Query as SeaQuery,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Convert FilterValue to sea_query::Value
fn filter_value_to_sea_value(v: &FilterValue) -> sea_query::Value {
	match v {
		FilterValue::String(s) => s.clone().into(),
		FilterValue::Integer(i) | FilterValue::Int(i) => (*i).into(),
		FilterValue::Float(f) => (*f).into(),
		FilterValue::Boolean(b) | FilterValue::Bool(b) => (*b).into(),
		FilterValue::Null => sea_query::Value::Int(None),
		FilterValue::Array(_) => sea_query::Value::String(None),
	}
}

/// Build sea-query Condition from filters
fn build_filter_condition(filters: &[Filter]) -> Option<Condition> {
	if filters.is_empty() {
		return None;
	}

	let mut condition = Condition::all();

	for filter in filters {
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
			_ => continue, // Skip unsupported combinations
		};

		condition = condition.add(expr);
	}

	Some(condition)
}

/// Admin database interface
///
/// Provides CRUD operations for admin panel, leveraging reinhardt-orm.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::AdminDatabase;
/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model};
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
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model, Filter, FilterOperator, FilterValue};
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
			.map_err(AdminError::DatabaseError)?;

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

	/// Get a single item by ID
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model};
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
			.map_err(AdminError::DatabaseError)?;

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
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model};
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
			.map_err(AdminError::DatabaseError)?;

		Ok(affected)
	}

	/// Update an existing item
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model};
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
			.map_err(AdminError::DatabaseError)?;

		Ok(affected)
	}

	/// Delete an item by ID
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model};
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
			.map_err(AdminError::DatabaseError)?;

		Ok(affected)
	}

	/// Delete multiple items by IDs (bulk delete)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model};
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
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::DatabaseConnection;
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
			.map_err(AdminError::DatabaseError)?;

		Ok(affected)
	}

	/// Count total items with optional filters
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::AdminDatabase;
	/// use reinhardt_orm::{DatabaseConnection, DatabaseBackend, Model, Filter, FilterOperator, FilterValue};
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
			.map_err(AdminError::DatabaseError)?;

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

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_orm::DatabaseBackend;
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
}
