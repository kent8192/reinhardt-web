//! Performance optimization utilities for serializers
//!
//! This module provides:
//! - Batch validation query optimization
//! - Field introspection result caching
//! - Query result caching
//! - Performance monitoring utilities

use super::SerializerError;
use crate::metadata::FieldInfo;
// Note: reinhardt_db::orm::Model will be needed when database validation is implemented
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

/// Internal type for representing unique constraint checks
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are used in database validation
enum UniqueCheckQuery {
	/// Single field uniqueness check
	Single {
		check_id: String,
		field: String,
		value: String,
	},
	/// Multiple fields uniqueness check (unique_together)
	Together {
		check_id: String,
		fields: Vec<String>,
		values: Vec<String>,
	},
}

impl UniqueCheckQuery {
	/// Get the check identifier
	fn check_id(&self) -> &str {
		match self {
			UniqueCheckQuery::Single { check_id, .. } => check_id,
			UniqueCheckQuery::Together { check_id, .. } => check_id,
		}
	}
}

/// Cache for field introspection results
///
/// Stores field metadata to avoid repeated introspection operations.
/// Thread-safe with RwLock for concurrent access.
///
/// # Examples
///
/// ```
/// use reinhardt_rest::serializers::performance::IntrospectionCache;
///
/// let cache = IntrospectionCache::new();
///
/// // Cache miss - introspect and store
/// if cache.get("User").is_none() {
///     let fields = vec![/* field info */];
///     cache.set("User".to_string(), fields.clone());
/// }
///
/// // Cache hit - retrieve and verify
/// if let Some(fields) = cache.get("User") {
///     assert_eq!(fields.len(), 0); // Empty vec in this example
/// }
/// ```
#[derive(Debug, Clone)]
pub struct IntrospectionCache {
	cache: Arc<RwLock<HashMap<String, Vec<FieldInfo>>>>,
}

impl IntrospectionCache {
	/// Create a new introspection cache
	pub fn new() -> Self {
		Self {
			cache: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Get cached field info for a model
	///
	/// Returns `None` if not cached.
	pub fn get(&self, model_name: &str) -> Option<Vec<FieldInfo>> {
		self.cache
			.read()
			.ok()
			.and_then(|cache| cache.get(model_name).cloned())
	}

	/// Set field info for a model
	pub fn set(&self, model_name: String, fields: Vec<FieldInfo>) {
		if let Ok(mut cache) = self.cache.write() {
			cache.insert(model_name, fields);
		}
	}

	/// Clear all cached entries
	pub fn clear(&self) {
		if let Ok(mut cache) = self.cache.write() {
			cache.clear();
		}
	}

	/// Get cache size (number of models)
	pub fn size(&self) -> usize {
		self.cache.read().ok().map(|cache| cache.len()).unwrap_or(0)
	}

	/// Check if a model is cached
	pub fn contains(&self, model_name: &str) -> bool {
		self.cache
			.read()
			.ok()
			.map(|cache| cache.contains_key(model_name))
			.unwrap_or(false)
	}
}

impl Default for IntrospectionCache {
	fn default() -> Self {
		Self::new()
	}
}

/// Batch validation query builder
///
/// Optimizes validation by combining multiple database checks
/// into a single query when possible.
///
/// # Examples
///
/// ```rust,no_run,ignore
/// # #[tokio::main]
/// # async fn main() {
/// use reinhardt_rest::serializers::performance::BatchValidator;
///
/// let mut validator = BatchValidator::new();
///
/// // Add multiple checks
/// validator.add_unique_check("users", "email", "alice@example.com");
/// validator.add_unique_check("users", "username", "alice");
///
/// // Execute all checks in optimized query
/// let results = validator.execute().await?;
///
/// # }
/// ```
#[derive(Debug)]
pub struct BatchValidator {
	/// Unique constraint checks: (table, field, value)
	unique_checks: Vec<(String, String, String)>,
	/// Unique together checks: (table, fields, values)
	unique_together_checks: Vec<(String, Vec<String>, Vec<String>)>,
}

impl BatchValidator {
	/// Create a new batch validator
	pub fn new() -> Self {
		Self {
			unique_checks: Vec::new(),
			unique_together_checks: Vec::new(),
		}
	}

	/// Add a unique field check.
	///
	/// For PostgreSQL JSONB columns (including domains), pass a serialized JSON
	/// value, such as `r#"{"a":1}"#`. JSON strings include their JSON quotes.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::serializers::performance::BatchValidator;
	///
	/// let mut validator = BatchValidator::new();
	/// validator.add_unique_check("users", "email", "test@example.com");
	/// ```
	pub fn add_unique_check(
		&mut self,
		table: impl Into<String>,
		field: impl Into<String>,
		value: impl Into<String>,
	) {
		self.unique_checks
			.push((table.into(), field.into(), value.into()));
	}

	/// Add a unique together check
	pub fn add_unique_together_check(
		&mut self,
		table: impl Into<String>,
		fields: Vec<String>,
		values: Vec<String>,
	) {
		self.unique_together_checks
			.push((table.into(), fields, values));
	}

	/// Execute all validation checks
	///
	/// Returns map of failed checks: key is check identifier, value is error message
	///
	/// # Implementation Strategy
	///
	/// 1. **Group checks by table**: Organize all validation checks by their target table
	/// 2. **Build optimized queries**: Create UNION ALL queries for checks on the same table
	/// 3. **Execute batched queries**: Run one validation query per table instead of
	///    N individual queries. PostgreSQL additionally resolves column types
	///    once per table, including domains and type modifiers.
	/// 4. **Map results**: Parse query results and identify which specific checks failed
	///
	/// # Performance Benefits
	///
	/// - Groups N checks into T validation round-trips (T is the number of tables),
	///   plus one type-introspection round-trip per PostgreSQL table
	/// - Leverages database query optimization for UNION ALL
	/// - Minimizes connection pool contention
	///
	/// # Example SQL Generated
	///
	/// For multiple unique checks on the "users" table:
	/// ```sql
	/// SELECT 'email:alice@example.com' as check_id, COUNT(*) as count
	/// FROM users WHERE email = 'alice@example.com'
	/// UNION ALL
	/// SELECT 'username:alice' as check_id, COUNT(*) as count
	/// FROM users WHERE username = 'alice'
	/// ```
	pub async fn execute(&self) -> Result<HashMap<String, String>, SerializerError> {
		let mut failed_checks = HashMap::new();

		// Step 1: Group checks by table
		let mut checks_by_table: HashMap<String, Vec<UniqueCheckQuery>> = HashMap::new();

		// Add unique field checks
		for (table, field, value) in &self.unique_checks {
			let check_id = format!("{}:{}:{}", table, field, value);
			let query = UniqueCheckQuery::Single {
				check_id,
				field: field.clone(),
				value: value.clone(),
			};
			checks_by_table
				.entry(table.clone())
				.or_default()
				.push(query);
		}

		// Add unique together checks
		for (table, fields, values) in &self.unique_together_checks {
			let check_id = format!("{}:{}:{}", table, fields.join("+"), values.join("+"));
			let query = UniqueCheckQuery::Together {
				check_id,
				fields: fields.clone(),
				values: values.clone(),
			};
			checks_by_table
				.entry(table.clone())
				.or_default()
				.push(query);
		}

		// Step 2 & 3: Build and execute queries per table
		for (table, queries) in checks_by_table {
			match self.execute_table_checks(&table, &queries).await {
				Ok(table_failures) => {
					failed_checks.extend(table_failures);
				}
				Err(e) => {
					// Log error but continue with other tables
					eprintln!("Error executing checks for table {}: {}", table, e);
					// Mark all checks for this table as failed
					for query in queries {
						let check_id = query.check_id();
						failed_checks
							.insert(check_id.to_string(), format!("Database error: {}", e));
					}
				}
			}
		}

		Ok(failed_checks)
	}

	/// Execute all validation checks for a single table
	///
	/// Builds and executes a parameterized UNION ALL query for the table.
	async fn execute_table_checks(
		&self,
		table: &str,
		queries: &[UniqueCheckQuery],
	) -> Result<HashMap<String, String>, SerializerError> {
		use reinhardt_db::orm::manager::get_connection;

		let mut failed_checks = HashMap::new();

		// Get database connection
		let conn = get_connection().await.map_err(|e| SerializerError::Other {
			message: format!("Failed to get database connection: {}", e),
		})?;

		let column_types =
			if conn.backend() == reinhardt_db::orm::connection::DatabaseBackend::Postgres {
				let table_name = quoted_validation_table(table, conn.backend())?;
				// PostgreSQL format_type emits valid, quoted SQL type names, including
				// domains and typmods. Only server-produced names enter the cast syntax.
				let rows = conn
					.query(
						concat!(
							"SELECT attr.attname AS field, ",
							"pg_catalog.format_type(attr.atttypid, attr.atttypmod) AS cast_type ",
							"FROM pg_catalog.pg_attribute attr ",
							"WHERE attr.attrelid = $1::text::regclass AND attr.attnum > 0 ",
							"AND NOT attr.attisdropped",
						),
						vec![reinhardt_db::orm::connection::QueryValue::String(
							table_name,
						)],
					)
					.await
					.map_err(|error| SerializerError::Other {
						message: format!("Failed to resolve validation column types: {error}"),
					})?;
				rows.into_iter()
					.filter_map(|row| {
						let field = row.data.get("field")?.as_str()?.to_owned();
						let type_name = row.data.get("cast_type")?.as_str()?.to_owned();
						Some((field, type_name))
					})
					.collect()
			} else {
				HashMap::new()
			};
		let (final_query, params) =
			build_validation_query(table, queries, conn.backend(), &column_types)?;

		// Execute query
		let rows = conn
			.query(&final_query, params)
			.await
			.map_err(|e| SerializerError::Other {
				message: format!("Failed to execute batch validation query: {}", e),
			})?;

		// Parse results and identify failed checks
		for row in rows {
			// Extract check_id and count from the row
			// The row.data should be a JSON object with "check_id" and "cnt" fields
			if let Some(obj) = row.data.as_object() {
				let check_id = obj
					.get("check_id")
					.and_then(|v| v.as_str())
					.unwrap_or("unknown");
				let count = obj.get("cnt").and_then(|v| v.as_i64()).unwrap_or(0);

				if count > 0 {
					// Extract field name from check_id for error message
					// Format: "table:field:value" or "table:field1+field2:value1+value2"
					let parts: Vec<&str> = check_id.split(':').collect();
					let field_info = if parts.len() >= 2 { parts[1] } else { "field" };

					let error_msg = if field_info.contains('+') {
						format!("Combination of {} already exists", field_info)
					} else {
						format!("{} already exists", field_info)
					};

					failed_checks.insert(check_id.to_string(), error_msg);
				}
			}
		}

		Ok(failed_checks)
	}

	/// Get number of pending checks
	pub fn pending_count(&self) -> usize {
		self.unique_checks.len() + self.unique_together_checks.len()
	}

	/// Clear all pending checks
	pub fn clear(&mut self) {
		self.unique_checks.clear();
		self.unique_together_checks.clear();
	}
}

fn validation_identifier(identifier: &str) -> Result<&str, SerializerError> {
	if !identifier.is_empty()
		&& identifier
			.chars()
			.all(|c| c.is_ascii_alphanumeric() || c == '_')
		&& !identifier.as_bytes()[0].is_ascii_digit()
	{
		Ok(identifier)
	} else {
		Err(SerializerError::Other {
			message: "Invalid validation table or field identifier".to_string(),
		})
	}
}

fn quoted_validation_table(
	table: &str,
	backend: reinhardt_db::orm::connection::DatabaseBackend,
) -> Result<String, SerializerError> {
	let quote = if backend == reinhardt_db::orm::connection::DatabaseBackend::MySql {
		'`'
	} else {
		'"'
	};
	table
		.split('.')
		.map(|part| validation_identifier(part).map(|part| format!("{quote}{part}{quote}")))
		.collect::<Result<Vec<_>, _>>()
		.map(|parts| parts.join("."))
}

fn build_validation_query(
	table: &str,
	queries: &[UniqueCheckQuery],
	backend: reinhardt_db::orm::connection::DatabaseBackend,
	column_types: &HashMap<String, String>,
) -> Result<(String, Vec<reinhardt_db::orm::connection::QueryValue>), SerializerError> {
	use reinhardt_db::orm::connection::{DatabaseBackend, QueryValue};

	let quote = if backend == DatabaseBackend::MySql {
		'`'
	} else {
		'"'
	};
	let table = quoted_validation_table(table, backend)?;
	let mut params = Vec::new();
	let mut subqueries = Vec::new();
	for query in queries {
		let (check_id, fields, values): (&str, Vec<&str>, Vec<&str>) = match query {
			UniqueCheckQuery::Single {
				check_id,
				field,
				value,
			} => (check_id, vec![field], vec![value]),
			UniqueCheckQuery::Together {
				check_id,
				fields,
				values,
			} => (
				check_id,
				fields.iter().map(String::as_str).collect(),
				values.iter().map(String::as_str).collect(),
			),
		};
		if fields.is_empty() || fields.len() != values.len() {
			return Err(SerializerError::Other {
				message: "Validation fields and values must have equal nonzero lengths".to_string(),
			});
		}
		let placeholder = |number: usize| {
			if backend == DatabaseBackend::Postgres {
				format!("${number}")
			} else {
				"?".to_string()
			}
		};
		params.push(QueryValue::String(check_id.to_string()));
		let check_placeholder = placeholder(params.len());
		let mut conditions = Vec::with_capacity(fields.len());
		for (field, value) in fields.into_iter().zip(values) {
			let field = validation_identifier(field)?;
			params.push(QueryValue::String(value.to_string()));
			let value_placeholder = placeholder(params.len());
			// Bind as text first, then invoke the actual column's input cast.
			// JSONB values stay JSON (including JSON null), and domains/typmods
			// apply only to this field rather than a partly-null composite row.
			let postgres_value = if backend == DatabaseBackend::Postgres {
				let type_name = column_types
					.get(field)
					.ok_or_else(|| SerializerError::Other {
						message: format!("Unknown validation column: {field}"),
					})?;
				Some(format!("{value_placeholder}::text::{type_name}"))
			} else {
				None
			};
			let typed_value = postgres_value.as_deref().unwrap_or(&value_placeholder);
			conditions.push(format!("{quote}{field}{quote} = {typed_value}"));
		}
		subqueries.push(format!(
			"SELECT {check_placeholder} as check_id, COUNT(*) as cnt FROM {table} WHERE {}",
			conditions.join(" AND ")
		));
	}
	Ok((subqueries.join(" UNION ALL "), params))
}

impl Default for BatchValidator {
	fn default() -> Self {
		Self::new()
	}
}

/// Query result cache
///
/// Caches database query results with TTL (time-to-live) support.
///
/// # Examples
///
/// ```
/// use reinhardt_rest::serializers::performance::QueryCache;
/// use std::time::Duration;
///
/// let cache = QueryCache::new(Duration::from_secs(300)); // 5 minutes TTL
///
/// // Cache a query result
/// cache.set("user:123".to_string(), serde_json::json!({"id": 123, "name": "Alice"}));
///
/// // Retrieve cached result and verify
/// if let Some(user) = cache.get("user:123") {
///     assert_eq!(user["id"], 123);
///     assert_eq!(user["name"], "Alice");
/// }
/// ```
#[derive(Debug)]
pub struct QueryCache {
	cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
	ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
	data: serde_json::Value,
	inserted_at: Instant,
}

impl QueryCache {
	/// Create a new query cache with specified TTL
	pub fn new(ttl: Duration) -> Self {
		Self {
			cache: Arc::new(RwLock::new(HashMap::new())),
			ttl,
		}
	}

	/// Get cached value
	///
	/// Returns `None` if not cached or expired.
	pub fn get(&self, key: &str) -> Option<serde_json::Value> {
		if let Ok(cache) = self.cache.read()
			&& let Some(entry) = cache.get(key)
			&& entry.inserted_at.elapsed() < self.ttl
		{
			return Some(entry.data.clone());
		}
		None
	}

	/// Set cached value
	pub fn set(&self, key: String, data: serde_json::Value) {
		if let Ok(mut cache) = self.cache.write() {
			cache.insert(
				key,
				CacheEntry {
					data,
					inserted_at: Instant::now(),
				},
			);
		}
	}

	/// Invalidate (remove) cached entry
	pub fn invalidate(&self, key: &str) {
		if let Ok(mut cache) = self.cache.write() {
			cache.remove(key);
		}
	}

	/// Clear all cached entries
	pub fn clear(&self) {
		if let Ok(mut cache) = self.cache.write() {
			cache.clear();
		}
	}

	/// Remove expired entries
	pub fn prune_expired(&self) {
		if let Ok(mut cache) = self.cache.write() {
			cache.retain(|_, entry| entry.inserted_at.elapsed() < self.ttl);
		}
	}

	/// Get cache size (number of entries)
	pub fn size(&self) -> usize {
		self.cache.read().ok().map(|cache| cache.len()).unwrap_or(0)
	}
}

/// Performance metrics collector
///
/// Tracks serialization and validation performance metrics.
///
/// # Examples
///
/// ```
/// use reinhardt_rest::serializers::performance::PerformanceMetrics;
///
/// let metrics = PerformanceMetrics::new();
///
/// // Record a serialization operation
/// metrics.record_serialization(50); // 50ms
/// metrics.record_serialization(30); // 30ms
///
/// // Get statistics and verify
/// let stats = metrics.get_stats();
/// assert_eq!(stats.total_serializations, 2);
/// assert_eq!(stats.avg_serialization_ms, 40.0); // (50 + 30) / 2
/// ```
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
	serialization_times: Arc<RwLock<Vec<u64>>>,
	validation_times: Arc<RwLock<Vec<u64>>>,
}

/// Aggregated performance statistics for serialization and validation operations.
#[derive(Debug, Clone)]
pub struct PerformanceStats {
	/// Total number of serialization operations recorded.
	pub total_serializations: usize,
	/// Total number of validation operations recorded.
	pub total_validations: usize,
	/// Average serialization time in milliseconds.
	pub avg_serialization_ms: f64,
	/// Average validation time in milliseconds.
	pub avg_validation_ms: f64,
	/// Maximum serialization time in milliseconds.
	pub max_serialization_ms: u64,
	/// Maximum validation time in milliseconds.
	pub max_validation_ms: u64,
}

impl PerformanceMetrics {
	/// Create a new metrics collector
	pub fn new() -> Self {
		Self {
			serialization_times: Arc::new(RwLock::new(Vec::new())),
			validation_times: Arc::new(RwLock::new(Vec::new())),
		}
	}

	/// Record a serialization time in milliseconds
	pub fn record_serialization(&self, time_ms: u64) {
		if let Ok(mut times) = self.serialization_times.write() {
			times.push(time_ms);
		}
	}

	/// Record a validation time in milliseconds
	pub fn record_validation(&self, time_ms: u64) {
		if let Ok(mut times) = self.validation_times.write() {
			times.push(time_ms);
		}
	}

	/// Get performance statistics
	pub fn get_stats(&self) -> PerformanceStats {
		let ser_times = self
			.serialization_times
			.read()
			.ok()
			.map(|t| t.clone())
			.unwrap_or_default();
		let val_times = self
			.validation_times
			.read()
			.ok()
			.map(|t| t.clone())
			.unwrap_or_default();

		let avg_ser = if !ser_times.is_empty() {
			ser_times.iter().sum::<u64>() as f64 / ser_times.len() as f64
		} else {
			0.0
		};

		let avg_val = if !val_times.is_empty() {
			val_times.iter().sum::<u64>() as f64 / val_times.len() as f64
		} else {
			0.0
		};

		PerformanceStats {
			total_serializations: ser_times.len(),
			total_validations: val_times.len(),
			avg_serialization_ms: avg_ser,
			avg_validation_ms: avg_val,
			max_serialization_ms: ser_times.iter().max().copied().unwrap_or(0),
			max_validation_ms: val_times.iter().max().copied().unwrap_or(0),
		}
	}

	/// Clear all metrics
	pub fn clear(&self) {
		if let Ok(mut times) = self.serialization_times.write() {
			times.clear();
		}
		if let Ok(mut times) = self.validation_times.write() {
			times.clear();
		}
	}
}

impl Default for PerformanceMetrics {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_introspection_cache_basic() {
		let cache = IntrospectionCache::new();
		assert_eq!(cache.size(), 0);

		let fields = vec![];
		cache.set("User".to_string(), fields.clone());

		assert_eq!(cache.size(), 1);
		assert!(cache.contains("User"));
		assert!(cache.get("User").is_some());
	}

	#[test]
	fn test_introspection_cache_clear() {
		let cache = IntrospectionCache::new();
		cache.set("User".to_string(), vec![]);
		cache.set("Post".to_string(), vec![]);

		assert_eq!(cache.size(), 2);

		cache.clear();
		assert_eq!(cache.size(), 0);
		assert!(!cache.contains("User"));
	}

	#[tokio::test]
	async fn test_batch_validator_unique_together() {
		let mut validator = BatchValidator::new();

		validator.add_unique_together_check(
			"posts",
			vec!["user_id".to_string(), "slug".to_string()],
			vec!["123".to_string(), "my-post".to_string()],
		);

		assert_eq!(validator.pending_count(), 1);

		// Test execute
		let result = validator.execute().await;
		assert!(result.is_ok());
	}

	#[rstest::rstest]
	fn test_batch_validation_binds_values_and_rejects_sql_identifiers() {
		use reinhardt_db::orm::connection::{DatabaseBackend, QueryValue};

		let checks = [UniqueCheckQuery::Single {
			check_id: "users:email:x' OR 1=1 --".to_string(),
			field: "email".to_string(),
			value: "x' OR 1=1 --".to_string(),
		}];
		let (sql, params) = build_validation_query(
			"users",
			&checks,
			DatabaseBackend::Postgres,
			&HashMap::from([("email".to_owned(), "text".to_owned())]),
		)
		.unwrap();
		assert_eq!(
			sql,
			"SELECT $1 as check_id, COUNT(*) as cnt FROM \"users\" WHERE \"email\" = $2::text::text"
		);
		assert!(!sql.contains("x' OR 1=1 --"));
		assert_eq!(params.len(), 2);
		assert!(
			matches!(&params[0], QueryValue::String(value) if value == "users:email:x' OR 1=1 --")
		);
		assert!(matches!(&params[1], QueryValue::String(value) if value == "x' OR 1=1 --"));
		assert!(
			build_validation_query(
				"users; DROP TABLE users",
				&checks,
				DatabaseBackend::Sqlite,
				&HashMap::new()
			)
			.is_err()
		);
		let (mysql_sql, _) =
			build_validation_query("users", &checks, DatabaseBackend::MySql, &HashMap::new())
				.unwrap();
		assert_eq!(
			mysql_sql,
			"SELECT ? as check_id, COUNT(*) as cnt FROM `users` WHERE `email` = ?"
		);
		let (sqlite_sql, _) =
			build_validation_query("users", &checks, DatabaseBackend::Sqlite, &HashMap::new())
				.unwrap();
		assert_eq!(
			sqlite_sql,
			"SELECT ? as check_id, COUNT(*) as cnt FROM \"users\" WHERE \"email\" = ?"
		);
	}

	#[test]
	fn test_query_cache_basic() {
		let cache = QueryCache::new(Duration::from_secs(60));

		let data = serde_json::json!({"id": 123, "name": "Alice"});
		cache.set("user:123".to_string(), data.clone());

		let cached = cache.get("user:123");
		assert!(cached.is_some());
		assert_eq!(cached.unwrap(), data);
	}

	#[test]
	fn test_query_cache_miss() {
		let cache = QueryCache::new(Duration::from_secs(60));
		assert!(cache.get("nonexistent").is_none());
	}

	#[test]
	fn test_query_cache_invalidate() {
		let cache = QueryCache::new(Duration::from_secs(60));

		cache.set("user:123".to_string(), serde_json::json!({"id": 123}));
		assert!(cache.get("user:123").is_some());

		cache.invalidate("user:123");
		assert!(cache.get("user:123").is_none());
	}

	#[test]
	fn test_query_cache_clear() {
		let cache = QueryCache::new(Duration::from_secs(60));

		cache.set("key1".to_string(), serde_json::json!(1));
		cache.set("key2".to_string(), serde_json::json!(2));

		assert_eq!(cache.size(), 2);

		cache.clear();
		assert_eq!(cache.size(), 0);
	}

	#[test]
	fn test_performance_metrics_basic() {
		let metrics = PerformanceMetrics::new();

		metrics.record_serialization(50);
		metrics.record_serialization(30);
		metrics.record_validation(20);

		let stats = metrics.get_stats();

		assert_eq!(stats.total_serializations, 2);
		assert_eq!(stats.total_validations, 1);
		assert_eq!(stats.avg_serialization_ms, 40.0);
		assert_eq!(stats.max_serialization_ms, 50);
		assert_eq!(stats.avg_validation_ms, 20.0);
	}

	#[test]
	fn test_performance_metrics_clear() {
		let metrics = PerformanceMetrics::new();

		metrics.record_serialization(50);
		metrics.record_validation(20);

		let stats = metrics.get_stats();
		assert_eq!(stats.total_serializations, 1);
		assert_eq!(stats.total_validations, 1);

		metrics.clear();

		let stats = metrics.get_stats();
		assert_eq!(stats.total_serializations, 0);
		assert_eq!(stats.total_validations, 0);
	}

	#[test]
	fn test_batch_validator_default() {
		let validator = BatchValidator::default();
		assert_eq!(validator.pending_count(), 0);
	}
}

/// N+1 query problem detector
///
/// Tracks query patterns to detect and warn about N+1 query problems.
/// Useful during development and testing to identify performance issues.
///
/// # Examples
///
/// ```
/// use reinhardt_rest::serializers::performance::N1Detector;
///
/// let mut detector = N1Detector::new();
///
/// // Simulate N queries in a loop
/// for i in 0..100 {
///     detector.record_query("SELECT * FROM users WHERE id = ?", &[i.to_string()]);
/// }
///
/// // Check for N+1 pattern and verify detection
/// if let Some(warning) = detector.check_n_plus_1() {
///     assert!(warning.contains("N+1 query detected"));
///     assert!(warning.contains("executed 100 times"));
/// }
/// ```
#[derive(Debug, Clone)]
pub struct N1Detector {
	queries: Vec<(String, Vec<String>, Instant)>,
	threshold: usize,
}

impl N1Detector {
	/// Create a new N+1 detector with default threshold (10)
	pub fn new() -> Self {
		Self {
			queries: Vec::new(),
			threshold: 10,
		}
	}

	/// Create a new N+1 detector with custom threshold
	pub fn with_threshold(threshold: usize) -> Self {
		Self {
			queries: Vec::new(),
			threshold,
		}
	}

	/// Record a query execution
	pub fn record_query(&mut self, sql: &str, params: &[String]) {
		self.queries
			.push((sql.to_string(), params.to_vec(), Instant::now()));
	}

	/// Check for N+1 query patterns
	///
	/// Returns a warning message if N+1 pattern is detected.
	pub fn check_n_plus_1(&self) -> Option<String> {
		if self.queries.len() < self.threshold {
			return None;
		}

		// Group queries by normalized SQL (without parameters)
		let mut query_counts: HashMap<String, usize> = HashMap::new();
		for (sql, _, _) in &self.queries {
			let normalized = self.normalize_query(sql);
			*query_counts.entry(normalized).or_insert(0) += 1;
		}

		// Find queries executed more than threshold times
		for (normalized_sql, count) in query_counts {
			if count >= self.threshold {
				return Some(format!(
					"N+1 query detected: '{}' executed {} times. \
                     Consider using select_related() or prefetch_related().",
					normalized_sql, count
				));
			}
		}

		None
	}

	/// Normalize SQL query by removing specific values
	///
	/// Uses regex for O(n) performance instead of O(n²) with multiple replacements.
	/// The regex is compiled once and cached for optimal performance.
	fn normalize_query(&self, sql: &str) -> String {
		// Cache compiled regex as a static for O(1) access
		static NUMBER_PATTERN: OnceLock<Regex> = OnceLock::new();
		let re = NUMBER_PATTERN.get_or_init(|| {
			// Pattern matches integers and floating point numbers
			Regex::new(r"\b\d+(\.\d+)?\b").unwrap()
		});

		// Replace all numbers with placeholder in a single pass: O(n)
		re.replace_all(sql, "?").to_string()
	}

	/// Clear recorded queries
	pub fn clear(&mut self) {
		self.queries.clear();
	}

	/// Get total number of recorded queries
	pub fn query_count(&self) -> usize {
		self.queries.len()
	}
}

impl Default for N1Detector {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod n1_detector_tests {
	use super::*;

	#[test]
	fn test_n1_detector_basic() {
		let mut detector = N1Detector::new();

		// Simulate N+1 pattern
		for i in 0..15 {
			detector.record_query("SELECT * FROM users WHERE id = $1", &[i.to_string()]);
		}

		assert_eq!(detector.query_count(), 15);

		let warning = detector.check_n_plus_1();
		assert!(warning.is_some());
		assert!(warning.unwrap().contains("N+1 query detected"));
	}

	#[test]
	fn test_n1_detector_below_threshold() {
		let mut detector = N1Detector::with_threshold(20);

		// Only 10 queries - below threshold
		for i in 0..10 {
			detector.record_query("SELECT * FROM users WHERE id = $1", &[i.to_string()]);
		}

		let warning = detector.check_n_plus_1();
		assert!(warning.is_none());
	}

	#[test]
	fn test_n1_detector_different_queries() {
		let mut detector = N1Detector::new();

		// Different queries - no N+1 pattern
		detector.record_query("SELECT * FROM users", &[]);
		detector.record_query("SELECT * FROM posts", &[]);
		detector.record_query("SELECT * FROM comments", &[]);

		let warning = detector.check_n_plus_1();
		assert!(warning.is_none());
	}

	#[test]
	fn test_n1_detector_clear() {
		let mut detector = N1Detector::new();

		for i in 0..15 {
			detector.record_query("SELECT * FROM users WHERE id = $1", &[i.to_string()]);
		}

		assert_eq!(detector.query_count(), 15);
		detector.clear();
		assert_eq!(detector.query_count(), 0);

		let warning = detector.check_n_plus_1();
		assert!(warning.is_none());
	}
}
