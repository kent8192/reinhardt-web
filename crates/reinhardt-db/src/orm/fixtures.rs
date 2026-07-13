//! Django-compatible model fixture loading and dumping.
//!
//! The fixture runtime is type-erased at the registry boundary, while each
//! registered model still loads through its generated `Model` implementation
//! and canonical fixture-field validation.

use super::connection::{DatabaseBackend, QueryValue};
use super::manager::get_connection;
use super::transaction::TransactionScope;
use super::{DatabaseConnection, Manager, Model};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use reinhardt_query::prelude::{
	Alias, DeleteStatement, Expr, ExprTrait, InsertStatement, MySqlQueryBuilder, OnConflict,
	PostgresQueryBuilder, Query, QueryBuilder, SelectStatement, SqliteQueryBuilder,
	UpdateStatement, Values,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Result type for fixture operations.
pub type FixtureResult<T> = Result<T, FixtureError>;

/// Errors returned by fixture loading and dumping.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
	/// The fixture JSON had an invalid model label.
	#[error("invalid fixture model label '{0}'; expected app_label.ModelName")]
	InvalidModelLabel(String),
	/// No registered fixture handler matched the requested model.
	#[error("fixture model '{0}' is not registered")]
	ModelNotRegistered(String),
	/// The fixture selection matched no models.
	#[error("fixture selector '{0}' matched no registered models")]
	SelectorMatchedNoModels(String),
	/// A fixture record used a model label that disagreed with the selected handler.
	#[error("fixture record model '{actual}' does not match handler '{expected}'")]
	ModelMismatch {
		/// Expected label.
		expected: String,
		/// Actual label.
		actual: String,
	},
	/// Fixture dependency ordering found a cycle.
	#[error("fixture model dependency cycle detected: {0}")]
	DependencyCycle(String),
	/// Serialization or deserialization failed.
	#[error("fixture serialization error: {0}")]
	Serde(#[from] serde_json::Error),
	/// Database execution failed.
	#[error("fixture database error: {0}")]
	Database(String),
}

impl From<anyhow::Error> for FixtureError {
	fn from(error: anyhow::Error) -> Self {
		Self::Database(error.to_string())
	}
}

impl From<reinhardt_core::exception::Error> for FixtureError {
	fn from(error: reinhardt_core::exception::Error) -> Self {
		Self::Database(error.to_string())
	}
}

/// Django-style serialized fixture record.
///
/// The default JSON shape is compatible with Django fixtures:
/// `{ "model": "app_label.ModelName", "pk": 1, "fields": { ... } }`.
///
/// Records that need to retain a JSON null additionally emit the Reinhardt-only
/// `_reinhardt_json_null_fields` sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureRecord {
	/// Fully-qualified model label.
	pub model: String,
	/// Primary key value.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub pk: Option<Value>,
	/// Non-primary-key field values.
	pub fields: Map<String, Value>,
	/// Nullable JSON fields whose `null` values represent JSON null rather than SQL NULL.
	///
	/// This Reinhardt extension preserves the distinction for fixture round trips.
	#[serde(
		default,
		skip_serializing_if = "BTreeSet::is_empty",
		rename = "_reinhardt_json_null_fields"
	)]
	pub json_null_fields: BTreeSet<String>,
}

impl FixtureRecord {
	/// Create a new fixture record.
	pub fn new(model: impl Into<String>, pk: Option<Value>, fields: Map<String, Value>) -> Self {
		Self {
			model: model.into(),
			pk,
			fields,
			json_null_fields: BTreeSet::new(),
		}
	}
}

/// Deserialize a macro-generated fixture projection without requiring callers
/// to depend on `serde_json` directly.
#[doc(hidden)]
pub fn __deserialize_fixture_projection<T>(fields: &super::FixtureFields) -> Result<T, String>
where
	T: serde::de::DeserializeOwned,
{
	serde_json::from_value(Value::Object(fields.clone())).map_err(|error| error.to_string())
}

/// Type-erased fixture handler for one model type.
#[async_trait]
pub trait FixtureModelHandler: Send + Sync {
	/// Application label.
	fn app_label(&self) -> &'static str;
	/// Model name.
	fn model_name(&self) -> &'static str;
	/// Database table name.
	fn table_name(&self) -> &'static str;
	/// Primary key field name.
	fn primary_key_field(&self) -> &'static str;
	/// Physical database column for the primary key.
	fn primary_key_database_column(&self) -> String {
		self.primary_key_field().to_string()
	}

	/// Fully-qualified model label.
	fn label(&self) -> String {
		format!("{}.{}", self.app_label(), self.model_name())
	}

	/// Map physical foreign-key columns to Django fixture relation names.
	#[cfg(feature = "migrations")]
	fn fixture_foreign_key_fields(&self) -> Vec<(String, String)> {
		Vec::new()
	}

	/// Load one fixture record through this model handler.
	async fn load_record(
		&self,
		record: &FixtureRecord,
		conn: &DatabaseConnection,
		tx: &mut TransactionScope,
	) -> FixtureResult<()>;

	/// Load deferred many-to-many assignments for one fixture record.
	async fn load_many_to_many_assignments(
		&self,
		_record: &FixtureRecord,
		_conn: &DatabaseConnection,
		_tx: &mut TransactionScope,
	) -> FixtureResult<()> {
		Ok(())
	}

	/// Dump records for this model.
	async fn dump_records(&self) -> FixtureResult<Vec<FixtureRecord>>;
}

/// Registry of type-erased fixture model handlers.
#[derive(Default)]
pub struct FixtureRegistry {
	models: RwLock<HashMap<String, Arc<dyn FixtureModelHandler>>>,
}

impl FixtureRegistry {
	/// Create an empty registry.
	pub fn new() -> Self {
		Self {
			models: RwLock::new(HashMap::new()),
		}
	}

	/// Register a fixture handler.
	pub fn register(&self, handler: Arc<dyn FixtureModelHandler>) {
		let key = canonical_label(handler.app_label(), handler.model_name());
		if let Ok(mut models) = self.models.write() {
			models.insert(key, handler);
		}
	}

	/// Register a model using the default typed handler.
	pub fn register_model<M>(&self)
	where
		M: Model + 'static,
	{
		self.register(Arc::new(TypedFixtureModel::<M>::new()));
	}

	/// Get a handler by model label.
	pub fn get(&self, label: &str) -> Option<Arc<dyn FixtureModelHandler>> {
		let (app_label, model_name) = parse_model_label(label).ok()?;
		let key = canonical_label(&app_label, &model_name);
		let models = self.models.read().ok()?;
		models
			.get(&key)
			.cloned()
			.or_else(|| find_case_insensitive(&models, &app_label, &model_name))
	}

	/// Return all handlers in stable model-label order.
	pub fn all(&self) -> Vec<Arc<dyn FixtureModelHandler>> {
		let mut handlers = if let Ok(models) = self.models.read() {
			models.values().cloned().collect::<Vec<_>>()
		} else {
			Vec::new()
		};
		handlers.sort_by_key(|handler| handler.label());
		handlers
	}

	/// Remove all registered handlers.
	pub fn clear(&self) {
		if let Ok(mut models) = self.models.write() {
			models.clear();
		}
	}

	/// Return the number of registered handlers.
	pub fn count(&self) -> usize {
		self.models.read().map(|models| models.len()).unwrap_or(0)
	}
}

/// Global fixture registry.
pub fn global_fixture_registry() -> &'static FixtureRegistry {
	static REGISTRY: Lazy<FixtureRegistry> = Lazy::new(FixtureRegistry::new);
	&REGISTRY
}

struct TypedFixtureModel<M> {
	_marker: std::marker::PhantomData<M>,
}

impl<M> TypedFixtureModel<M> {
	fn new() -> Self {
		Self {
			_marker: std::marker::PhantomData,
		}
	}
}

#[async_trait]
impl<M> FixtureModelHandler for TypedFixtureModel<M>
where
	M: Model + 'static,
{
	fn app_label(&self) -> &'static str {
		M::app_label()
	}

	fn model_name(&self) -> &'static str {
		std::any::type_name::<M>()
			.rsplit("::")
			.next()
			.unwrap_or(std::any::type_name::<M>())
	}

	fn table_name(&self) -> &'static str {
		M::table_name()
	}

	fn primary_key_field(&self) -> &'static str {
		M::primary_key_field()
	}

	fn primary_key_database_column(&self) -> String {
		fixture_primary_key_column::<M>()
	}

	#[cfg(feature = "migrations")]
	fn fixture_foreign_key_fields(&self) -> Vec<(String, String)> {
		metadata_for_model::<M>().map_or_else(Vec::new, |metadata| {
			foreign_key_fixture_field_names::<M>(&metadata)
		})
	}

	async fn load_record(
		&self,
		record: &FixtureRecord,
		conn: &DatabaseConnection,
		tx: &mut TransactionScope,
	) -> FixtureResult<()> {
		let expected = self.label();
		if !labels_match(&expected, &record.model) {
			return Err(FixtureError::ModelMismatch {
				expected,
				actual: record.model.clone(),
			});
		}
		ensure_single_column_primary_key::<M>()?;

		let mut object = record.fields.clone();
		remove_many_to_many_fixture_fields::<M>(&mut object)?;
		normalize_foreign_key_fixture_fields::<M>(&mut object)?;
		if let Some(pk) = &record.pk {
			object.insert(M::primary_key_field().to_string(), pk.clone());
		}
		validate_fixture_writable_projection_with_json_null_fields::<M>(
			&object,
			&record.json_null_fields,
		)?;
		execute_fixture_upsert::<M>(conn, tx, &object, &record.json_null_fields).await?;
		Ok(())
	}

	async fn load_many_to_many_assignments(
		&self,
		record: &FixtureRecord,
		conn: &DatabaseConnection,
		tx: &mut TransactionScope,
	) -> FixtureResult<()> {
		let mut object = record.fields.clone();
		let assignments = extract_many_to_many_assignments::<M>(&mut object)?;
		load_many_to_many_assignments::<M>(tx, conn, record.pk.as_ref(), &assignments).await?;
		Ok(())
	}

	async fn dump_records(&self) -> FixtureResult<Vec<FixtureRecord>> {
		ensure_single_column_primary_key::<M>()?;
		let conn = get_connection().await?;
		let select = build_fixture_dump_select::<M>()?;
		let (sql, values) = build_select_sql(&select, conn.backend());
		let rows = conn
			.query(&sql, super::execution::convert_values(values))
			.await?;
		let mut records = Vec::with_capacity(rows.len());
		let model_label = self.label();
		for row in rows {
			let mut object = fixture_fields_from_query_row::<M>(&row)?;
			let mut json_null_fields = fixture_json_null_field_names_from_query_row::<M>(&row)?;
			let pk = object.remove(M::primary_key_field());
			json_null_fields.remove(M::primary_key_field());
			denormalize_foreign_key_fixture_fields::<M>(&mut object)?;
			append_many_to_many_fixture_fields::<M>(&conn, pk.as_ref(), &mut object).await?;
			let mut record = FixtureRecord::new(model_label.clone(), pk, object);
			record.json_null_fields = json_null_fields;
			records.push(record);
		}
		Ok(records)
	}
}

/// Dump fixture records for selected models.
pub async fn dump_fixture_records(
	selectors: &[String],
	excludes: &[String],
) -> FixtureResult<Vec<FixtureRecord>> {
	let handlers = select_handlers(selectors, excludes)?;
	let mut records = Vec::new();
	for handler in handlers {
		records.extend(handler.dump_records().await?);
	}
	Ok(records)
}

/// Load fixture records in a single transaction.
pub async fn load_fixture_records(records: &[FixtureRecord]) -> FixtureResult<usize> {
	let ordered_records = order_records_by_dependencies(records)?;
	let conn = get_connection().await?;
	let mut tx = TransactionScope::begin(&conn).await?;

	for record in &ordered_records {
		let handler = global_fixture_registry()
			.get(&record.model)
			.ok_or_else(|| FixtureError::ModelNotRegistered(record.model.clone()))?;
		handler.load_record(record, &conn, &mut tx).await?;
	}
	for record in &ordered_records {
		let handler = global_fixture_registry()
			.get(&record.model)
			.ok_or_else(|| FixtureError::ModelNotRegistered(record.model.clone()))?;
		handler
			.load_many_to_many_assignments(record, &conn, &mut tx)
			.await?;
	}
	reset_sequences_after_explicit_pks(&conn, &mut tx, &ordered_records).await?;

	tx.commit().await?;
	Ok(ordered_records.len())
}

#[derive(Debug, Clone)]
struct FixtureManyToManySpec {
	field_name: String,
	through_table: String,
	source_field: String,
	target_field: String,
	is_explicit_through: bool,
}

#[derive(Debug, Clone)]
struct FixtureManyToManyAssignment {
	spec: FixtureManyToManySpec,
	values: Vec<Value>,
}

fn build_insert_sql(stmt: &InsertStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_insert(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_insert(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_insert(stmt),
	}
}

fn build_select_sql(stmt: &SelectStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_select(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_select(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_select(stmt),
	}
}

fn build_delete_sql(stmt: &DeleteStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_delete(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_delete(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_delete(stmt),
	}
}

fn build_update_sql(stmt: &UpdateStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_update(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_update(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_update(stmt),
	}
}

async fn execute_fixture_upsert<M>(
	conn: &DatabaseConnection,
	tx: &mut TransactionScope,
	object: &Map<String, Value>,
	json_null_fields: &BTreeSet<String>,
) -> FixtureResult<()>
where
	M: Model,
{
	let database_object = fixture_database_object::<M>(object)?;
	let primary_key = fixture_primary_key_column::<M>();
	if conn.backend() == DatabaseBackend::MySql && database_object.contains_key(&primary_key) {
		let (lookup_sql, lookup_values) =
			build_fixture_primary_key_lookup_sql_values_with_json_null_fields::<M>(
				conn.backend(),
				object,
				json_null_fields,
			)?;
		if tx
			.query_optional(&lookup_sql, lookup_values)
			.await?
			.is_some()
		{
			if let Some((update_sql, update_values)) =
				build_fixture_update_sql_values_with_json_null_fields::<M>(
					conn.backend(),
					object,
					json_null_fields,
				)? {
				tx.execute(&update_sql, update_values).await?;
			}
			return Ok(());
		}
	}

	let (sql, values) = build_fixture_upsert_sql_values_with_json_null_fields::<M>(
		conn.backend(),
		object,
		json_null_fields,
	)?;
	tx.execute(&sql, values).await?;
	Ok(())
}

fn fixture_database_object<M>(object: &Map<String, Value>) -> FixtureResult<Map<String, Value>>
where
	M: Model,
{
	let generated_fields = M::generated_field_names();
	let database_fields = fixture_database_fields::<M>()?;
	let mut database_object = Map::new();
	for (field_name, value) in object {
		if generated_fields.contains(&field_name.as_str()) {
			continue;
		}
		let database_column = database_fields
			.iter()
			.find(|(known_field_name, _)| known_field_name == field_name)
			.map(|(_, database_column)| database_column.clone())
			.ok_or_else(|| {
				FixtureError::Database(format!(
					"fixture field '{}' is not mapped for '{}.{}'",
					field_name,
					M::app_label(),
					rust_model_name::<M>()
				))
			})?;
		if database_object
			.insert(database_column.clone(), value.clone())
			.is_some()
		{
			return Err(FixtureError::Database(format!(
				"fixture fields for '{}.{}' map to the same database column '{}'",
				M::app_label(),
				rust_model_name::<M>(),
				database_column
			)));
		}
	}
	Ok(database_object)
}

#[cfg(test)]
fn validate_fixture_writable_projection<M>(object: &Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	validate_fixture_writable_projection_with_json_null_fields::<M>(object, &BTreeSet::new())
}

fn validate_fixture_writable_projection_with_json_null_fields<M>(
	object: &Map<String, Value>,
	json_null_fields: &BTreeSet<String>,
) -> FixtureResult<()>
where
	M: Model,
{
	M::validate_fixture_fields(object).map_err(|error| {
		FixtureError::Database(format!(
			"fixture validation failed for '{}.{}': {error}",
			M::app_label(),
			rust_model_name::<M>()
		))
	})?;
	validate_fixture_json_null_fields::<M>(object, json_null_fields)?;
	let database_object = fixture_database_object::<M>(object)?;
	let primary_key = fixture_primary_key_column::<M>();
	fixture_writable_columns(&database_object, &primary_key)?;
	Ok(())
}

fn validate_fixture_json_null_fields<M>(
	object: &Map<String, Value>,
	json_null_fields: &BTreeSet<String>,
) -> FixtureResult<()>
where
	M: Model,
{
	let field_metadata = M::field_metadata();
	for field_name in json_null_fields {
		let Some(value) = object.get(field_name) else {
			return Err(FixtureError::Database(format!(
				"fixture JSON-null field '{}' is missing from '{}.{}'",
				field_name,
				M::app_label(),
				rust_model_name::<M>()
			)));
		};
		let Some(field) = field_metadata
			.iter()
			.find(|field| field.name == *field_name)
		else {
			return Err(FixtureError::Database(format!(
				"fixture JSON-null field '{}' is not a model field for '{}.{}'",
				field_name,
				M::app_label(),
				rust_model_name::<M>()
			)));
		};
		if !field.nullable
			|| !crate::orm::json::is_json_field_type(&field.field_type)
			|| !value.is_null()
		{
			return Err(FixtureError::Database(format!(
				"fixture JSON-null field '{}' for '{}.{}' must be a nullable JSON field with a null value",
				field_name,
				M::app_label(),
				rust_model_name::<M>()
			)));
		}
	}
	Ok(())
}

fn fixture_database_fields<M>() -> FixtureResult<Vec<(String, String)>>
where
	M: Model,
{
	let mut fields = Vec::new();
	let mut add_field = |field_name: String, database_column: String| -> FixtureResult<()> {
		if fields.iter().any(|(existing_name, existing_column)| {
			existing_name == &field_name && existing_column == &database_column
		}) {
			return Ok(());
		}
		if fields
			.iter()
			.any(|(existing_name, _)| existing_name == &field_name)
		{
			return Err(FixtureError::Database(format!(
				"fixture field '{}' for '{}.{}' maps to multiple database columns",
				field_name,
				M::app_label(),
				rust_model_name::<M>()
			)));
		}
		if fields
			.iter()
			.any(|(_, existing_column)| existing_column == &database_column)
		{
			return Err(FixtureError::Database(format!(
				"database column '{}' for '{}.{}' maps to multiple fixture fields",
				database_column,
				M::app_label(),
				rust_model_name::<M>()
			)));
		}
		fields.push((field_name, database_column));
		Ok(())
	};

	add_field(
		M::primary_key_field().to_string(),
		fixture_primary_key_column::<M>(),
	)?;
	let field_metadata = M::field_metadata();
	for field in &field_metadata {
		let database_column = field.db_column_name().to_string();
		add_field(field.name.clone(), database_column)?;
	}
	for relationship in M::relationship_metadata() {
		if !matches!(
			relationship.relationship_type,
			crate::orm::relationship::RelationshipType::ManyToOne
				| crate::orm::relationship::RelationshipType::OneToOne
		) {
			continue;
		}
		let Some(foreign_key) = relationship.foreign_key else {
			continue;
		};
		let database_column = field_metadata
			.iter()
			.find(|field| field.name == foreign_key)
			.map(crate::orm::inspection::FieldInfo::db_column_name)
			.unwrap_or(&foreign_key)
			.to_string();
		add_field(foreign_key, database_column)?;
	}

	Ok(fields)
}

fn build_fixture_dump_select<M>() -> FixtureResult<SelectStatement>
where
	M: Model,
{
	let database_fields = fixture_database_fields::<M>()?;
	let primary_key_column = fixture_primary_key_column::<M>();
	let mut select = Query::select();
	select.from(Alias::new(M::table_name()));
	for (_, database_column) in database_fields {
		select.column(Alias::new(database_column));
	}
	select.order_by(
		Alias::new(primary_key_column.as_str()),
		reinhardt_query::prelude::Order::Asc,
	);
	Ok(select)
}

fn fixture_fields_from_query_row<M>(
	row: &super::connection::QueryRow,
) -> FixtureResult<Map<String, Value>>
where
	M: Model,
{
	let database_row = row.data.as_object().ok_or_else(|| {
		FixtureError::Database("fixture query must return object rows".to_string())
	})?;
	fixture_fields_from_database_row_with_json_provenance::<M>(
		database_row,
		row.native_json_fields(),
	)
}

fn fixture_json_null_field_names_from_query_row<M>(
	row: &super::connection::QueryRow,
) -> FixtureResult<BTreeSet<String>>
where
	M: Model,
{
	let database_row = row.data.as_object().ok_or_else(|| {
		FixtureError::Database("fixture query must return object rows".to_string())
	})?;
	let field_metadata = M::field_metadata();
	let mut json_null_fields = BTreeSet::new();
	for (field_name, database_column) in fixture_database_fields::<M>()? {
		let Some(field_info) = field_metadata
			.iter()
			.find(|field| field.name == field_name || field.db_column_name() == database_column)
		else {
			continue;
		};
		if !field_info.nullable || !crate::orm::json::is_json_field_type(&field_info.field_type) {
			continue;
		}
		let value = database_row.get(&database_column).cloned().ok_or_else(|| {
			FixtureError::Database(format!(
				"fixture dump row for '{}.{}' is missing database column '{}'",
				M::app_label(),
				rust_model_name::<M>(),
				database_column
			))
		})?;
		let is_native_json = row.native_json_fields().contains(&database_column);
		let hydrated_value = hydrate_fixture_json_value::<M>(
			value.clone(),
			Some(field_info),
			&field_name,
			&database_column,
			is_native_json,
		)?;
		if hydrated_value.is_null()
			&& (row.json_null_fields().contains(&database_column)
				|| (!is_native_json && value.is_string()))
		{
			json_null_fields.insert(field_name);
		}
	}
	Ok(json_null_fields)
}

#[cfg(test)]
fn fixture_fields_from_database_row<M>(
	row: &Map<String, Value>,
) -> FixtureResult<Map<String, Value>>
where
	M: Model,
{
	fixture_fields_from_database_row_with_json_provenance::<M>(row, &HashSet::new())
}

fn fixture_fields_from_database_row_with_json_provenance<M>(
	row: &Map<String, Value>,
	native_json_fields: &HashSet<String>,
) -> FixtureResult<Map<String, Value>>
where
	M: Model,
{
	let field_metadata = M::field_metadata();
	let mut fixture_fields = Map::new();
	for (field_name, database_column) in fixture_database_fields::<M>()? {
		let value = row.get(&database_column).cloned().ok_or_else(|| {
			FixtureError::Database(format!(
				"fixture dump row for '{}.{}' is missing database column '{}'",
				M::app_label(),
				rust_model_name::<M>(),
				database_column
			))
		})?;
		let field_info = field_metadata
			.iter()
			.find(|field| field.name == field_name || field.db_column_name() == database_column);
		let value = hydrate_fixture_json_value::<M>(
			value,
			field_info,
			&field_name,
			&database_column,
			native_json_fields.contains(&database_column),
		)?;
		if fixture_fields.insert(field_name.clone(), value).is_some() {
			return Err(FixtureError::Database(format!(
				"fixture fields for '{}.{}' map multiple database columns to '{}'",
				M::app_label(),
				rust_model_name::<M>(),
				field_name
			)));
		}
	}
	Ok(fixture_fields)
}

fn hydrate_fixture_json_value<M>(
	value: Value,
	field_info: Option<&crate::orm::inspection::FieldInfo>,
	field_name: &str,
	database_column: &str,
	is_native_json: bool,
) -> FixtureResult<Value>
where
	M: Model,
{
	if is_native_json
		|| !field_info.is_some_and(|field| crate::orm::json::is_json_field_type(&field.field_type))
	{
		return Ok(value);
	}

	match value {
		Value::String(text) => serde_json::from_str(&text).map_err(|error| {
			FixtureError::Database(format!(
				"failed to hydrate JSON fixture field '{}.{}' from database column '{}': {error}",
				M::app_label(),
				field_name,
				database_column
			))
		}),
		value => Ok(value),
	}
}

fn fixture_primary_key_column<M>() -> String
where
	M: Model,
{
	let primary_key = M::primary_key_field();
	M::field_metadata()
		.into_iter()
		.find(|field| field.name == primary_key)
		.map(|field| field.db_column.unwrap_or(field.name))
		.unwrap_or_else(|| primary_key.to_string())
}

fn fixture_writable_columns(
	object: &Map<String, Value>,
	primary_key: &str,
) -> FixtureResult<Vec<String>> {
	let mut columns = object.keys().cloned().collect::<Vec<_>>();
	if columns.is_empty() {
		return Err(FixtureError::Database(
			"fixture record must contain at least one writable database column".to_string(),
		));
	}
	columns.sort();
	if let Some(primary_key_index) = columns.iter().position(|column| column == primary_key) {
		columns.swap(0, primary_key_index);
	}
	Ok(columns)
}

fn fixture_primary_key_is_identity_always<M>(object: &Map<String, Value>, primary_key: &str) -> bool
where
	M: Model,
{
	object.contains_key(primary_key)
		&& M::field_metadata().into_iter().any(|field| {
			field.primary_key
				&& field.db_column_name() == primary_key
				&& matches!(
					field.attributes.get("identity_always"),
					Some(crate::orm::fields::FieldKwarg::Bool(true))
				)
		})
}

fn fixture_field_is_none(
	value: &Value,
	field_info: Option<&crate::orm::inspection::FieldInfo>,
	json_null_fields: &BTreeSet<String>,
) -> bool {
	field_info.is_some_and(|field| {
		field.nullable
			&& value.is_null()
			&& !(crate::orm::json::is_json_field_type(&field.field_type)
				&& json_null_fields.contains(&field.name))
	})
}

fn fixture_value_to_sea_value_for_field<M>(
	value: &Value,
	field_info: Option<&crate::orm::inspection::FieldInfo>,
	field_is_none: bool,
) -> reinhardt_query::value::Value
where
	M: Model,
{
	if field_info.is_some_and(|field| is_fixture_text_field_type(&field.field_type))
		&& let Value::String(value) = value
	{
		return reinhardt_query::value::Value::String(Some(Box::new(value.clone())));
	}
	Manager::<M>::json_to_sea_value_for_field(value, field_info, field_is_none)
}

fn is_fixture_text_field_type(field_type: &str) -> bool {
	[
		"CharField",
		"TextField",
		"EmailField",
		"URLField",
		"SlugField",
		"FilePathField",
		"GenericIPAddressField",
	]
	.iter()
	.any(|field_name| field_type.ends_with(field_name))
}

#[cfg(test)]
fn build_fixture_upsert_sql_values<M>(
	backend: DatabaseBackend,
	object: &Map<String, Value>,
) -> FixtureResult<(String, Vec<QueryValue>)>
where
	M: Model,
{
	build_fixture_upsert_sql_values_with_json_null_fields::<M>(backend, object, &BTreeSet::new())
}

fn build_fixture_upsert_sql_values_with_json_null_fields<M>(
	backend: DatabaseBackend,
	object: &Map<String, Value>,
	json_null_fields: &BTreeSet<String>,
) -> FixtureResult<(String, Vec<QueryValue>)>
where
	M: Model,
{
	ensure_single_column_primary_key::<M>()?;
	let object = fixture_database_object::<M>(object)?;
	let pk_field = fixture_primary_key_column::<M>();
	let columns = fixture_writable_columns(&object, &pk_field)?;

	let mut stmt = Query::insert();
	stmt.into_table(Alias::new(M::table_name()));
	stmt.columns(columns.iter().map(|column| Alias::new(column.as_str())));
	let field_metadata = M::field_metadata();
	stmt.values_panic(
		columns
			.iter()
			.map(|column| {
				let field_info = field_metadata.iter().find(|field| {
					field.name == *column || field.db_column.as_deref() == Some(column.as_str())
				});
				let field_is_none =
					fixture_field_is_none(&object[column], field_info, json_null_fields);
				fixture_value_to_sea_value_for_field::<M>(
					&object[column],
					field_info,
					field_is_none,
				)
			})
			.collect::<Vec<_>>(),
	);
	if backend == DatabaseBackend::Postgres
		&& fixture_primary_key_is_identity_always::<M>(&object, &pk_field)
	{
		stmt.overriding_system_value();
	}

	if object.contains_key(&pk_field) && backend != DatabaseBackend::MySql {
		let update_columns = columns
			.iter()
			.filter(|column| column.as_str() != pk_field.as_str())
			.map(|column| Alias::new(column.as_str()))
			.collect::<Vec<_>>();
		let conflict = if update_columns.is_empty() {
			OnConflict::column(Alias::new(pk_field.as_str())).do_nothing()
		} else {
			OnConflict::column(Alias::new(pk_field.as_str())).update_columns(update_columns)
		};
		stmt.on_conflict(conflict.to_owned());
	}

	let (sql, values) = build_insert_sql(&stmt, backend);
	Ok((sql, super::execution::convert_values(values)))
}

#[cfg(test)]
fn build_fixture_primary_key_lookup_sql_values<M>(
	backend: DatabaseBackend,
	object: &Map<String, Value>,
) -> FixtureResult<(String, Vec<QueryValue>)>
where
	M: Model,
{
	build_fixture_primary_key_lookup_sql_values_with_json_null_fields::<M>(
		backend,
		object,
		&BTreeSet::new(),
	)
}

fn build_fixture_primary_key_lookup_sql_values_with_json_null_fields<M>(
	backend: DatabaseBackend,
	object: &Map<String, Value>,
	json_null_fields: &BTreeSet<String>,
) -> FixtureResult<(String, Vec<QueryValue>)>
where
	M: Model,
{
	ensure_single_column_primary_key::<M>()?;
	let object = fixture_database_object::<M>(object)?;
	let primary_key = fixture_primary_key_column::<M>();
	let primary_key_value = object.get(&primary_key).ok_or_else(|| {
		FixtureError::Database(format!(
			"fixture record for '{}.{}' must include primary key '{}'",
			M::app_label(),
			rust_model_name::<M>(),
			primary_key
		))
	})?;
	let field_metadata = M::field_metadata();
	let field_info = field_metadata.iter().find(|field| {
		field.name == primary_key || field.db_column.as_deref() == Some(primary_key.as_str())
	});
	let field_is_none = fixture_field_is_none(primary_key_value, field_info, json_null_fields);
	let primary_key_value =
		fixture_value_to_sea_value_for_field::<M>(primary_key_value, field_info, field_is_none);

	let mut stmt = Query::select();
	stmt.column(Alias::new(primary_key.as_str()))
		.from(Alias::new(M::table_name()))
		.and_where(Expr::col(Alias::new(primary_key.as_str())).eq(primary_key_value));
	let (sql, values) = build_select_sql(&stmt, backend);
	Ok((sql, super::execution::convert_values(values)))
}

#[cfg(test)]
fn build_fixture_update_sql_values<M>(
	backend: DatabaseBackend,
	object: &Map<String, Value>,
) -> FixtureResult<Option<(String, Vec<QueryValue>)>>
where
	M: Model,
{
	build_fixture_update_sql_values_with_json_null_fields::<M>(backend, object, &BTreeSet::new())
}

fn build_fixture_update_sql_values_with_json_null_fields<M>(
	backend: DatabaseBackend,
	object: &Map<String, Value>,
	json_null_fields: &BTreeSet<String>,
) -> FixtureResult<Option<(String, Vec<QueryValue>)>>
where
	M: Model,
{
	ensure_single_column_primary_key::<M>()?;
	let object = fixture_database_object::<M>(object)?;
	let primary_key = fixture_primary_key_column::<M>();
	let primary_key_value = object.get(&primary_key).ok_or_else(|| {
		FixtureError::Database(format!(
			"fixture record for '{}.{}' must include primary key '{}'",
			M::app_label(),
			rust_model_name::<M>(),
			primary_key
		))
	})?;
	let update_columns = fixture_writable_columns(&object, &primary_key)?
		.into_iter()
		.filter(|column| column != &primary_key)
		.collect::<Vec<_>>();
	if update_columns.is_empty() {
		return Ok(None);
	}

	let field_metadata = M::field_metadata();
	let mut stmt = Query::update();
	stmt.table(Alias::new(M::table_name()));
	stmt.values(update_columns.iter().map(|column| {
		let field_info = field_metadata.iter().find(|field| {
			field.name == *column || field.db_column.as_deref() == Some(column.as_str())
		});
		let field_is_none = fixture_field_is_none(&object[column], field_info, json_null_fields);
		(
			Alias::new(column.as_str()),
			fixture_value_to_sea_value_for_field::<M>(&object[column], field_info, field_is_none),
		)
	}));
	let primary_key_field = field_metadata.iter().find(|field| {
		field.name == primary_key || field.db_column.as_deref() == Some(primary_key.as_str())
	});
	let primary_key_is_none =
		fixture_field_is_none(primary_key_value, primary_key_field, json_null_fields);
	stmt.and_where(Expr::col(Alias::new(primary_key.as_str())).eq(
		fixture_value_to_sea_value_for_field::<M>(
			primary_key_value,
			primary_key_field,
			primary_key_is_none,
		),
	));
	let (sql, values) = build_update_sql(&stmt, backend);
	Ok(Some((sql, super::execution::convert_values(values))))
}

fn ensure_single_column_primary_key<M>() -> FixtureResult<()>
where
	M: Model,
{
	if let Some(primary_key) = M::composite_primary_key() {
		return Err(FixtureError::Database(format!(
			"fixture loading does not support composite primary keys for '{}.{}' ({})",
			M::app_label(),
			rust_model_name::<M>(),
			primary_key.fields().join(", ")
		)));
	}
	Ok(())
}

#[cfg(feature = "migrations")]
fn normalize_foreign_key_fixture_fields<M>(object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	let Some(metadata) = metadata_for_model::<M>() else {
		return Ok(());
	};
	for (field_name, relation_name) in foreign_key_fixture_field_names::<M>(&metadata) {
		if object.contains_key(&field_name) {
			continue;
		}
		if let Some(value) = object.remove(&relation_name) {
			object.insert(field_name, value);
		}
	}
	Ok(())
}

#[cfg(feature = "migrations")]
fn denormalize_foreign_key_fixture_fields<M>(object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	let Some(metadata) = metadata_for_model::<M>() else {
		return Ok(());
	};
	let mut renames = Vec::new();
	for (field_name, relation_name) in foreign_key_fixture_field_names::<M>(&metadata) {
		if !object.contains_key(&field_name) || object.contains_key(&relation_name) {
			continue;
		}
		renames.push((field_name, relation_name));
	}
	for (field_name, relation_name) in renames {
		if let Some(value) = object.remove(&field_name) {
			object.insert(relation_name, value);
		}
	}
	Ok(())
}

#[cfg(feature = "migrations")]
fn foreign_key_fixture_field_names<M>(
	metadata: &crate::migrations::model_registry::ModelMetadata,
) -> Vec<(String, String)>
where
	M: Model,
{
	let relationship_names = M::relationship_metadata()
		.into_iter()
		.filter_map(|relationship| {
			relationship
				.foreign_key
				.map(|field_name| (field_name, relationship.name))
		})
		.collect::<HashMap<_, _>>();

	metadata
		.fields
		.keys()
		.filter(|field_name| is_foreign_key_field(metadata, field_name))
		.filter_map(|field_name| {
			let relation_name = relationship_names
				.get(field_name)
				.cloned()
				.or_else(|| field_name.strip_suffix("_id").map(str::to_string))?;
			Some((field_name.clone(), relation_name))
		})
		.collect()
}

#[cfg(not(feature = "migrations"))]
fn denormalize_foreign_key_fixture_fields<M>(_object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	Ok(())
}

#[cfg(not(feature = "migrations"))]
fn normalize_foreign_key_fixture_fields<M>(_object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	Ok(())
}

#[cfg(feature = "migrations")]
fn extract_many_to_many_assignments<M>(
	object: &mut Map<String, Value>,
) -> FixtureResult<Vec<FixtureManyToManyAssignment>>
where
	M: Model,
{
	let mut assignments = Vec::new();
	for spec in many_to_many_specs_for::<M>() {
		let Some(raw_value) = object.remove(&spec.field_name) else {
			continue;
		};
		if spec.is_explicit_through {
			continue;
		}
		let values = raw_value.as_array().cloned().ok_or_else(|| {
			FixtureError::Database(format!(
				"many-to-many fixture field '{}' must be an array",
				spec.field_name
			))
		})?;
		assignments.push(FixtureManyToManyAssignment { spec, values });
	}
	Ok(assignments)
}

#[cfg(feature = "migrations")]
fn remove_many_to_many_fixture_fields<M>(object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	let _ = extract_many_to_many_assignments::<M>(object)?;
	Ok(())
}

#[cfg(not(feature = "migrations"))]
fn remove_many_to_many_fixture_fields<M>(_object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	Ok(())
}

#[cfg(not(feature = "migrations"))]
fn extract_many_to_many_assignments<M>(
	_object: &mut Map<String, Value>,
) -> FixtureResult<Vec<FixtureManyToManyAssignment>>
where
	M: Model,
{
	Ok(Vec::new())
}

async fn load_many_to_many_assignments<M>(
	tx: &mut TransactionScope,
	conn: &DatabaseConnection,
	source_pk: Option<&Value>,
	assignments: &[FixtureManyToManyAssignment],
) -> FixtureResult<()>
where
	M: Model,
{
	if assignments.is_empty() {
		return Ok(());
	}
	let source_pk = source_pk.ok_or_else(|| {
		FixtureError::Database(format!(
			"fixture record for '{}.{}' must include pk to load many-to-many fields",
			M::app_label(),
			rust_model_name::<M>()
		))
	})?;

	for assignment in assignments {
		let mut delete = Query::delete();
		delete
			.from_table(Alias::new(assignment.spec.through_table.as_str()))
			.and_where(
				Expr::col(Alias::new(assignment.spec.source_field.as_str()))
					.eq(Manager::<M>::json_to_sea_value(source_pk)),
			);
		let (sql, values) = build_delete_sql(&delete, conn.backend());
		tx.execute(&sql, super::execution::convert_values(values))
			.await?;

		for target_pk in &assignment.values {
			let mut insert = Query::insert();
			insert
				.into_table(Alias::new(assignment.spec.through_table.as_str()))
				.columns([
					Alias::new(assignment.spec.source_field.as_str()),
					Alias::new(assignment.spec.target_field.as_str()),
				])
				.values_panic([
					Manager::<M>::json_to_sea_value(source_pk),
					Manager::<M>::json_to_sea_value(target_pk),
				])
				.on_conflict(
					OnConflict::columns([
						Alias::new(assignment.spec.source_field.as_str()),
						Alias::new(assignment.spec.target_field.as_str()),
					])
					.do_nothing()
					.to_owned(),
				);
			let (sql, values) = build_insert_sql(&insert, conn.backend());
			tx.execute(&sql, super::execution::convert_values(values))
				.await?;
		}
	}
	Ok(())
}

async fn append_many_to_many_fixture_fields<M>(
	conn: &DatabaseConnection,
	pk: Option<&Value>,
	object: &mut Map<String, Value>,
) -> FixtureResult<()>
where
	M: Model,
{
	let Some(pk) = pk else {
		return Ok(());
	};
	for spec in many_to_many_specs_for::<M>() {
		if spec.is_explicit_through {
			continue;
		}
		let mut select = Query::select();
		select
			.column(Alias::new(spec.target_field.as_str()))
			.from(Alias::new(spec.through_table.as_str()))
			.and_where(
				Expr::col(Alias::new(spec.source_field.as_str()))
					.eq(Manager::<M>::json_to_sea_value(pk)),
			)
			.order_by(
				Alias::new(spec.target_field.as_str()),
				reinhardt_query::prelude::Order::Asc,
			);
		let (sql, values) = build_select_sql(&select, conn.backend());
		let rows = conn
			.query(&sql, super::execution::convert_values(values))
			.await?;
		let values = rows
			.into_iter()
			.filter_map(|row| row.data.get(&spec.target_field).cloned())
			.collect::<Vec<_>>();
		object.insert(spec.field_name, Value::Array(values));
	}
	Ok(())
}

#[cfg(feature = "migrations")]
fn many_to_many_specs_for<M>() -> Vec<FixtureManyToManySpec>
where
	M: Model,
{
	let Some(metadata) = metadata_for_model::<M>() else {
		return Vec::new();
	};
	metadata
		.many_to_many_fields
		.iter()
		.map(|field| {
			let target_table = related_model_metadata(&metadata.app_label, &field.to_model)
				.map(|target| target.table_name)
				.unwrap_or_else(|| default_target_table_name(&field.to_model));
			let is_explicit_through = field.through.is_some();
			let through_table = field.through.clone().unwrap_or_else(|| {
				crate::m2m_naming::default_through_table(M::table_name(), &field.field_name)
			});
			let (default_source_field, default_target_field) =
				crate::m2m_naming::default_m2m_columns(M::table_name(), &target_table);
			FixtureManyToManySpec {
				field_name: field.field_name.clone(),
				through_table,
				source_field: field.source_field.clone().unwrap_or(default_source_field),
				target_field: field.target_field.clone().unwrap_or(default_target_field),
				is_explicit_through,
			}
		})
		.collect()
}

#[cfg(not(feature = "migrations"))]
fn many_to_many_specs_for<M>() -> Vec<FixtureManyToManySpec>
where
	M: Model,
{
	Vec::new()
}

async fn reset_sequences_after_explicit_pks(
	conn: &DatabaseConnection,
	tx: &mut TransactionScope,
	records: &[FixtureRecord],
) -> FixtureResult<()> {
	if conn.backend() != DatabaseBackend::Postgres {
		return Ok(());
	}

	let mut handlers = Vec::<Arc<dyn FixtureModelHandler>>::new();
	let mut seen = HashSet::new();
	for record in records {
		let Some(pk) = &record.pk else {
			continue;
		};
		if pk.as_i64().is_none() && pk.as_u64().is_none() {
			continue;
		}
		let handler = global_fixture_registry()
			.get(&record.model)
			.ok_or_else(|| FixtureError::ModelNotRegistered(record.model.clone()))?;
		let label = handler.label();
		if seen.insert(label) {
			handlers.push(handler);
		}
	}

	for handler in handlers {
		let primary_key_column = handler.primary_key_database_column();
		let sql = build_postgres_sequence_reset_sql(handler.table_name(), &primary_key_column);
		tx.query_optional(
			&sql,
			vec![
				QueryValue::String(handler.table_name().to_string()),
				QueryValue::String(primary_key_column),
			],
		)
		.await?;
	}
	Ok(())
}

fn build_postgres_sequence_reset_sql(table_name: &str, primary_key_column: &str) -> String {
	let table = quote_identifier_path(table_name);
	let primary_key_column = quote_identifier(primary_key_column);
	format!(
		"SELECT CASE \
		 WHEN pg_get_serial_sequence($1, $2) IS NULL THEN NULL \
		 ELSE setval(pg_get_serial_sequence($1, $2), \
		 COALESCE((SELECT MAX({primary_key_column}) FROM {table}), 1), \
		 (SELECT MAX({primary_key_column}) FROM {table}) IS NOT NULL) \
		 END",
	)
}

fn quote_identifier(identifier: &str) -> String {
	format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn quote_identifier_path(path: &str) -> String {
	path.split('.')
		.map(quote_identifier)
		.collect::<Vec<_>>()
		.join(".")
}

fn rust_model_name<M>() -> &'static str {
	std::any::type_name::<M>()
		.rsplit("::")
		.next()
		.unwrap_or(std::any::type_name::<M>())
}

#[cfg(feature = "migrations")]
fn metadata_for_model<M>() -> Option<crate::migrations::model_registry::ModelMetadata>
where
	M: Model,
{
	find_model_metadata(M::app_label(), rust_model_name::<M>())
}

#[cfg(feature = "migrations")]
fn find_model_metadata(
	app_label: &str,
	model_name: &str,
) -> Option<crate::migrations::model_registry::ModelMetadata> {
	let registry = crate::migrations::model_registry::global_registry();
	registry
		.find_model_qualified(app_label, model_name)
		.or_else(|| {
			registry.get_models().into_iter().find(|metadata| {
				metadata.app_label.eq_ignore_ascii_case(app_label)
					&& metadata.model_name.eq_ignore_ascii_case(model_name)
			})
		})
}

#[cfg(feature = "migrations")]
fn related_model_metadata(
	current_app_label: &str,
	to_model: &str,
) -> Option<crate::migrations::model_registry::ModelMetadata> {
	if let Ok((app_label, model_name)) = parse_model_label(to_model) {
		return find_model_metadata(&app_label, &model_name);
	}
	find_model_metadata(current_app_label, to_model).or_else(|| {
		let registry = crate::migrations::model_registry::global_registry();
		registry.find_model_by_name(to_model).or_else(|| {
			registry
				.get_models()
				.into_iter()
				.find(|metadata| metadata.model_name.eq_ignore_ascii_case(to_model))
		})
	})
}

#[cfg(feature = "migrations")]
fn is_foreign_key_field(
	metadata: &crate::migrations::model_registry::ModelMetadata,
	field_name: &str,
) -> bool {
	metadata
		.fields
		.get(field_name)
		.map(|field| field.params.contains_key("fk_target") || field.foreign_key.is_some())
		.unwrap_or(false)
}

#[cfg(feature = "migrations")]
fn default_target_table_name(to_model: &str) -> String {
	if let Ok((app_label, model_name)) = parse_model_label(to_model) {
		format!(
			"{}_{}",
			app_label.to_lowercase(),
			to_snake_case(&model_name)
		)
	} else {
		to_snake_case(to_model)
	}
}

#[cfg(feature = "migrations")]
fn to_snake_case(value: &str) -> String {
	let mut output = String::new();
	for (index, ch) in value.chars().enumerate() {
		if ch.is_ascii_uppercase() {
			if index > 0 {
				output.push('_');
			}
			output.push(ch.to_ascii_lowercase());
		} else {
			output.push(ch.to_ascii_lowercase());
		}
	}
	output
}

fn select_handlers(
	selectors: &[String],
	excludes: &[String],
) -> FixtureResult<Vec<Arc<dyn FixtureModelHandler>>> {
	let registry = global_fixture_registry();
	let exclude_set = excluded_handler_labels(excludes)?;
	let mut handlers = if selectors.is_empty() {
		registry.all()
	} else {
		let mut selected = Vec::new();
		for selector in selectors {
			selected.extend(handlers_for_selector(selector)?);
		}
		selected
	};

	handlers.retain(|handler| !exclude_set.contains(&handler.label()));
	handlers.sort_by_key(|handler| handler.label());
	handlers.dedup_by_key(|handler| handler.label());
	Ok(handlers)
}

fn excluded_handler_labels(excludes: &[String]) -> FixtureResult<HashSet<String>> {
	let mut labels = HashSet::new();
	for exclude in excludes {
		for handler in handlers_for_selector(exclude)? {
			labels.insert(handler.label());
		}
	}
	Ok(labels)
}

fn handlers_for_selector(selector: &str) -> FixtureResult<Vec<Arc<dyn FixtureModelHandler>>> {
	if selector.contains('.') {
		return global_fixture_registry()
			.get(selector)
			.map(|handler| vec![handler])
			.ok_or_else(|| FixtureError::SelectorMatchedNoModels(selector.to_string()));
	}

	let mut handlers: Vec<_> = global_fixture_registry()
		.all()
		.into_iter()
		.filter(|handler| handler.app_label().eq_ignore_ascii_case(selector))
		.collect();
	handlers.sort_by_key(|handler| handler.label());
	if handlers.is_empty() {
		return Err(FixtureError::SelectorMatchedNoModels(selector.to_string()));
	}
	Ok(handlers)
}

#[cfg(feature = "migrations")]
fn order_records_by_dependencies(records: &[FixtureRecord]) -> FixtureResult<Vec<FixtureRecord>> {
	let dependencies = fixture_record_dependencies(records)?;
	let order = topological_record_order(records, dependencies)?;
	Ok(order
		.into_iter()
		.map(|index| records[index].clone())
		.collect())
}

#[cfg(not(feature = "migrations"))]
fn order_records_by_dependencies(records: &[FixtureRecord]) -> FixtureResult<Vec<FixtureRecord>> {
	for record in records {
		parse_model_label(&record.model)?;
	}
	Ok(records.to_vec())
}

#[cfg(feature = "migrations")]
fn canonical_model_key(app_label: &str, model_name: &str) -> String {
	let label = canonical_label(app_label, model_name);
	if let Some(metadata) = find_model_metadata(app_label, model_name) {
		return canonical_label(&metadata.app_label, &metadata.model_name);
	}
	canonical_record_label(&label).unwrap_or(label)
}

#[cfg(feature = "migrations")]
fn fixture_record_dependencies(
	records: &[FixtureRecord],
) -> FixtureResult<HashMap<usize, HashSet<usize>>> {
	let mut record_indices = HashMap::new();
	for (index, record) in records.iter().enumerate() {
		let Some(pk_key) = record.pk.as_ref().and_then(json_dependency_key) else {
			continue;
		};
		let model_key = canonical_record_label(&record.model)?;
		record_indices.entry((model_key, pk_key)).or_insert(index);
	}

	let mut dependencies = (0..records.len())
		.map(|index| (index, HashSet::new()))
		.collect::<HashMap<_, _>>();

	for (source_index, record) in records.iter().enumerate() {
		let source_key = canonical_record_label(&record.model)?;
		let (app_label, model_name) = parse_model_label(&source_key)?;
		let Some(metadata) = find_model_metadata(&app_label, &model_name) else {
			continue;
		};
		let relationship_names = global_fixture_registry()
			.get(&record.model)
			.map(|handler| {
				handler
					.fixture_foreign_key_fields()
					.into_iter()
					.collect::<HashMap<_, _>>()
			})
			.unwrap_or_default();
		for (field_name, field) in &metadata.fields {
			let Some(target_key) = fixture_foreign_key_target_key(&metadata, field) else {
				continue;
			};
			let is_primary_key_foreign_key = field
				.params
				.get("primary_key")
				.is_some_and(|value| value == "true");
			let Some(value) = fixture_field_value(
				record,
				field_name,
				relationship_names.get(field_name).map(String::as_str),
				is_primary_key_foreign_key,
			) else {
				continue;
			};
			let Some(target_pk) = json_dependency_key(value) else {
				continue;
			};
			let Some(target_index) = record_indices.get(&(target_key, target_pk)).copied() else {
				continue;
			};
			if target_index != source_index {
				dependencies
					.entry(source_index)
					.or_default()
					.insert(target_index);
			}
		}
	}

	Ok(dependencies)
}

#[cfg(feature = "migrations")]
fn fixture_foreign_key_target_key(
	source_metadata: &crate::migrations::model_registry::ModelMetadata,
	field: &crate::migrations::model_registry::FieldMetadata,
) -> Option<String> {
	if let Some(target_model) = field.params.get("fk_target") {
		let target_app = field
			.params
			.get("fk_target_app")
			.map(String::as_str)
			.unwrap_or(&source_metadata.app_label);
		return Some(canonical_model_key(target_app, target_model));
	}

	let referenced_table = &field.foreign_key.as_ref()?.referenced_table;
	crate::migrations::model_registry::global_registry()
		.get_models()
		.into_iter()
		.find(|metadata| metadata.table_name.eq_ignore_ascii_case(referenced_table))
		.map(|metadata| canonical_label(&metadata.app_label, &metadata.model_name))
}

#[cfg(feature = "migrations")]
fn fixture_field_value<'a>(
	record: &'a FixtureRecord,
	field_name: &str,
	relation_name: Option<&str>,
	is_primary_key_foreign_key: bool,
) -> Option<&'a Value> {
	record
		.fields
		.get(field_name)
		.or_else(|| relation_name.and_then(|name| record.fields.get(name)))
		.or_else(|| {
			field_name
				.strip_suffix("_id")
				.and_then(|name| record.fields.get(name))
		})
		.or({
			if is_primary_key_foreign_key {
				record.pk.as_ref()
			} else {
				None
			}
		})
}

#[cfg(feature = "migrations")]
fn json_dependency_key(value: &Value) -> Option<String> {
	if value.is_null() {
		None
	} else {
		Some(value.to_string())
	}
}

#[cfg(feature = "migrations")]
fn topological_record_order(
	records: &[FixtureRecord],
	mut dependencies: HashMap<usize, HashSet<usize>>,
) -> FixtureResult<Vec<usize>> {
	let mut remaining = (0..records.len()).collect::<HashSet<_>>();
	let mut ordered = Vec::with_capacity(records.len());

	while !remaining.is_empty() {
		let next = remaining
			.iter()
			.filter(|index| {
				dependencies
					.get(index)
					.map(|deps| deps.is_disjoint(&remaining))
					.unwrap_or(true)
			})
			.copied()
			.min();

		let Some(next) = next else {
			let mut cycle = remaining
				.iter()
				.map(|index| canonical_record_label(&records[*index].model))
				.collect::<FixtureResult<Vec<_>>>()?;
			cycle.sort();
			cycle.dedup();
			return Err(FixtureError::DependencyCycle(cycle.join(", ")));
		};

		remaining.remove(&next);
		dependencies.remove(&next);
		ordered.push(next);
	}

	Ok(ordered)
}

#[cfg(feature = "migrations")]
fn canonical_record_label(label: &str) -> FixtureResult<String> {
	let (app_label, model_name) = parse_model_label(label)?;
	if let Some(handler) = global_fixture_registry().get(label) {
		return Ok(handler.label());
	}
	if let Some(metadata) = find_model_metadata(&app_label, &model_name) {
		return Ok(canonical_label(&metadata.app_label, &metadata.model_name));
	}
	Ok(canonical_label(&app_label, &model_name))
}

fn parse_model_label(label: &str) -> FixtureResult<(String, String)> {
	let mut parts = label.split('.');
	let Some(app_label) = parts.next() else {
		return Err(FixtureError::InvalidModelLabel(label.to_string()));
	};
	let Some(model_name) = parts.next() else {
		return Err(FixtureError::InvalidModelLabel(label.to_string()));
	};
	if parts.next().is_some() || app_label.is_empty() || model_name.is_empty() {
		return Err(FixtureError::InvalidModelLabel(label.to_string()));
	}
	Ok((app_label.to_string(), model_name.to_string()))
}

fn canonical_label(app_label: &str, model_name: &str) -> String {
	format!("{}.{}", app_label, model_name)
}

fn labels_match(expected: &str, actual: &str) -> bool {
	if expected == actual {
		return true;
	}
	let Ok((expected_app, expected_model)) = parse_model_label(expected) else {
		return false;
	};
	let Ok((actual_app, actual_model)) = parse_model_label(actual) else {
		return false;
	};
	expected_app.eq_ignore_ascii_case(&actual_app)
		&& expected_model.eq_ignore_ascii_case(&actual_model)
}

fn find_case_insensitive(
	models: &HashMap<String, Arc<dyn FixtureModelHandler>>,
	app_label: &str,
	model_name: &str,
) -> Option<Arc<dyn FixtureModelHandler>> {
	models
		.values()
		.find(|handler| {
			handler.app_label().eq_ignore_ascii_case(app_label)
				&& handler.model_name().eq_ignore_ascii_case(model_name)
		})
		.cloned()
}

#[cfg(test)]
mod tests {
	use super::*;
	#[cfg(feature = "migrations")]
	use crate::orm::model::FieldSelector;

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixturePost {
		id: Option<i64>,
		author_id: Option<i64>,
		parent_id: Option<i64>,
		title: String,
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone)]
	struct FixturePostFields;

	#[cfg(feature = "migrations")]
	impl FieldSelector for FixturePostFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	#[cfg(feature = "migrations")]
	impl Model for FixturePost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_tests"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureM2mPost {
		id: Option<i64>,
		title: String,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureM2mPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_m2m_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_m2m"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureFieldAwarePost {
		id: Option<i64>,
		writer_id: Option<i64>,
		payload: Value,
		generated_value: i64,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureFieldAwarePost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_field_aware_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_field_aware"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn validate_fixture_fields(fields: &crate::orm::FixtureFields) -> Result<(), String> {
			// The projection is deserialized only to validate fixture input.
			#[allow(dead_code)]
			#[derive(Deserialize)]
			struct FixtureProjection {
				id: Option<i64>,
				#[serde(default)]
				writer_id: Option<i64>,
				payload: Value,
			}

			let FixtureProjection {
				id: _,
				writer_id: _,
				payload: _,
			} = __deserialize_fixture_projection::<FixtureProjection>(fields)?;
			Ok(())
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			vec![fixture_field_info("payload", "JsonField", false)]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"author",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"Author",
				)
				.with_foreign_key("writer_id"),
			]
		}

		fn generated_field_names() -> &'static [&'static str] {
			&["generated_value"]
		}
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureIdentityAlwaysPost {
		id: Option<i64>,
		payload: String,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureIdentityAlwaysPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_identity_always_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_identity_always"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "BigIntegerField", false);
			id.primary_key = true;
			id.attributes.insert(
				"identity_always".to_string(),
				crate::orm::fields::FieldKwarg::Bool(true),
			);
			vec![id, fixture_field_info("payload", "CharField", false)]
		}
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureDatabaseColumnPost {
		id: Option<i64>,
		title: String,
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureSerdeMappedPost {
		id: Option<i64>,
		#[serde(rename = "displayName")]
		title: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		note: Option<String>,
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureCustomForeignKeyPost {
		id: Option<i64>,
		author_id: i64,
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureJsonPost {
		id: Option<i64>,
		payload: Value,
		optional_payload: Option<Value>,
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureTextPost {
		id: Option<i64>,
		uuid_like: String,
		timestamp_like: String,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureDatabaseColumnPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_database_column_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_database_column"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "BigIntegerField", false);
			id.primary_key = true;
			id.db_column = Some("fixture_id".to_string());
			let mut title = fixture_field_info("title", "CharField", false);
			title.db_column = Some("fixture_title".to_string());
			vec![id, title]
		}
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureSerdeMappedPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_serde_mapped_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_serde_mapped"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "BigIntegerField", false);
			id.primary_key = true;
			id.db_column = Some("fixture_id".to_string());
			let mut title = fixture_field_info("title", "CharField", false);
			title.db_column = Some("fixture_title".to_string());
			let mut note = fixture_field_info("note", "CharField", true);
			note.db_column = Some("fixture_note".to_string());
			vec![id, title, note]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"owner",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"Owner",
				)
				.with_foreign_key("fixture_owner_id"),
			]
		}
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureCustomForeignKeyPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_custom_foreign_key_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_custom_foreign_key"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "BigIntegerField", false);
			id.primary_key = true;
			id.db_column = Some("fixture_id".to_string());
			let mut author_id = fixture_field_info("author_id", "BigIntegerField", false);
			author_id.db_column = Some("author_fk".to_string());
			vec![id, author_id]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"author",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"Author",
				)
				.with_foreign_key("author_id"),
			]
		}
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureJsonPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_json_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_json"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "BigIntegerField", false);
			id.primary_key = true;
			vec![
				id,
				fixture_field_info("payload", "JsonField", false),
				fixture_field_info("optional_payload", "JsonField", true),
			]
		}
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureTextPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_text_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_text"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "BigIntegerField", false);
			id.primary_key = true;
			vec![
				id,
				fixture_field_info("uuid_like", "CharField", false),
				fixture_field_info("timestamp_like", "TextField", false),
			]
		}
	}

	#[cfg(feature = "migrations")]
	fn fixture_field_info(
		name: &str,
		field_type: &str,
		nullable: bool,
	) -> crate::orm::inspection::FieldInfo {
		crate::orm::inspection::FieldInfo {
			name: name.to_string(),
			field_type: field_type.to_string(),
			nullable,
			primary_key: false,
			unique: false,
			blank: false,
			editable: true,
			default: None,
			db_default: None,
			db_column: None,
			choices: None,
			attributes: std::collections::HashMap::new(),
		}
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct CompositeFixture {
		left_id: i64,
		right_id: i64,
		label: String,
	}

	#[cfg(feature = "migrations")]
	impl Model for CompositeFixture {
		type PrimaryKey = String;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_composite"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_composite"
		}

		fn primary_key_field() -> &'static str {
			"left_id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(format!("{}:{}", self.left_id, self.right_id))
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			let mut parts = value.split(':');
			self.left_id = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
			self.right_id = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
		}

		fn composite_primary_key() -> Option<crate::orm::composite_pk::CompositePrimaryKey> {
			crate::orm::composite_pk::CompositePrimaryKey::new(vec![
				"left_id".to_string(),
				"right_id".to_string(),
			])
			.ok()
		}
	}

	#[test]
	fn fixture_record_uses_django_shape() {
		let json = r#"[{"model":"blog.Post","pk":1,"fields":{"title":"Hello"}}]"#;

		let records: Vec<FixtureRecord> = serde_json::from_str(json).unwrap();

		assert_eq!(records.len(), 1);
		assert_eq!(records[0].model, "blog.Post");
		assert_eq!(records[0].pk, Some(Value::from(1)));
		assert_eq!(records[0].fields["title"], Value::from("Hello"));
		assert!(records[0].json_null_fields.is_empty());
	}

	#[test]
	fn invalid_model_label_is_rejected() {
		let error = parse_model_label("blog").unwrap_err();

		assert!(matches!(error, FixtureError::InvalidModelLabel(_)));
	}

	#[test]
	fn labels_match_app_and_model_case_insensitively() {
		assert!(labels_match("blog.Post", "Blog.post"));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn dependency_order_places_fk_targets_first() {
		let mut author = crate::migrations::ModelMetadata::new("blog", "Author", "blog_author");
		author.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger),
		);
		let mut post = crate::migrations::ModelMetadata::new("blog", "Post", "blog_post");
		post.add_field(
			"author_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "Author")
				.with_param("fk_target_app", "blog"),
		);
		crate::migrations::model_registry::global_registry().register_model(author);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut post_fields = Map::new();
		post_fields.insert("author".to_string(), Value::from(1));
		let records = vec![
			FixtureRecord::new("blog.Post", Some(Value::from(1)), post_fields),
			FixtureRecord::new("blog.Author", Some(Value::from(1)), Map::new()),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "blog.Author");
		assert_eq!(ordered[1].model, "blog.Post");
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn dependency_order_places_scalar_field_fk_targets_first() {
		let mut author = crate::migrations::ModelMetadata::new(
			"fixture_scalar",
			"Author",
			"fixture_scalar_author",
		);
		author.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger),
		);
		let mut post =
			crate::migrations::ModelMetadata::new("fixture_scalar", "Post", "fixture_scalar_post");
		post.add_field(
			"author_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_foreign_key(crate::migrations::ForeignKeyInfo {
					referenced_table: "fixture_scalar_author".to_string(),
					referenced_column: "id".to_string(),
					on_delete: crate::migrations::ForeignKeyAction::Cascade,
					on_update: crate::migrations::ForeignKeyAction::Cascade,
				}),
		);
		crate::migrations::model_registry::global_registry().register_model(author);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut post_fields = Map::new();
		post_fields.insert("author".to_string(), Value::from(1));
		let records = vec![
			FixtureRecord::new("fixture_scalar.Post", Some(Value::from(1)), post_fields),
			FixtureRecord::new("fixture_scalar.Author", Some(Value::from(1)), Map::new()),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_scalar.Author");
		assert_eq!(ordered[1].model, "fixture_scalar.Post");
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn foreign_key_fixture_fields_accept_django_relation_names() {
		let mut post =
			crate::migrations::ModelMetadata::new("fixture_tests", "FixturePost", "fixture_post");
		post.add_field(
			"author_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "Author")
				.with_param("fk_target_app", "fixture_tests"),
		);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut object = Map::new();
		object.insert("author".to_string(), Value::from(7));

		normalize_foreign_key_fixture_fields::<FixturePost>(&mut object).unwrap();

		assert_eq!(object.get("author_id"), Some(&Value::from(7)));
		assert!(object.get("author").is_none());
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn dumped_foreign_key_fields_use_django_relation_names() {
		let mut post =
			crate::migrations::ModelMetadata::new("fixture_tests", "FixturePost", "fixture_post");
		post.add_field(
			"author_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "Author")
				.with_param("fk_target_app", "fixture_tests"),
		);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut object = Map::new();
		object.insert("author_id".to_string(), Value::from(7));
		object.insert("title".to_string(), Value::from("Fixture"));

		denormalize_foreign_key_fixture_fields::<FixturePost>(&mut object).unwrap();

		assert_eq!(object.get("author"), Some(&Value::from(7)));
		assert_eq!(object.get("title"), Some(&Value::from("Fixture")));
		assert!(object.get("author_id").is_none());
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn custom_foreign_key_columns_use_relationship_names() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_field_aware",
			"FixtureFieldAwarePost",
			"fixture_field_aware_post",
		);
		post.add_field(
			"writer_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "Author")
				.with_param("fk_target_app", "fixture_field_aware"),
		);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut object = Map::new();
		object.insert("author".to_string(), Value::from(7));

		normalize_foreign_key_fixture_fields::<FixtureFieldAwarePost>(&mut object).unwrap();

		assert_eq!(object.get("writer_id"), Some(&Value::from(7)));
		assert_eq!(object.get("author"), None);

		denormalize_foreign_key_fixture_fields::<FixtureFieldAwarePost>(&mut object).unwrap();

		assert_eq!(object.get("author"), Some(&Value::from(7)));
		assert_eq!(object.get("writer_id"), None);
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn dependency_order_uses_relationship_names_for_custom_foreign_key_columns() {
		let mut author = crate::migrations::ModelMetadata::new(
			"fixture_field_aware",
			"Author",
			"fixture_field_aware_author",
		);
		author.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger),
		);
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_field_aware",
			"FixtureFieldAwarePost",
			"fixture_field_aware_post",
		);
		post.add_field(
			"writer_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "Author")
				.with_param("fk_target_app", "fixture_field_aware"),
		);
		crate::migrations::model_registry::global_registry().register_model(author);
		crate::migrations::model_registry::global_registry().register_model(post);
		global_fixture_registry().register_model::<FixtureFieldAwarePost>();

		let mut post_fields = Map::new();
		post_fields.insert("author".to_string(), Value::from(7));
		let records = vec![
			FixtureRecord::new(
				"fixture_field_aware.FixtureFieldAwarePost",
				Some(Value::from(1)),
				post_fields,
			),
			FixtureRecord::new(
				"fixture_field_aware.Author",
				Some(Value::from(7)),
				Map::new(),
			),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_field_aware.Author");
		assert_eq!(
			ordered[1].model,
			"fixture_field_aware.FixtureFieldAwarePost"
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_upserts_preserve_json_values_and_skip_generated_fields() {
		let payload = serde_json::json!({"theme": "paper"});
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), payload.clone());
		object.insert("generated_value".to_string(), Value::from(99));

		let (sql, values) = build_fixture_upsert_sql_values::<FixtureFieldAwarePost>(
			DatabaseBackend::Sqlite,
			&object,
		)
		.unwrap();

		assert!(!sql.contains("generated_value"));
		assert_eq!(values.len(), 2);
		assert!(matches!(
			values.get(1),
			Some(QueryValue::Json(Some(value))) if **value == payload
		));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_validation_uses_writable_projection_for_generated_fields() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), serde_json::json!({"theme": "paper"}));

		validate_fixture_writable_projection::<FixtureFieldAwarePost>(&object)
			.expect("generated fields must not be required by fixture validation");
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_validation_rejects_missing_non_generated_fields() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));

		assert!(
			validate_fixture_writable_projection::<FixtureFieldAwarePost>(&object).is_err(),
			"fixture validation must still reject missing writable fields"
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_validation_rejects_unknown_canonical_fields_before_sql() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("title".to_string(), Value::from("Fixture title"));
		object.insert("stale_field".to_string(), Value::from("unexpected"));

		let error = build_fixture_upsert_sql_values::<FixtureSerdeMappedPost>(
			DatabaseBackend::Sqlite,
			&object,
		)
		.expect_err("unknown fixture fields must fail before SQL execution");

		assert!(
			error.to_string().contains("stale_field"),
			"error must name the unknown fixture field: {error}"
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_dump_fields_ignore_serde_names_and_omissions() {
		let mut row = Map::new();
		row.insert("fixture_id".to_string(), Value::from(1));
		row.insert("fixture_title".to_string(), Value::from("Fixture title"));
		row.insert("fixture_note".to_string(), Value::Null);
		row.insert("fixture_owner_id".to_string(), Value::from(7));

		let fields = fixture_fields_from_database_row::<FixtureSerdeMappedPost>(&row).unwrap();

		assert_eq!(fields.get("id"), Some(&Value::from(1)));
		assert_eq!(fields.get("title"), Some(&Value::from("Fixture title")));
		assert_eq!(fields.get("note"), Some(&Value::Null));
		assert_eq!(fields.get("fixture_owner_id"), Some(&Value::from(7)));
		assert!(fields.get("displayName").is_none());

		let serialized = serde_json::to_value(FixtureSerdeMappedPost {
			id: Some(1),
			title: "Fixture title".to_string(),
			note: None,
		})
		.unwrap();
		assert!(serialized.get("displayName").is_some());
		assert!(serialized.get("note").is_none());
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_dump_selects_database_columns_and_forward_foreign_keys() {
		let select = build_fixture_dump_select::<FixtureSerdeMappedPost>().unwrap();
		let (sql, _) = build_select_sql(&select, DatabaseBackend::Sqlite);

		assert!(sql.contains("\"fixture_id\""));
		assert!(sql.contains("\"fixture_title\""));
		assert!(sql.contains("\"fixture_note\""));
		assert!(sql.contains("\"fixture_owner_id\""));
		assert!(sql.contains("ORDER BY \"fixture_id\" ASC"));
		assert!(!sql.contains("displayName"));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_dump_uses_physical_columns_for_custom_foreign_keys() {
		let fields = fixture_database_fields::<FixtureCustomForeignKeyPost>().unwrap();

		assert_eq!(
			fields,
			vec![
				("id".to_string(), "fixture_id".to_string()),
				("author_id".to_string(), "author_fk".to_string()),
			]
		);

		let select = build_fixture_dump_select::<FixtureCustomForeignKeyPost>().unwrap();
		let (sql, _) = build_select_sql(&select, DatabaseBackend::Sqlite);
		assert!(sql.contains("\"author_fk\""));
		assert!(!sql.contains("\"author_id\""));

		let mut row = Map::new();
		row.insert("fixture_id".to_string(), Value::from(1));
		row.insert("author_fk".to_string(), Value::from(7));
		let fields = fixture_fields_from_database_row::<FixtureCustomForeignKeyPost>(&row).unwrap();

		assert_eq!(fields.get("author_id"), Some(&Value::from(7)));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_dump_hydrates_json_text_columns() {
		let mut row = Map::new();
		row.insert("id".to_string(), Value::from(1));
		row.insert(
			"payload".to_string(),
			Value::String(r#"{"theme":"paper"}"#.to_string()),
		);
		row.insert("optional_payload".to_string(), Value::Null);

		let fields = fixture_fields_from_database_row::<FixtureJsonPost>(&row).unwrap();

		assert_eq!(
			fields.get("payload"),
			Some(&serde_json::json!({"theme": "paper"}))
		);
		assert_eq!(fields.get("optional_payload"), Some(&Value::Null));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_dump_preserves_native_json_scalar_values() {
		let mut backend_row = crate::backends::types::Row::new();
		backend_row.insert("id".to_string(), QueryValue::Int(1));
		backend_row.insert(
			"payload".to_string(),
			QueryValue::Json(Some(Box::new(Value::String("draft".to_string())))),
		);
		backend_row.insert("optional_payload".to_string(), QueryValue::Json(None));
		let row = crate::orm::connection::QueryRow::from_backend_row(backend_row);

		let fields = fixture_fields_from_query_row::<FixtureJsonPost>(&row).unwrap();

		assert_eq!(
			fields.get("payload"),
			Some(&Value::String("draft".to_string()))
		);
		assert_eq!(fields.get("optional_payload"), Some(&Value::Null));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_json_null_sidecar_preserves_native_json_nulls() {
		let mut backend_row = crate::backends::types::Row::new();
		backend_row.insert("id".to_string(), QueryValue::Int(1));
		backend_row.insert(
			"payload".to_string(),
			QueryValue::Json(Some(Box::new(serde_json::json!({"theme": "paper"})))),
		);
		backend_row.insert(
			"optional_payload".to_string(),
			QueryValue::Json(Some(Box::new(Value::Null))),
		);
		let row = crate::orm::connection::QueryRow::from_backend_row(backend_row);

		let mut fields = fixture_fields_from_query_row::<FixtureJsonPost>(&row).unwrap();
		let json_null_fields =
			fixture_json_null_field_names_from_query_row::<FixtureJsonPost>(&row).unwrap();
		assert_eq!(
			json_null_fields,
			BTreeSet::from(["optional_payload".to_string()])
		);

		let pk = fields.remove("id");
		let mut record = FixtureRecord::new("fixture_json.FixtureJsonPost", pk, fields);
		record.json_null_fields = json_null_fields;
		let encoded = serde_json::to_value(&record).unwrap();
		assert_eq!(
			encoded["_reinhardt_json_null_fields"],
			serde_json::json!(["optional_payload"])
		);
		let record: FixtureRecord = serde_json::from_value(encoded).unwrap();
		assert_eq!(
			record.json_null_fields,
			BTreeSet::from(["optional_payload".to_string()])
		);

		let mut object = record.fields;
		object.insert("id".to_string(), record.pk.unwrap());
		let (_, values) = build_fixture_upsert_sql_values_with_json_null_fields::<FixtureJsonPost>(
			DatabaseBackend::Postgres,
			&object,
			&record.json_null_fields,
		)
		.unwrap();
		assert!(
			values
				.iter()
				.any(|value| matches!(value, QueryValue::Json(Some(value)) if value.is_null())),
			"marked fixture nulls must bind as JSON nulls"
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_json_text_nulls_are_marked_but_sql_nulls_are_not() {
		let mut json_text_row = crate::backends::types::Row::new();
		json_text_row.insert("id".to_string(), QueryValue::Int(1));
		json_text_row.insert(
			"payload".to_string(),
			QueryValue::String(r#"{"theme":"paper"}"#.to_string()),
		);
		json_text_row.insert(
			"optional_payload".to_string(),
			QueryValue::String("null".to_string()),
		);
		let json_text_row = crate::orm::connection::QueryRow::from_backend_row(json_text_row);
		assert_eq!(
			fixture_json_null_field_names_from_query_row::<FixtureJsonPost>(&json_text_row)
				.unwrap(),
			BTreeSet::from(["optional_payload".to_string()])
		);

		let mut sql_null_row = crate::backends::types::Row::new();
		sql_null_row.insert("id".to_string(), QueryValue::Int(1));
		sql_null_row.insert(
			"payload".to_string(),
			QueryValue::String(r#"{"theme":"paper"}"#.to_string()),
		);
		sql_null_row.insert("optional_payload".to_string(), QueryValue::Null);
		let sql_null_row = crate::orm::connection::QueryRow::from_backend_row(sql_null_row);
		assert!(
			fixture_json_null_field_names_from_query_row::<FixtureJsonPost>(&sql_null_row)
				.unwrap()
				.is_empty()
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_unmarked_json_nulls_remain_sql_nulls() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), serde_json::json!({"theme": "paper"}));
		object.insert("optional_payload".to_string(), Value::Null);

		let (sql, values) =
			build_fixture_upsert_sql_values::<FixtureJsonPost>(DatabaseBackend::Postgres, &object)
				.unwrap();

		assert!(
			sql.contains("NULL"),
			"unmarked fixture JSON nulls must produce SQL NULL: {sql}"
		);
		assert!(
			!values
				.iter()
				.any(|value| matches!(value, QueryValue::Json(Some(value)) if value.is_null())),
			"unmarked fixture JSON nulls must not bind as JSON nulls: {values:?}"
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_validation_rejects_invalid_json_null_sidecar_fields() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), serde_json::json!({"theme": "paper"}));
		object.insert("optional_payload".to_string(), Value::Null);
		let json_null_fields = BTreeSet::from(["missing_field".to_string()]);

		let error = validate_fixture_writable_projection_with_json_null_fields::<FixtureJsonPost>(
			&object,
			&json_null_fields,
		)
		.expect_err("unknown JSON-null sidecar fields must fail validation");
		assert!(error.to_string().contains("missing_field"));
	}

	#[cfg(feature = "migrations")]
	#[serial_test::serial(sqlx_drivers)]
	#[tokio::test]
	async fn fixture_dump_and_load_hydrates_sqlite_json_text() {
		let conn = DatabaseConnection::connect("sqlite::memory:")
			.await
			.unwrap();
		conn.execute(
			"CREATE TABLE fixture_json_post (\
				id INTEGER PRIMARY KEY, \
				payload TEXT NOT NULL, \
				optional_payload TEXT NULL)",
			vec![],
		)
		.await
		.unwrap();
		conn.execute(
			"INSERT INTO fixture_json_post (id, payload, optional_payload) VALUES (?, ?, ?)",
			vec![
				QueryValue::Int(1),
				QueryValue::String(r#"{"theme":"paper"}"#.to_string()),
				QueryValue::Null,
			],
		)
		.await
		.unwrap();

		let select = build_fixture_dump_select::<FixtureJsonPost>().unwrap();
		let (select_sql, select_values) = build_select_sql(&select, conn.backend());
		let rows = conn
			.query(
				&select_sql,
				crate::orm::execution::convert_values(select_values),
			)
			.await
			.unwrap();
		let fields = fixture_fields_from_query_row::<FixtureJsonPost>(&rows[0]).unwrap();

		assert_eq!(
			fields.get("payload"),
			Some(&serde_json::json!({"theme": "paper"}))
		);
		assert_eq!(fields.get("optional_payload"), Some(&Value::Null));

		conn.execute("DELETE FROM fixture_json_post", vec![])
			.await
			.unwrap();
		let (insert_sql, insert_values) =
			build_fixture_upsert_sql_values::<FixtureJsonPost>(conn.backend(), &fields).unwrap();
		conn.execute(&insert_sql, insert_values).await.unwrap();

		let restored = conn
			.query(
				"SELECT payload, optional_payload FROM fixture_json_post WHERE id = ?",
				vec![QueryValue::Int(1)],
			)
			.await
			.unwrap();
		let restored = restored[0]
			.data
			.as_object()
			.expect("restored row must be an object");
		assert_eq!(
			restored.get("payload"),
			Some(&Value::String(r#"{"theme":"paper"}"#.to_string()))
		);
		assert_eq!(restored.get("optional_payload"), Some(&Value::Null));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn mysql_fixture_upserts_do_not_update_non_primary_key_conflicts() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), serde_json::json!({"theme": "paper"}));

		let (sql, _) = build_fixture_upsert_sql_values::<FixtureFieldAwarePost>(
			DatabaseBackend::MySql,
			&object,
		)
		.unwrap();

		assert!(!sql.contains("ON DUPLICATE KEY UPDATE"));

		let (lookup_sql, lookup_values) = build_fixture_primary_key_lookup_sql_values::<
			FixtureFieldAwarePost,
		>(DatabaseBackend::MySql, &object)
		.unwrap();
		assert!(lookup_sql.starts_with("SELECT"));
		assert!(lookup_sql.contains("WHERE"));
		assert_eq!(lookup_values, vec![QueryValue::Int(1)]);

		let (update_sql, update_values) = build_fixture_update_sql_values::<FixtureFieldAwarePost>(
			DatabaseBackend::MySql,
			&object,
		)
		.unwrap()
		.expect("fixture with writable fields should produce an update");
		assert!(update_sql.starts_with("UPDATE"));
		assert!(update_sql.contains("WHERE"));
		assert!(!update_sql.contains("ON DUPLICATE KEY UPDATE"));
		assert_eq!(update_values.len(), 2);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn postgres_fixture_upserts_override_identity_always_primary_keys() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), Value::from("fixture payload"));

		let (sql, values) = build_fixture_upsert_sql_values::<FixtureIdentityAlwaysPost>(
			DatabaseBackend::Postgres,
			&object,
		)
		.unwrap();

		assert!(sql.contains("OVERRIDING SYSTEM VALUE"));
		assert!(sql.contains("ON CONFLICT"));
		assert_eq!(values.len(), 2);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_upserts_use_database_column_names() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("title".to_string(), Value::from("fixture title"));

		let (sql, values) = build_fixture_upsert_sql_values::<FixtureDatabaseColumnPost>(
			DatabaseBackend::Sqlite,
			&object,
		)
		.unwrap();

		assert!(sql.contains("\"fixture_id\""));
		assert!(sql.contains("\"fixture_title\""));
		assert!(sql.contains("ON CONFLICT (\"fixture_id\")"));
		assert_eq!(values.len(), 2);

		let (lookup_sql, _) = build_fixture_primary_key_lookup_sql_values::<
			FixtureDatabaseColumnPost,
		>(DatabaseBackend::MySql, &object)
		.unwrap();
		assert!(lookup_sql.contains("`fixture_id`"));

		let (update_sql, _) = build_fixture_update_sql_values::<FixtureDatabaseColumnPost>(
			DatabaseBackend::MySql,
			&object,
		)
		.unwrap()
		.expect("fixture with writable fields should produce an update");
		assert!(update_sql.contains("`fixture_title`"));
		assert!(update_sql.contains("`fixture_id`"));

		let handler = TypedFixtureModel::<FixtureDatabaseColumnPost>::new();
		let primary_key_column = handler.primary_key_database_column();
		assert_eq!(primary_key_column, "fixture_id");
		let sequence_sql =
			build_postgres_sequence_reset_sql(handler.table_name(), &primary_key_column);
		assert!(sequence_sql.contains("MAX(\"fixture_id\")"));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_upserts_bind_uuid_and_timestamp_shaped_text_as_strings() {
		let uuid_like = "123e4567-e89b-12d3-a456-426614174000";
		let timestamp_like = "2026-07-13T08:25:26+00:00";
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("uuid_like".to_string(), Value::from(uuid_like));
		object.insert("timestamp_like".to_string(), Value::from(timestamp_like));

		let (_, values) =
			build_fixture_upsert_sql_values::<FixtureTextPost>(DatabaseBackend::Postgres, &object)
				.unwrap();

		assert!(values.contains(&QueryValue::String(uuid_like.to_string())));
		assert!(values.contains(&QueryValue::String(timestamp_like.to_string())));
		assert!(
			!values
				.iter()
				.any(|value| matches!(value, QueryValue::Uuid(_) | QueryValue::Timestamp(_))),
			"text fixture values must not be rebound as UUID or timestamp parameters"
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn explicit_through_many_to_many_fixture_fields_are_not_replayed() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_m2m",
			"FixtureM2mPost",
			"fixture_m2m_post",
		);
		post.add_many_to_many(
			crate::migrations::ManyToManyMetadata::new("tags", "Tag")
				.with_through("fixture_m2m_post_tags")
				.with_source_field("post_id")
				.with_target_field("tag_id"),
		);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut object = Map::new();
		object.insert("title".to_string(), Value::from("Fixture"));
		object.insert(
			"tags".to_string(),
			Value::Array(vec![Value::from(1), Value::from(2)]),
		);

		let assignments = extract_many_to_many_assignments::<FixtureM2mPost>(&mut object).unwrap();

		assert!(assignments.is_empty());
		assert_eq!(object.get("title"), Some(&Value::from("Fixture")));
		assert!(object.get("tags").is_none());
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn dependency_order_places_self_referential_targets_first() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_self",
			"FixturePost",
			"fixture_self_post",
		);
		post.add_field(
			"parent_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "FixturePost")
				.with_param("fk_target_app", "fixture_self"),
		);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut child_fields = Map::new();
		child_fields.insert("parent".to_string(), Value::from(1));
		let records = vec![
			FixtureRecord::new(
				"fixture_self.FixturePost",
				Some(Value::from(2)),
				child_fields,
			),
			FixtureRecord::new("fixture_self.FixturePost", Some(Value::from(1)), Map::new()),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].pk, Some(Value::from(1)));
		assert_eq!(ordered[1].pk, Some(Value::from(2)));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn dependency_order_uses_primary_key_foreign_key_fixture_pk() {
		let mut parent = crate::migrations::ModelMetadata::new(
			"fixture_primary_fk",
			"Parent",
			"fixture_primary_fk_parent",
		);
		parent.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("primary_key", "true"),
		);

		let mut child = crate::migrations::ModelMetadata::new(
			"fixture_primary_fk",
			"Child",
			"fixture_primary_fk_child",
		);
		child.add_field(
			"parent_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("primary_key", "true")
				.with_param("fk_target", "Parent")
				.with_param("fk_target_app", "fixture_primary_fk"),
		);

		crate::migrations::model_registry::global_registry().register_model(parent);
		crate::migrations::model_registry::global_registry().register_model(child);

		let records = vec![
			FixtureRecord::new("fixture_primary_fk.Child", Some(Value::from(1)), Map::new()),
			FixtureRecord::new(
				"fixture_primary_fk.Parent",
				Some(Value::from(1)),
				Map::new(),
			),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_primary_fk.Parent");
		assert_eq!(ordered[1].model, "fixture_primary_fk.Child");
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn dependency_order_normalizes_model_label_case() {
		let mut author =
			crate::migrations::ModelMetadata::new("fixture_case", "Author", "fixture_case_author");
		author.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger),
		);
		let mut post =
			crate::migrations::ModelMetadata::new("fixture_case", "Post", "fixture_case_post");
		post.add_field(
			"author_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "Author")
				.with_param("fk_target_app", "fixture_case"),
		);
		crate::migrations::model_registry::global_registry().register_model(author);
		crate::migrations::model_registry::global_registry().register_model(post);
		let mut post_fields = Map::new();
		post_fields.insert("author".to_string(), Value::from(1));
		let records = vec![
			FixtureRecord::new("fixture_case.post", Some(Value::from(1)), post_fields),
			FixtureRecord::new("fixture_case.author", Some(Value::from(1)), Map::new()),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_case.author");
		assert_eq!(ordered[1].model, "fixture_case.post");
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn dependency_order_uses_non_null_fixture_fk_values() {
		let mut left =
			crate::migrations::ModelMetadata::new("fixture_cycle", "Left", "fixture_cycle_left");
		left.add_field(
			"right_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_nullable(true)
				.with_param("fk_target", "Right")
				.with_param("fk_target_app", "fixture_cycle"),
		);
		let mut right =
			crate::migrations::ModelMetadata::new("fixture_cycle", "Right", "fixture_cycle_right");
		right.add_field(
			"left_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_nullable(true)
				.with_param("fk_target", "Left")
				.with_param("fk_target_app", "fixture_cycle"),
		);
		crate::migrations::model_registry::global_registry().register_model(left);
		crate::migrations::model_registry::global_registry().register_model(right);
		let mut left_fields = Map::new();
		left_fields.insert("right".to_string(), Value::from(1));
		let mut right_fields = Map::new();
		right_fields.insert("left".to_string(), Value::Null);
		let records = vec![
			FixtureRecord::new("fixture_cycle.Left", Some(Value::from(1)), left_fields),
			FixtureRecord::new("fixture_cycle.Right", Some(Value::from(1)), right_fields),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_cycle.Right");
		assert_eq!(ordered[1].model, "fixture_cycle.Left");
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn dependency_order_tracks_cross_model_fixture_records() {
		let mut left = crate::migrations::ModelMetadata::new(
			"fixture_record_order",
			"Left",
			"fixture_record_order_left",
		);
		left.add_field(
			"right_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_nullable(true)
				.with_param("fk_target", "Right")
				.with_param("fk_target_app", "fixture_record_order"),
		);
		let mut right = crate::migrations::ModelMetadata::new(
			"fixture_record_order",
			"Right",
			"fixture_record_order_right",
		);
		right.add_field(
			"left_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_nullable(true)
				.with_param("fk_target", "Left")
				.with_param("fk_target_app", "fixture_record_order"),
		);
		crate::migrations::model_registry::global_registry().register_model(left);
		crate::migrations::model_registry::global_registry().register_model(right);

		let mut left_first_fields = Map::new();
		left_first_fields.insert("right".to_string(), Value::from(1));
		let mut right_second_fields = Map::new();
		right_second_fields.insert("left".to_string(), Value::from(2));
		let records = vec![
			FixtureRecord::new(
				"fixture_record_order.Left",
				Some(Value::from(1)),
				left_first_fields,
			),
			FixtureRecord::new(
				"fixture_record_order.Right",
				Some(Value::from(1)),
				Map::new(),
			),
			FixtureRecord::new(
				"fixture_record_order.Left",
				Some(Value::from(2)),
				Map::new(),
			),
			FixtureRecord::new(
				"fixture_record_order.Right",
				Some(Value::from(2)),
				right_second_fields,
			),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_record_order.Right");
		assert_eq!(ordered[0].pk, Some(Value::from(1)));
		assert_eq!(ordered[1].model, "fixture_record_order.Left");
		assert_eq!(ordered[1].pk, Some(Value::from(1)));
		assert_eq!(ordered[2].model, "fixture_record_order.Left");
		assert_eq!(ordered[2].pk, Some(Value::from(2)));
		assert_eq!(ordered[3].model, "fixture_record_order.Right");
		assert_eq!(ordered[3].pk, Some(Value::from(2)));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn composite_primary_key_models_are_rejected_for_fixtures() {
		let error = ensure_single_column_primary_key::<CompositeFixture>().unwrap_err();

		assert!(matches!(error, FixtureError::Database(_)));
		assert!(error.to_string().contains("composite primary keys"));
	}
}
