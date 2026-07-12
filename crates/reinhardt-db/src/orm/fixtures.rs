//! Django-compatible model fixture loading and dumping.
//!
//! The fixture runtime is type-erased at the registry boundary, while each
//! registered model still loads through its generated `Model` implementation
//! and `serde` validation.

use super::connection::{DatabaseBackend, QueryValue};
use super::manager::get_connection;
use super::transaction::TransactionScope;
use super::{DatabaseConnection, Manager, Model};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use reinhardt_query::prelude::{
	Alias, DeleteStatement, Expr, ExprTrait, InsertStatement, MySqlQueryBuilder, OnConflict,
	PostgresQueryBuilder, Query, QueryBuilder, SelectStatement, SqliteQueryBuilder, Values,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
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
/// The JSON shape is compatible with Django fixtures:
/// `{ "model": "app_label.ModelName", "pk": 1, "fields": { ... } }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureRecord {
	/// Fully-qualified model label.
	pub model: String,
	/// Primary key value.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub pk: Option<Value>,
	/// Non-primary-key field values.
	pub fields: Map<String, Value>,
}

impl FixtureRecord {
	/// Create a new fixture record.
	pub fn new(model: impl Into<String>, pk: Option<Value>, fields: Map<String, Value>) -> Self {
		Self {
			model: model.into(),
			pk,
			fields,
		}
	}
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
		M: Model + serde::de::DeserializeOwned + Serialize + 'static,
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
	M: Model + serde::de::DeserializeOwned + Serialize + 'static,
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
		let _model: M = serde_json::from_value(Value::Object(object.clone()))?;
		let (sql, values) = build_fixture_upsert_sql_values::<M>(conn, &object)?;
		tx.execute(&sql, values).await?;
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
		let manager = Manager::<M>::new();
		let rows = manager.order_by(&[M::primary_key_field()]).all().await?;
		let conn = get_connection().await?;
		let mut records = Vec::with_capacity(rows.len());
		let model_label = self.label();
		for row in rows {
			let value = serde_json::to_value(row)?;
			let mut object = value.as_object().cloned().ok_or_else(|| {
				FixtureError::Database("model must serialize to a JSON object".to_string())
			})?;
			let pk = object.remove(M::primary_key_field());
			denormalize_foreign_key_fixture_fields::<M>(&mut object)?;
			append_many_to_many_fixture_fields::<M>(&conn, pk.as_ref(), &mut object).await?;
			records.push(FixtureRecord::new(model_label.clone(), pk, object));
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

fn build_fixture_upsert_sql_values<M>(
	conn: &DatabaseConnection,
	object: &Map<String, Value>,
) -> FixtureResult<(String, Vec<QueryValue>)>
where
	M: Model,
{
	ensure_single_column_primary_key::<M>()?;
	if object.is_empty() {
		return Err(FixtureError::Database(
			"fixture record must contain at least one database column".to_string(),
		));
	}

	let pk_field = M::primary_key_field();
	let generated_fields = M::generated_field_names();
	let mut columns = object
		.keys()
		.filter(|column| !generated_fields.contains(&column.as_str()))
		.cloned()
		.collect::<Vec<_>>();
	if columns.is_empty() {
		return Err(FixtureError::Database(
			"fixture record must contain at least one writable database column".to_string(),
		));
	}
	columns.sort();
	if let Some(pk_index) = columns.iter().position(|column| column == pk_field) {
		columns.swap(0, pk_index);
	}

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
					field_info.is_some_and(|field| field.nullable && object[column].is_null());
				Manager::<M>::json_to_sea_value_for_field(
					&object[column],
					field_info,
					field_is_none,
				)
			})
			.collect::<Vec<_>>(),
	);

	if object.contains_key(pk_field) {
		let update_columns = columns
			.iter()
			.filter(|column| column.as_str() != pk_field)
			.map(|column| Alias::new(column.as_str()))
			.collect::<Vec<_>>();
		let conflict = if update_columns.is_empty() {
			OnConflict::column(Alias::new(pk_field)).do_nothing()
		} else {
			OnConflict::column(Alias::new(pk_field)).update_columns(update_columns)
		};
		stmt.on_conflict(conflict.to_owned());
	}

	let (sql, values) = build_insert_sql(&stmt, conn.backend());
	Ok((sql, super::execution::convert_values(values)))
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
		let table = quote_identifier_path(handler.table_name());
		let pk_field = quote_identifier(handler.primary_key_field());
		let sql = format!(
			"SELECT CASE \
			 WHEN pg_get_serial_sequence($1, $2) IS NULL THEN NULL \
			 ELSE setval(pg_get_serial_sequence($1, $2), \
			 COALESCE((SELECT MAX({pk_field}) FROM {table}), 1), \
			 (SELECT MAX({pk_field}) FROM {table}) IS NOT NULL) \
			 END",
		);
		tx.query_optional(
			&sql,
			vec![
				QueryValue::String(handler.table_name().to_string()),
				QueryValue::String(handler.primary_key_field().to_string()),
			],
		)
		.await?;
	}
	Ok(())
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
			let Some(value) = fixture_field_value(
				record,
				field_name,
				relationship_names.get(field_name).map(String::as_str),
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
	#[tokio::test]
	async fn fixture_upserts_preserve_json_values_and_skip_generated_fields() {
		let database_file = tempfile::NamedTempFile::new().unwrap();
		let database_url = format!("sqlite://{}", database_file.path().display());
		let connection = DatabaseConnection::connect(&database_url).await.unwrap();
		let payload = serde_json::json!({"theme": "paper"});
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), payload.clone());
		object.insert("generated_value".to_string(), Value::from(99));

		let (sql, values) =
			build_fixture_upsert_sql_values::<FixtureFieldAwarePost>(&connection, &object).unwrap();

		assert!(!sql.contains("generated_value"));
		assert_eq!(values.len(), 2);
		assert!(matches!(
			values.get(1),
			Some(QueryValue::Json(Some(value))) if **value == payload
		));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn many_to_many_fixture_fields_are_removed_before_model_deserialize() {
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

		remove_many_to_many_fixture_fields::<FixtureM2mPost>(&mut object).unwrap();

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
