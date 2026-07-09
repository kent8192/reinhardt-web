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

	/// Load one fixture record through this model handler.
	async fn load_record(
		&self,
		record: &FixtureRecord,
		conn: &DatabaseConnection,
		tx: &mut TransactionScope,
	) -> FixtureResult<()>;

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

		let mut object = record.fields.clone();
		let m2m_assignments = extract_many_to_many_assignments::<M>(&mut object)?;
		normalize_foreign_key_fixture_fields::<M>(&mut object)?;
		if let Some(pk) = &record.pk {
			object.insert(M::primary_key_field().to_string(), pk.clone());
		}
		let _model: M = serde_json::from_value(Value::Object(object.clone()))?;
		let (sql, values) = build_fixture_upsert_sql_values::<M>(conn, &object)?;
		tx.execute(&sql, values).await?;
		load_many_to_many_assignments::<M>(tx, conn, record.pk.as_ref(), &m2m_assignments).await?;
		Ok(())
	}

	async fn dump_records(&self) -> FixtureResult<Vec<FixtureRecord>> {
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
	if object.is_empty() {
		return Err(FixtureError::Database(
			"fixture record must contain at least one database column".to_string(),
		));
	}

	let pk_field = M::primary_key_field();
	let mut columns = object.keys().cloned().collect::<Vec<_>>();
	columns.sort();
	if let Some(pk_index) = columns.iter().position(|column| column == pk_field) {
		columns.swap(0, pk_index);
	}

	let mut stmt = Query::insert();
	stmt.into_table(Alias::new(M::table_name()));
	stmt.columns(columns.iter().map(|column| Alias::new(column.as_str())));
	stmt.values_panic(
		columns
			.iter()
			.map(|column| Manager::<M>::json_to_sea_value(&object[column]))
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

#[cfg(feature = "migrations")]
fn normalize_foreign_key_fixture_fields<M>(object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	let Some(metadata) = metadata_for_model::<M>() else {
		return Ok(());
	};
	for field_name in metadata.fields.keys() {
		if !is_foreign_key_field(&metadata, field_name) {
			continue;
		}
		let Some(relation_name) = field_name.strip_suffix("_id") else {
			continue;
		};
		if object.contains_key(field_name) {
			continue;
		}
		if let Some(value) = object.remove(relation_name) {
			object.insert(field_name.clone(), value);
		}
	}
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
	let present_models: HashSet<String> = records
		.iter()
		.map(|record| canonical_record_label(&record.model))
		.collect::<FixtureResult<_>>()?;
	let mut dependencies = HashMap::<String, HashSet<String>>::new();

	for model_key in &present_models {
		let (app_label, model_name) = parse_model_label(model_key)?;
		let mut model_dependencies = HashSet::new();
		if let Some(metadata) = find_model_metadata(&app_label, &model_name) {
			for field in metadata.fields.values() {
				if let Some(target_model) = field.params.get("fk_target") {
					let target_app = field
						.params
						.get("fk_target_app")
						.map(String::as_str)
						.unwrap_or(&app_label);
					let target_key = canonical_model_key(target_app, target_model);
					if target_key != *model_key && present_models.contains(&target_key) {
						model_dependencies.insert(target_key);
					}
				}
			}
		}
		dependencies.insert(model_key.clone(), model_dependencies);
	}

	let model_order = topological_model_order(&present_models, dependencies)?;
	let rank: HashMap<String, usize> = model_order
		.into_iter()
		.enumerate()
		.map(|(index, model)| (model, index))
		.collect();
	let mut indexed_records = records.iter().cloned().enumerate().collect::<Vec<_>>();
	indexed_records.sort_by_key(|(index, record)| {
		let model_key =
			canonical_record_label(&record.model).unwrap_or_else(|_| record.model.clone());
		(rank.get(&model_key).copied().unwrap_or(usize::MAX), *index)
	});
	let ordered_records = order_self_referential_record_groups(indexed_records)?;
	Ok(ordered_records
		.into_iter()
		.map(|(_, record)| record)
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
fn order_self_referential_record_groups(
	indexed_records: Vec<(usize, FixtureRecord)>,
) -> FixtureResult<Vec<(usize, FixtureRecord)>> {
	let mut ordered = Vec::with_capacity(indexed_records.len());
	let mut start = 0;
	while start < indexed_records.len() {
		let model_key = canonical_record_label(&indexed_records[start].1.model)?;
		let mut end = start + 1;
		while end < indexed_records.len()
			&& canonical_record_label(&indexed_records[end].1.model)? == model_key
		{
			end += 1;
		}
		ordered.extend(order_self_referential_record_group(
			&model_key,
			&indexed_records[start..end],
		)?);
		start = end;
	}
	Ok(ordered)
}

#[cfg(feature = "migrations")]
fn order_self_referential_record_group(
	model_key: &str,
	records: &[(usize, FixtureRecord)],
) -> FixtureResult<Vec<(usize, FixtureRecord)>> {
	let (app_label, model_name) = parse_model_label(model_key)?;
	let Some(metadata) = find_model_metadata(&app_label, &model_name) else {
		return Ok(records.to_vec());
	};
	let self_fk_fields = metadata
		.fields
		.iter()
		.filter_map(|(field_name, field)| {
			let target_model = field.params.get("fk_target")?;
			let target_app = field
				.params
				.get("fk_target_app")
				.map(String::as_str)
				.unwrap_or(&app_label);
			(canonical_model_key(target_app, target_model) == model_key).then(|| field_name.clone())
		})
		.collect::<Vec<_>>();
	if self_fk_fields.is_empty() {
		return Ok(records.to_vec());
	}

	let mut pk_to_local_index = HashMap::new();
	for (local_index, (_, record)) in records.iter().enumerate() {
		if let Some(pk_key) = record.pk.as_ref().and_then(json_dependency_key) {
			pk_to_local_index.insert(pk_key, local_index);
		}
	}

	let mut dependencies = HashMap::<usize, HashSet<usize>>::new();
	for (local_index, (_, record)) in records.iter().enumerate() {
		let mut deps = HashSet::new();
		for field_name in &self_fk_fields {
			let Some(value) = fixture_field_value(record, field_name) else {
				continue;
			};
			let Some(target_key) = json_dependency_key(value) else {
				continue;
			};
			let Some(target_index) = pk_to_local_index.get(&target_key).copied() else {
				continue;
			};
			if target_index != local_index {
				deps.insert(target_index);
			}
		}
		dependencies.insert(local_index, deps);
	}

	let mut remaining = (0..records.len()).collect::<HashSet<_>>();
	let mut ordered = Vec::with_capacity(records.len());
	while !remaining.is_empty() {
		let mut ready = remaining
			.iter()
			.filter(|index| {
				dependencies
					.get(index)
					.map(|deps| deps.is_disjoint(&remaining))
					.unwrap_or(true)
			})
			.copied()
			.collect::<Vec<_>>();
		ready.sort_by_key(|index| records[*index].0);
		if ready.is_empty() {
			return Err(FixtureError::DependencyCycle(model_key.to_string()));
		}
		for index in ready {
			remaining.remove(&index);
			ordered.push(records[index].clone());
		}
	}
	Ok(ordered)
}

#[cfg(feature = "migrations")]
fn fixture_field_value<'a>(record: &'a FixtureRecord, field_name: &str) -> Option<&'a Value> {
	record.fields.get(field_name).or_else(|| {
		field_name
			.strip_suffix("_id")
			.and_then(|relation_name| record.fields.get(relation_name))
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
fn topological_model_order(
	models: &HashSet<String>,
	mut dependencies: HashMap<String, HashSet<String>>,
) -> FixtureResult<Vec<String>> {
	let mut remaining = models.clone();
	let mut ordered = Vec::with_capacity(models.len());

	while !remaining.is_empty() {
		let mut ready: Vec<_> = remaining
			.iter()
			.filter(|model| {
				dependencies
					.get(*model)
					.map(|deps| deps.is_disjoint(&remaining))
					.unwrap_or(true)
			})
			.cloned()
			.collect();
		ready.sort();

		if ready.is_empty() {
			let mut cycle = remaining.into_iter().collect::<Vec<_>>();
			cycle.sort();
			return Err(FixtureError::DependencyCycle(cycle.join(", ")));
		}

		for model in ready {
			remaining.remove(&model);
			dependencies.remove(&model);
			ordered.push(model);
		}
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
		let records = vec![
			FixtureRecord::new("blog.Post", Some(Value::from(1)), Map::new()),
			FixtureRecord::new("blog.Author", Some(Value::from(1)), Map::new()),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "blog.Author");
		assert_eq!(ordered[1].model, "blog.Post");
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
		let records = vec![
			FixtureRecord::new("fixture_case.post", Some(Value::from(1)), Map::new()),
			FixtureRecord::new("fixture_case.author", Some(Value::from(1)), Map::new()),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_case.author");
		assert_eq!(ordered[1].model, "fixture_case.post");
	}
}
