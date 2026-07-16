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
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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

/// Foreign-key metadata exposed by a fixture model handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureForeignKeyRelation {
	/// Fixture field name used for the relation.
	pub fixture_field: String,
	/// Physical database column that stores the foreign key.
	pub database_column: String,
	/// Name of the related model.
	pub related_model: String,
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
	/// Return whether the primary key is bound as text in fixture statements.
	fn fixture_primary_key_is_text(&self) -> bool {
		false
	}

	/// Return the foreign-key relations needed for fixture value binding and ordering.
	fn fixture_foreign_key_relations(&self) -> Vec<FixtureForeignKeyRelation> {
		Vec::new()
	}

	/// Return identity columns explicitly written by a fixture record.
	fn fixture_written_identity_always_columns(
		&self,
		_record: &FixtureRecord,
	) -> FixtureResult<Vec<String>> {
		Ok(Vec::new())
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

	/// Map a physical database column back to its fixture field name.
	#[cfg(feature = "migrations")]
	fn fixture_field_name_for_database_column(&self, _database_column: &str) -> Option<String> {
		None
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

	fn fixture_primary_key_is_text(&self) -> bool {
		fixture_primary_key_binding_for_model::<M>() == FixturePrimaryKeyBinding::Text
	}

	fn fixture_foreign_key_relations(&self) -> Vec<FixtureForeignKeyRelation> {
		fixture_foreign_key_relations_from_relationship_metadata::<M>()
	}

	fn fixture_written_identity_always_columns(
		&self,
		record: &FixtureRecord,
	) -> FixtureResult<Vec<String>> {
		let mut object = record.fields.clone();
		remove_many_to_many_fixture_fields::<M>(&mut object)?;
		normalize_foreign_key_fixture_fields::<M>(&mut object)?;
		if let Some(primary_key) = &record.pk {
			object.insert(M::primary_key_field().to_string(), primary_key.clone());
		}
		let database_object = fixture_database_object::<M>(&object)?;
		Ok(M::field_metadata()
			.into_iter()
			.filter(|field| {
				database_object.contains_key(field.db_column_name())
					&& (matches!(
						field.attributes.get("identity_always"),
						Some(crate::orm::fields::FieldKwarg::Bool(true))
					) || matches!(
						field.attributes.get("identity_by_default"),
						Some(crate::orm::fields::FieldKwarg::Bool(true))
					))
			})
			.map(|field| field.db_column_name().to_string())
			.collect())
	}

	#[cfg(feature = "migrations")]
	fn fixture_foreign_key_fields(&self) -> Vec<(String, String)> {
		foreign_key_fixture_field_names_with_metadata_fallback::<M>()
	}

	#[cfg(feature = "migrations")]
	fn fixture_field_name_for_database_column(&self, database_column: &str) -> Option<String> {
		M::field_metadata()
			.into_iter()
			.find(|field| {
				field.name == database_column || field.db_column_name() == database_column
			})
			.map(|field| field.name)
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
	source_primary_key_binding: FixturePrimaryKeyBinding,
	target_primary_key_binding: FixturePrimaryKeyBinding,
	is_explicit_through: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixturePrimaryKeyBinding {
	Generic,
	Text,
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
		let hydrated_value = hydrate_fixture_database_value::<M>(
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
		let value = hydrate_fixture_database_value::<M>(
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

fn hydrate_fixture_database_value<M>(
	value: Value,
	field_info: Option<&crate::orm::inspection::FieldInfo>,
	field_name: &str,
	database_column: &str,
	is_native_json: bool,
) -> FixtureResult<Value>
where
	M: Model,
{
	let value = if is_native_json
		|| !field_info.is_some_and(|field| crate::orm::json::is_json_field_type(&field.field_type))
	{
		value
	} else {
		match value {
			Value::String(text) => serde_json::from_str(&text).map_err(|error| {
				FixtureError::Database(format!(
					"failed to hydrate JSON fixture field '{}.{}' from database column '{}': {error}",
					M::app_label(),
					field_name,
					database_column
				))
			})?,
			value => value,
		}
	};

	hydrate_fixture_binary_value::<M>(value, field_info, field_name, database_column)
}

fn hydrate_fixture_binary_value<M>(
	value: Value,
	field_info: Option<&crate::orm::inspection::FieldInfo>,
	field_name: &str,
	database_column: &str,
) -> FixtureResult<Value>
where
	M: Model,
{
	if !field_info.is_some_and(|field| is_fixture_binary_field_type(&field.field_type)) {
		return Ok(value);
	}

	match value {
		Value::String(encoded) => {
			use base64::Engine;
			base64::engine::general_purpose::STANDARD
				.decode(&encoded)
				.map_err(|error| {
					FixtureError::Database(format!(
						"failed to decode binary fixture field '{}.{}' from database column '{}': {error}",
						M::app_label(),
						field_name,
						database_column
					))
				})?;
			Ok(Value::String(encoded))
		}
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
	columns.sort();
	if let Some(primary_key_index) = columns.iter().position(|column| column == primary_key) {
		columns.swap(0, primary_key_index);
	}
	Ok(columns)
}

fn fixture_default_values_insert_sql(
	table_name: &str,
	backend: DatabaseBackend,
) -> (String, Vec<QueryValue>) {
	let quoted_table = match backend {
		DatabaseBackend::MySql => format!("`{table_name}`"),
		DatabaseBackend::Postgres | DatabaseBackend::Sqlite => format!("\"{table_name}\""),
	};
	let values = match backend {
		DatabaseBackend::MySql => "() VALUES ()",
		DatabaseBackend::Postgres | DatabaseBackend::Sqlite => "DEFAULT VALUES",
	};
	(format!("INSERT INTO {quoted_table} {values}"), Vec::new())
}

fn fixture_omitted_database_default_columns<M>(object: &Map<String, Value>) -> Vec<String>
where
	M: Model,
{
	M::field_metadata()
		.into_iter()
		.filter(|field| field.db_default.is_some() && !object.contains_key(field.db_column_name()))
		.map(|field| field.db_column_name().to_string())
		.collect()
}

fn fixture_writes_identity_always_column<M>(object: &Map<String, Value>) -> bool
where
	M: Model,
{
	M::field_metadata().into_iter().any(|field| {
		object.contains_key(field.db_column_name())
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
) -> FixtureResult<reinhardt_query::value::Value>
where
	M: Model,
{
	if field_info.is_some_and(|field| is_fixture_binary_field_type(&field.field_type)) {
		return fixture_binary_value_to_sea_value(value, field_is_none);
	}
	if field_info.is_some_and(|field| is_fixture_text_field_type(&field.field_type))
		&& let Value::String(value) = value
	{
		return Ok(reinhardt_query::value::Value::String(Some(Box::new(
			value.clone(),
		))));
	}
	Ok(Manager::<M>::json_to_sea_value_for_field(
		value,
		field_info,
		field_is_none,
	))
}

fn fixture_value_to_sea_value_for_database_column<M>(
	value: &Value,
	database_column: &str,
	field_info: Option<&crate::orm::inspection::FieldInfo>,
	field_is_none: bool,
) -> FixtureResult<reinhardt_query::value::Value>
where
	M: Model,
{
	#[cfg(feature = "migrations")]
	let foreign_key_binding = if let Some(binding) =
		fixture_foreign_key_binding_for_database_column::<M>(database_column)
	{
		Some(binding)
	} else {
		fixture_foreign_key_binding_from_relationship_metadata::<M>(database_column)?
	};
	#[cfg(not(feature = "migrations"))]
	let foreign_key_binding =
		fixture_foreign_key_binding_from_relationship_metadata::<M>(database_column)?;

	if foreign_key_binding == Some(FixturePrimaryKeyBinding::Text)
		&& let Value::String(value) = value
	{
		return Ok(reinhardt_query::value::Value::String(Some(Box::new(
			value.clone(),
		))));
	}

	fixture_value_to_sea_value_for_field::<M>(value, field_info, field_is_none)
}

fn fixture_binary_value_to_sea_value(
	value: &Value,
	field_is_none: bool,
) -> FixtureResult<reinhardt_query::value::Value> {
	if field_is_none {
		return Ok(reinhardt_query::value::Value::Bytes(None));
	}
	let bytes = match value {
		Value::String(encoded) => {
			use base64::Engine;
			base64::engine::general_purpose::STANDARD
				.decode(encoded)
				.map_err(|error| {
					FixtureError::Database(format!(
						"binary fixture fields must be valid base64 strings: {error}"
					))
				})?
		}
		Value::Array(values) => values
			.iter()
			.enumerate()
			.map(|(index, value)| {
				value
					.as_u64()
					.filter(|value| *value <= u8::MAX as u64)
					.map_or_else(
						|| {
							Err(FixtureError::Database(format!(
								"binary fixture byte at index {index} must be an integer between 0 and {}",
								u8::MAX
							)))
						},
						|value| Ok(value as u8),
					)
			})
			.collect::<FixtureResult<Vec<_>>>()?,
		_ => {
			return Err(FixtureError::Database(
				"binary fixture fields must be base64 strings or JSON byte arrays".to_string(),
			));
		}
	};
	Ok(reinhardt_query::value::Value::Bytes(Some(Box::new(bytes))))
}

fn fixture_field_is_identity_or_auto_increment(field: &crate::orm::inspection::FieldInfo) -> bool {
	[
		"identity_always",
		"identity_by_default",
		"auto_increment",
		"autoincrement",
	]
	.iter()
	.any(|name| {
		matches!(
			field.attributes.get(*name),
			Some(crate::orm::fields::FieldKwarg::Bool(true))
		)
	})
}

fn fixture_many_to_many_key_value<M>(
	value: &Value,
	binding: FixturePrimaryKeyBinding,
) -> reinhardt_query::value::Value
where
	M: Model,
{
	if binding == FixturePrimaryKeyBinding::Text
		&& let Value::String(value) = value
	{
		return reinhardt_query::value::Value::String(Some(Box::new(value.clone())));
	}
	Manager::<M>::json_to_sea_value(value)
}

fn fixture_primary_key_binding_for_model<M>() -> FixturePrimaryKeyBinding
where
	M: Model,
{
	let primary_key = M::primary_key_field();
	let field_info = M::field_metadata()
		.into_iter()
		.find(|field| field.primary_key || field.name == primary_key);
	fixture_primary_key_binding_from_field_info(field_info.as_ref())
}

fn fixture_primary_key_binding_from_field_info(
	field_info: Option<&crate::orm::inspection::FieldInfo>,
) -> FixturePrimaryKeyBinding {
	if field_info.is_some_and(|field| is_fixture_text_field_type(&field.field_type)) {
		FixturePrimaryKeyBinding::Text
	} else {
		FixturePrimaryKeyBinding::Generic
	}
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
		"CITextField",
	]
	.iter()
	.any(|field_name| field_type.ends_with(field_name))
}

fn is_fixture_binary_field_type(field_type: &str) -> bool {
	["BinaryField", "BlobField", "ByteaField"]
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
	let mut object = fixture_database_object::<M>(object)?;
	let pk_field = fixture_primary_key_column::<M>();
	if object.contains_key(&pk_field) {
		let omitted_defaults = fixture_omitted_database_default_columns::<M>(&object);
		if !omitted_defaults.is_empty() {
			return Err(FixtureError::Database(format!(
				"fixture record for '{}.{}' with primary key must include database-default column(s): {}",
				M::app_label(),
				rust_model_name::<M>(),
				omitted_defaults.join(", "),
			)));
		}
		for field in M::field_metadata().into_iter().filter(|field| {
			field.nullable
				&& !field.primary_key
				&& !M::generated_field_names().contains(&field.name.as_str())
				&& !fixture_field_is_identity_or_auto_increment(field)
		}) {
			let column = field.db_column_name().to_string();
			object.entry(column).or_insert(Value::Null);
		}
	}
	let columns = fixture_writable_columns(&object, &pk_field)?;
	if columns.is_empty() {
		return Ok(fixture_default_values_insert_sql(M::table_name(), backend));
	}

	let mut stmt = Query::insert();
	stmt.into_table(Alias::new(M::table_name()));
	stmt.columns(columns.iter().map(|column| Alias::new(column.as_str())));
	let field_metadata = M::field_metadata();
	let values = columns
		.iter()
		.map(|column| {
			let field_info = field_metadata.iter().find(|field| {
				field.name == *column || field.db_column.as_deref() == Some(column.as_str())
			});
			let field_is_none =
				fixture_field_is_none(&object[column], field_info, json_null_fields);
			fixture_value_to_sea_value_for_database_column::<M>(
				&object[column],
				column,
				field_info,
				field_is_none,
			)
		})
		.collect::<FixtureResult<Vec<_>>>()?;
	stmt.values_panic(values);
	if backend == DatabaseBackend::Postgres && fixture_writes_identity_always_column::<M>(&object) {
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
	let primary_key_value = fixture_value_to_sea_value_for_database_column::<M>(
		primary_key_value,
		&primary_key,
		field_info,
		field_is_none,
	)?;

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
	let mut object = fixture_database_object::<M>(object)?;
	let primary_key = fixture_primary_key_column::<M>();
	let omitted_defaults = fixture_omitted_database_default_columns::<M>(&object);
	if !omitted_defaults.is_empty() {
		return Err(FixtureError::Database(format!(
			"fixture record for '{}.{}' with primary key must include database-default column(s): {}",
			M::app_label(),
			rust_model_name::<M>(),
			omitted_defaults.join(", "),
		)));
	}
	for field in M::field_metadata().into_iter().filter(|field| {
		field.nullable
			&& !field.primary_key
			&& !M::generated_field_names().contains(&field.name.as_str())
			&& !fixture_field_is_identity_or_auto_increment(field)
	}) {
		object
			.entry(field.db_column_name().to_string())
			.or_insert(Value::Null);
	}
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
	let update_values = update_columns
		.iter()
		.map(|column| {
			let field_info = field_metadata.iter().find(|field| {
				field.name == *column || field.db_column.as_deref() == Some(column.as_str())
			});
			let field_is_none =
				fixture_field_is_none(&object[column], field_info, json_null_fields);
			Ok((
				Alias::new(column.as_str()),
				fixture_value_to_sea_value_for_database_column::<M>(
					&object[column],
					column,
					field_info,
					field_is_none,
				)?,
			))
		})
		.collect::<FixtureResult<Vec<_>>>()?;
	stmt.values(update_values);
	let primary_key_field = field_metadata.iter().find(|field| {
		field.name == primary_key || field.db_column.as_deref() == Some(primary_key.as_str())
	});
	let primary_key_is_none =
		fixture_field_is_none(primary_key_value, primary_key_field, json_null_fields);
	stmt.and_where(Expr::col(Alias::new(primary_key.as_str())).eq(
		fixture_value_to_sea_value_for_database_column::<M>(
			primary_key_value,
			&primary_key,
			primary_key_field,
			primary_key_is_none,
		)?,
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

fn foreign_key_fixture_field_names_from_relationship_metadata<M>() -> Vec<(String, String)>
where
	M: Model,
{
	M::relationship_metadata()
		.into_iter()
		.filter_map(|relationship| {
			relationship
				.foreign_key
				.map(|field_name| (field_name, relationship.name))
		})
		.collect()
}

fn fixture_foreign_key_relations_from_relationship_metadata<M>() -> Vec<FixtureForeignKeyRelation>
where
	M: Model,
{
	let field_metadata = M::field_metadata();
	M::relationship_metadata()
		.into_iter()
		.filter_map(|relationship| {
			if !matches!(
				relationship.relationship_type,
				crate::orm::relationship::RelationshipType::ManyToOne
					| crate::orm::relationship::RelationshipType::OneToOne
			) {
				return None;
			}
			let foreign_key = relationship.foreign_key?;
			let database_column = field_metadata
				.iter()
				.find(|field| field.name == foreign_key || field.db_column_name() == foreign_key)
				.map(crate::orm::inspection::FieldInfo::db_column_name)
				.unwrap_or(&foreign_key)
				.to_string();
			Some(FixtureForeignKeyRelation {
				fixture_field: relationship.name,
				database_column,
				related_model: relationship.related_model,
			})
		})
		.collect()
}

fn fixture_related_model_handler(
	current_app_label: &str,
	related_model: &str,
) -> FixtureResult<Option<Arc<dyn FixtureModelHandler>>> {
	let registry = global_fixture_registry();
	if related_model.contains('.') {
		return Ok(registry.get(related_model));
	}

	let related_model = related_model.rsplit("::").next().unwrap_or(related_model);
	let mut handlers = registry
		.all()
		.into_iter()
		.filter(|handler| handler.model_name().eq_ignore_ascii_case(related_model));
	let Some(handler) = handlers.next() else {
		return Ok(None);
	};
	if handlers.next().is_some() {
		return Err(FixtureError::Database(format!(
			"fixture relation from app '{}' references ambiguous model '{}'; use an app-qualified related model name",
			current_app_label, related_model
		)));
	}
	Ok(Some(handler))
}

fn fixture_foreign_key_binding_from_relationship_metadata<M>(
	database_column: &str,
) -> FixtureResult<Option<FixturePrimaryKeyBinding>>
where
	M: Model,
{
	let relation = fixture_foreign_key_relations_from_relationship_metadata::<M>()
		.into_iter()
		.find(|relation| relation.database_column == database_column);
	let Some(relation) = relation else {
		return Ok(None);
	};
	let Some(handler) = fixture_related_model_handler(M::app_label(), &relation.related_model)?
	else {
		return Ok(None);
	};
	Ok(Some(if handler.fixture_primary_key_is_text() {
		FixturePrimaryKeyBinding::Text
	} else {
		FixturePrimaryKeyBinding::Generic
	}))
}

#[cfg(feature = "migrations")]
fn foreign_key_fixture_field_names_with_metadata_fallback<M>() -> Vec<(String, String)>
where
	M: Model,
{
	metadata_for_model::<M>().map_or_else(
		foreign_key_fixture_field_names_from_relationship_metadata::<M>,
		|metadata| foreign_key_fixture_field_names::<M>(&metadata),
	)
}

#[cfg(feature = "migrations")]
fn normalize_foreign_key_fixture_fields<M>(object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	for (field_name, relation_name) in foreign_key_fixture_field_names_with_metadata_fallback::<M>()
	{
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
	let mut renames = Vec::new();
	for (field_name, relation_name) in foreign_key_fixture_field_names_with_metadata_fallback::<M>()
	{
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
	let relationship_names = foreign_key_fixture_field_names_from_relationship_metadata::<M>()
		.into_iter()
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
	let mut renames = Vec::new();
	for (field_name, relation_name) in
		foreign_key_fixture_field_names_from_relationship_metadata::<M>()
	{
		if !_object.contains_key(&field_name) || _object.contains_key(&relation_name) {
			continue;
		}
		renames.push((field_name, relation_name));
	}
	for (field_name, relation_name) in renames {
		if let Some(value) = _object.remove(&field_name) {
			_object.insert(relation_name, value);
		}
	}
	Ok(())
}

#[cfg(not(feature = "migrations"))]
fn normalize_foreign_key_fixture_fields<M>(object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	for (field_name, relation_name) in
		foreign_key_fixture_field_names_from_relationship_metadata::<M>()
	{
		if object.contains_key(&field_name) {
			continue;
		}
		if let Some(value) = object.remove(&relation_name) {
			object.insert(field_name, value);
		}
	}
	Ok(())
}

fn extract_many_to_many_assignments<M>(
	object: &mut Map<String, Value>,
) -> FixtureResult<Vec<FixtureManyToManyAssignment>>
where
	M: Model,
{
	let mut assignments = Vec::new();
	for spec in many_to_many_specs_for::<M>()? {
		let Some(raw_value) = object.remove(&spec.field_name) else {
			continue;
		};
		if spec.is_explicit_through {
			return Err(FixtureError::Database(format!(
				"many-to-many fixture field '{}' uses an explicit through model; load through-model records instead",
				spec.field_name
			)));
		}
		let values = raw_value.as_array().cloned().ok_or_else(|| {
			FixtureError::Database(format!(
				"many-to-many fixture field '{}' must be an array",
				spec.field_name
			))
		})?;
		if values.iter().any(Value::is_null) {
			return Err(FixtureError::Database(format!(
				"many-to-many fixture field '{}' must not contain null identifiers",
				spec.field_name
			)));
		}
		assignments.push(FixtureManyToManyAssignment { spec, values });
	}
	Ok(assignments)
}

fn remove_many_to_many_fixture_fields<M>(object: &mut Map<String, Value>) -> FixtureResult<()>
where
	M: Model,
{
	let _ = extract_many_to_many_assignments::<M>(object)?;
	Ok(())
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
		let source_value = fixture_many_to_many_key_value::<M>(
			source_pk,
			assignment.spec.source_primary_key_binding,
		);
		let mut delete = Query::delete();
		delete
			.from_table(Alias::new(assignment.spec.through_table.as_str()))
			.and_where(
				Expr::col(Alias::new(assignment.spec.source_field.as_str()))
					.eq(source_value.clone()),
			);
		let (sql, values) = build_delete_sql(&delete, conn.backend());
		tx.execute(&sql, super::execution::convert_values(values))
			.await?;

		for target_pk in &assignment.values {
			let target_value = fixture_many_to_many_key_value::<M>(
				target_pk,
				assignment.spec.target_primary_key_binding,
			);
			let mut insert = Query::insert();
			insert
				.into_table(Alias::new(assignment.spec.through_table.as_str()))
				.columns([
					Alias::new(assignment.spec.source_field.as_str()),
					Alias::new(assignment.spec.target_field.as_str()),
				])
				.values_panic([source_value.clone(), target_value])
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
	for spec in many_to_many_specs_for::<M>()? {
		if spec.is_explicit_through {
			continue;
		}
		let source_value = fixture_many_to_many_key_value::<M>(pk, spec.source_primary_key_binding);
		let mut select = Query::select();
		select
			.column(Alias::new(spec.target_field.as_str()))
			.from(Alias::new(spec.through_table.as_str()))
			.and_where(Expr::col(Alias::new(spec.source_field.as_str())).eq(source_value))
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
fn fixture_m2m_has_explicit_through_model(
	through_table: &str,
	source_field: &str,
	target_field: &str,
) -> bool {
	let Some(metadata) = crate::migrations::model_registry::global_registry()
		.get_models()
		.into_iter()
		.find(|metadata| metadata.table_name.eq_ignore_ascii_case(through_table))
	else {
		return false;
	};

	metadata.fields.iter().any(|(field_name, field)| {
		let matches_column = |column: &str| {
			field_name.eq_ignore_ascii_case(column)
				|| field
					.params
					.get("db_column")
					.is_some_and(|db_column| db_column.eq_ignore_ascii_case(column))
		};
		let is_through_key = matches_column(source_field) || matches_column(target_field);
		let is_implicit_primary_key = field_name.eq_ignore_ascii_case("id")
			|| field
				.params
				.get("primary_key")
				.is_some_and(|primary_key| primary_key == "true");
		!is_through_key && !is_implicit_primary_key
	})
}

#[cfg(not(feature = "migrations"))]
fn fixture_m2m_has_explicit_through_model(
	_through_table: &str,
	_source_field: &str,
	_target_field: &str,
) -> bool {
	false
}

#[cfg(feature = "migrations")]
fn many_to_many_specs_for<M>() -> FixtureResult<Vec<FixtureManyToManySpec>>
where
	M: Model,
{
	let Some(metadata) = metadata_for_model::<M>() else {
		return many_to_many_specs_from_relationship_metadata::<M>();
	};
	metadata
		.many_to_many_fields
		.iter()
		.map(|field| {
			let target_metadata = related_model_metadata(&metadata.app_label, &field.to_model);
			let target_handler = global_fixture_registry().get(&field.to_model).or_else(|| {
				global_fixture_registry()
					.get(&canonical_label(&metadata.app_label, &field.to_model))
			});
			let target_table = target_metadata
				.as_ref()
				.map(|target| target.table_name.clone())
				.or_else(|| {
					target_handler
						.as_ref()
						.map(|handler| handler.table_name().to_string())
				})
				.unwrap_or_else(|| default_target_table_name(&field.to_model));
			let source_primary_key_binding = fixture_primary_key_binding_for_model::<M>();
			let target_primary_key_binding = target_metadata
				.as_ref()
				.map(fixture_primary_key_binding_for_registered_model)
				.or_else(|| {
					target_handler.as_ref().map(|handler| {
						if handler.fixture_primary_key_is_text() {
							FixturePrimaryKeyBinding::Text
						} else {
							FixturePrimaryKeyBinding::Generic
						}
					})
				})
				.unwrap_or(FixturePrimaryKeyBinding::Generic);
			let through_table = field.through.clone().unwrap_or_else(|| {
				crate::m2m_naming::default_through_table(M::table_name(), &field.field_name)
			});
			let (default_source_field, default_target_field) =
				crate::m2m_naming::default_m2m_columns(M::table_name(), &target_table);
			let source_field = field.source_field.clone().unwrap_or(default_source_field);
			let target_field = field.target_field.clone().unwrap_or(default_target_field);
			let is_explicit_through = field.through.is_some()
				&& fixture_m2m_has_explicit_through_model(
					&through_table,
					&source_field,
					&target_field,
				);
			Ok(FixtureManyToManySpec {
				field_name: field.field_name.clone(),
				through_table,
				source_field,
				target_field,
				source_primary_key_binding,
				target_primary_key_binding,
				is_explicit_through,
			})
		})
		.collect()
}

fn many_to_many_specs_from_relationship_metadata<M>() -> FixtureResult<Vec<FixtureManyToManySpec>>
where
	M: Model,
{
	M::relationship_metadata()
		.into_iter()
		.filter(|relation| {
			relation.relationship_type == crate::orm::relationship::RelationshipType::ManyToMany
		})
		.map(|relation| {
			let field_name = relation.name;
			let custom_through_table = relation.through_table;
			let target_handler =
				fixture_related_model_handler(M::app_label(), &relation.related_model)?;
			let target_table = target_handler
				.as_ref()
				.map(|handler| handler.table_name().to_string())
				.unwrap_or_else(|| default_target_table_name(&relation.related_model));
			let (default_source_field, default_target_field) =
				crate::m2m_naming::default_m2m_columns(M::table_name(), &target_table);
			let through_table = custom_through_table.clone().unwrap_or_else(|| {
				crate::m2m_naming::default_through_table(M::table_name(), &field_name)
			});
			let source_field = relation.source_field.unwrap_or(default_source_field);
			let target_field = relation.target_field.unwrap_or(default_target_field);
			let is_explicit_through = custom_through_table.is_some()
				&& fixture_m2m_has_explicit_through_model(
					&through_table,
					&source_field,
					&target_field,
				);
			Ok(FixtureManyToManySpec {
				field_name: field_name.clone(),
				through_table,
				source_field,
				target_field,
				source_primary_key_binding: fixture_primary_key_binding_for_model::<M>(),
				target_primary_key_binding: target_handler.map_or(
					FixturePrimaryKeyBinding::Generic,
					|handler| {
						if handler.fixture_primary_key_is_text() {
							FixturePrimaryKeyBinding::Text
						} else {
							FixturePrimaryKeyBinding::Generic
						}
					},
				),
				is_explicit_through,
			})
		})
		.collect()
}

#[cfg(not(feature = "migrations"))]
fn many_to_many_specs_for<M>() -> FixtureResult<Vec<FixtureManyToManySpec>>
where
	M: Model,
{
	many_to_many_specs_from_relationship_metadata::<M>()
}

async fn reset_sequences_after_explicit_pks(
	conn: &DatabaseConnection,
	tx: &mut TransactionScope,
	records: &[FixtureRecord],
) -> FixtureResult<()> {
	if conn.backend() != DatabaseBackend::Postgres {
		return Ok(());
	}

	for (handler, column) in
		fixture_identity_sequence_reset_targets(global_fixture_registry(), records)?
	{
		let sql = build_postgres_sequence_reset_sql(handler.table_name(), &column);
		tx.query_optional(
			&sql,
			vec![
				QueryValue::String(handler.table_name().to_string()),
				QueryValue::String(column),
			],
		)
		.await?;
	}
	Ok(())
}

fn fixture_identity_sequence_reset_targets(
	registry: &FixtureRegistry,
	records: &[FixtureRecord],
) -> FixtureResult<Vec<(Arc<dyn FixtureModelHandler>, String)>> {
	let mut targets = BTreeMap::new();
	for record in records {
		let handler = registry
			.get(&record.model)
			.ok_or_else(|| FixtureError::ModelNotRegistered(record.model.clone()))?;
		if record.pk.as_ref().is_some_and(|primary_key| {
			primary_key.as_i64().is_some() || primary_key.as_u64().is_some()
		}) {
			targets
				.entry((handler.label(), handler.primary_key_database_column()))
				.or_insert_with(|| Arc::clone(&handler));
		}
		for column in handler.fixture_written_identity_always_columns(record)? {
			targets
				.entry((handler.label(), column))
				.or_insert_with(|| Arc::clone(&handler));
		}
	}
	Ok(targets
		.into_iter()
		.map(|((_, column), handler)| (handler, column))
		.collect())
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
fn fixture_foreign_key_target_metadata(
	source_metadata: &crate::migrations::model_registry::ModelMetadata,
	field: &crate::migrations::model_registry::FieldMetadata,
) -> Option<crate::migrations::model_registry::ModelMetadata> {
	field
		.params
		.get("fk_target")
		.and_then(|target_model| {
			let target_app = field
				.params
				.get("fk_target_app")
				.map(String::as_str)
				.unwrap_or(&source_metadata.app_label);
			related_model_metadata(target_app, target_model)
		})
		.or_else(|| {
			let referenced_table = &field.foreign_key.as_ref()?.referenced_table;
			crate::migrations::model_registry::global_registry()
				.get_models()
				.into_iter()
				.find(|metadata| metadata.table_name.eq_ignore_ascii_case(referenced_table))
		})
}

#[cfg(feature = "migrations")]
fn fixture_primary_key_field_name(
	metadata: &crate::migrations::model_registry::ModelMetadata,
) -> String {
	metadata
		.fields
		.iter()
		.find(|(_, field)| {
			field
				.params
				.get("primary_key")
				.is_some_and(|value| value == "true")
		})
		.map(|(field_name, _)| field_name.clone())
		.unwrap_or_else(|| "id".to_string())
}

#[cfg(feature = "migrations")]
fn fixture_foreign_key_referenced_column(
	field: &crate::migrations::model_registry::FieldMetadata,
	target_metadata: &crate::migrations::model_registry::ModelMetadata,
) -> String {
	let database_column = field
		.foreign_key
		.as_ref()
		.map(|foreign_key| foreign_key.referenced_column.clone())
		.unwrap_or_else(|| fixture_primary_key_field_name(target_metadata));
	let target_label = canonical_label(&target_metadata.app_label, &target_metadata.model_name);
	global_fixture_registry()
		.get(&target_label)
		.and_then(|handler| handler.fixture_field_name_for_database_column(&database_column))
		.or_else(|| {
			target_metadata
				.fields
				.contains_key(&database_column)
				.then_some(database_column.clone())
		})
		.or_else(|| {
			target_metadata
				.fields
				.iter()
				.find_map(|(field_name, metadata)| {
					(metadata.params.get("db_column") == Some(&database_column))
						.then(|| field_name.clone())
				})
		})
		.unwrap_or(database_column)
}

#[cfg(feature = "migrations")]
fn fixture_foreign_key_binding_for_database_column<M>(
	database_column: &str,
) -> Option<FixturePrimaryKeyBinding>
where
	M: Model,
{
	let source_metadata = metadata_for_model::<M>()?;
	let source_field = M::field_metadata()
		.into_iter()
		.find(|field| field.name == database_column || field.db_column_name() == database_column)
		.and_then(|field| source_metadata.fields.get(&field.name))
		.or_else(|| source_metadata.fields.get(database_column))?;
	let target_metadata = fixture_foreign_key_target_metadata(&source_metadata, source_field)?;
	let referenced_column = fixture_foreign_key_referenced_column(source_field, &target_metadata);
	target_metadata.fields.get(&referenced_column).map(|field| {
		if is_fixture_text_migration_field_type(&field.field_type) {
			FixturePrimaryKeyBinding::Text
		} else {
			FixturePrimaryKeyBinding::Generic
		}
	})
}

#[cfg(feature = "migrations")]
fn fixture_primary_key_binding_for_registered_model(
	metadata: &crate::migrations::model_registry::ModelMetadata,
) -> FixturePrimaryKeyBinding {
	let field = metadata
		.fields
		.iter()
		.find(|(_, field)| {
			field
				.params
				.get("primary_key")
				.is_some_and(|value| value == "true")
		})
		.or_else(|| metadata.fields.get_key_value("id"));
	if field.is_some_and(|(_, field)| is_fixture_text_migration_field_type(&field.field_type)) {
		FixturePrimaryKeyBinding::Text
	} else {
		FixturePrimaryKeyBinding::Generic
	}
}

#[cfg(feature = "migrations")]
fn is_fixture_text_migration_field_type(field_type: &crate::migrations::FieldType) -> bool {
	matches!(
		field_type,
		crate::migrations::FieldType::Char(_)
			| crate::migrations::FieldType::VarChar(_)
			| crate::migrations::FieldType::Text
			| crate::migrations::FieldType::TinyText
			| crate::migrations::FieldType::MediumText
			| crate::migrations::FieldType::LongText
			| crate::migrations::FieldType::CIText
			| crate::migrations::FieldType::Enum { .. }
			| crate::migrations::FieldType::Set { .. }
	)
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
	let dependencies = fixture_record_dependencies_from_relationship_metadata(records)?;
	let order = topological_record_order(records, dependencies)?;
	Ok(order
		.into_iter()
		.map(|index| records[index].clone())
		.collect())
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
		let model_key = canonical_record_label(&record.model)?;
		let primary_key_column = parse_model_label(&model_key)
			.ok()
			.and_then(|(app_label, model_name)| find_model_metadata(&app_label, &model_name))
			.map(|metadata| fixture_primary_key_field_name(&metadata))
			.unwrap_or_else(|| "id".to_string());
		if let Some(pk_key) = record.pk.as_ref().and_then(json_dependency_key) {
			record_indices
				.entry((model_key.clone(), primary_key_column, pk_key))
				.or_insert(index);
		}
		for (field_name, value) in &record.fields {
			let Some(value_key) = json_dependency_key(value) else {
				continue;
			};
			record_indices
				.entry((model_key.clone(), field_name.clone(), value_key))
				.or_insert(index);
		}
	}

	let mut dependencies = fixture_record_dependencies_from_relationship_metadata(records)?;

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
			let Some((target_key, target_column)) = fixture_foreign_key_target(&metadata, field)
			else {
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
			let Some(target_value) = json_dependency_key(value) else {
				continue;
			};
			let Some(target_index) = record_indices
				.get(&(target_key, target_column, target_value))
				.copied()
			else {
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

fn fixture_record_dependencies_from_relationship_metadata(
	records: &[FixtureRecord],
) -> FixtureResult<HashMap<usize, HashSet<usize>>> {
	let mut record_indices = HashMap::new();
	for (index, record) in records.iter().enumerate() {
		let model_key = canonical_record_label(&record.model)?;
		let primary_key_field = global_fixture_registry()
			.get(&record.model)
			.map(|handler| handler.primary_key_field().to_string())
			.unwrap_or_else(|| "id".to_string());
		if let Some(pk_key) = record.pk.as_ref().and_then(json_dependency_key) {
			record_indices
				.entry((model_key.clone(), primary_key_field, pk_key))
				.or_insert(index);
		}
		for (field_name, value) in &record.fields {
			let Some(value_key) = json_dependency_key(value) else {
				continue;
			};
			record_indices
				.entry((model_key.clone(), field_name.clone(), value_key))
				.or_insert(index);
		}
	}

	let mut dependencies = (0..records.len())
		.map(|index| (index, HashSet::new()))
		.collect::<HashMap<_, _>>();
	for (source_index, record) in records.iter().enumerate() {
		#[cfg(feature = "migrations")]
		if parse_model_label(&record.model)
			.ok()
			.and_then(|(app_label, model_name)| find_model_metadata(&app_label, &model_name))
			.is_some()
		{
			continue;
		}
		let Some(source_handler) = global_fixture_registry().get(&record.model) else {
			continue;
		};
		for relation in source_handler.fixture_foreign_key_relations() {
			let is_primary_key_foreign_key =
				relation.database_column == source_handler.primary_key_database_column();
			let Some(value) = fixture_field_value(
				record,
				&relation.database_column,
				Some(&relation.fixture_field),
				is_primary_key_foreign_key,
			) else {
				continue;
			};
			let Some(target_value) = json_dependency_key(value) else {
				continue;
			};
			let Some(target_handler) =
				fixture_related_model_handler(source_handler.app_label(), &relation.related_model)?
			else {
				continue;
			};
			let target_key = target_handler.label();
			let target_primary_key = target_handler.primary_key_field().to_string();
			let Some(target_index) = record_indices
				.get(&(target_key, target_primary_key, target_value))
				.copied()
			else {
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
fn fixture_foreign_key_target(
	source_metadata: &crate::migrations::model_registry::ModelMetadata,
	field: &crate::migrations::model_registry::FieldMetadata,
) -> Option<(String, String)> {
	if let Some(target_model) = field.params.get("fk_target") {
		let target_app = field
			.params
			.get("fk_target_app")
			.map(String::as_str)
			.unwrap_or(&source_metadata.app_label);
		let target_metadata = related_model_metadata(target_app, target_model);
		let referenced_column = target_metadata
			.as_ref()
			.map(|metadata| fixture_foreign_key_referenced_column(field, metadata))
			.or_else(|| {
				field
					.foreign_key
					.as_ref()
					.map(|foreign_key| foreign_key.referenced_column.clone())
			})
			.unwrap_or_else(|| "id".to_string());
		let referenced_column = if target_metadata.is_none() {
			fixture_related_model_handler(target_app, target_model)
				.ok()
				.flatten()
				.and_then(|handler| {
					handler.fixture_field_name_for_database_column(&referenced_column)
				})
				.unwrap_or(referenced_column)
		} else {
			referenced_column
		};
		let target_key = target_metadata.as_ref().map_or_else(
			|| {
				fixture_related_model_handler(target_app, target_model)
					.ok()
					.flatten()
					.map(|handler| handler.label())
					.unwrap_or_else(|| canonical_model_key(target_app, target_model))
			},
			|metadata| canonical_label(&metadata.app_label, &metadata.model_name),
		);
		return Some((target_key, referenced_column));
	}

	let target_metadata = fixture_foreign_key_target_metadata(source_metadata, field)?;
	let referenced_column = fixture_foreign_key_referenced_column(field, &target_metadata);
	Some((
		canonical_label(&target_metadata.app_label, &target_metadata.model_name),
		referenced_column,
	))
}

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

fn json_dependency_key(value: &Value) -> Option<String> {
	if value.is_null() {
		None
	} else {
		Some(value.to_string())
	}
}

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

fn canonical_record_label(label: &str) -> FixtureResult<String> {
	let (app_label, model_name) = parse_model_label(label)?;
	if let Some(handler) = global_fixture_registry().get(label) {
		return Ok(handler.label());
	}
	#[cfg(feature = "migrations")]
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

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyPost {
		id: Option<i64>,
		author_id: i64,
	}

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone)]
	struct FixtureOrmOnlyPostFields;

	#[cfg(not(feature = "migrations"))]
	impl crate::orm::model::FieldSelector for FixtureOrmOnlyPostFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyPost {
		type PrimaryKey = i64;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_post"
		}

		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}

		fn app_label() -> &'static str {
			"fixture_orm_only"
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

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"author",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"FixtureOrmOnlyAuthor",
				)
				.with_foreign_key("author_id"),
			]
		}
	}

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyM2mTag {
		id: Option<i64>,
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyM2mTag {
		type PrimaryKey = i64;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_custom_tags"
		}
		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}
		fn app_label() -> &'static str {
			"fixture_orm_only"
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

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyM2mPost {
		id: Option<i64>,
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyM2mPost {
		type PrimaryKey = i64;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_m2m_post"
		}
		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}
		fn app_label() -> &'static str {
			"fixture_orm_only"
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

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![crate::orm::inspection::RelationInfo::new(
				"tags",
				crate::orm::relationship::RelationshipType::ManyToMany,
				"FixtureOrmOnlyM2mTag",
			)]
		}
	}

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyTextAuthor {
		id: Option<String>,
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyTextAuthor {
		type PrimaryKey = String;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_text_author"
		}

		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}

		fn app_label() -> &'static str {
			"fixture_orm_only"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id.clone()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "CharField", false);
			id.primary_key = true;
			vec![id]
		}
	}

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyTextForeignKeyPost {
		id: Option<i64>,
		author_id: String,
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyTextForeignKeyPost {
		type PrimaryKey = i64;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_text_foreign_key_post"
		}

		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}

		fn app_label() -> &'static str {
			"fixture_orm_only"
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
			vec![id, fixture_field_info("author_id", "IntegerField", false)]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"author",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"FixtureOrmOnlyTextAuthor",
				)
				.with_foreign_key("author_id"),
			]
		}
	}

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyPrimaryKeyForeignKeyParent {
		id: Option<i64>,
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyPrimaryKeyForeignKeyParent {
		type PrimaryKey = i64;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_primary_key_foreign_key_parent"
		}

		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}

		fn app_label() -> &'static str {
			"fixture_orm_only_primary_key_foreign_key"
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
			vec![id]
		}
	}

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyPrimaryKeyForeignKeyChild {
		id: Option<i64>,
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyPrimaryKeyForeignKeyChild {
		type PrimaryKey = i64;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_primary_key_foreign_key_child"
		}

		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}

		fn app_label() -> &'static str {
			"fixture_orm_only_primary_key_foreign_key"
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
			vec![id]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"parent",
					crate::orm::relationship::RelationshipType::OneToOne,
					"FixtureOrmOnlyPrimaryKeyForeignKeyParent",
				)
				.with_foreign_key("id"),
			]
		}
	}

	#[cfg(not(feature = "migrations"))]
	mod fixture_orm_only_ambiguous_auth {
		use super::*;

		#[derive(Clone, Serialize, Deserialize)]
		pub(super) struct User {
			id: Option<i64>,
		}

		impl Model for User {
			type PrimaryKey = i64;
			type Fields = FixtureOrmOnlyPostFields;
			type Objects = Manager<Self>;

			fn table_name() -> &'static str {
				"fixture_orm_only_ambiguous_auth_user"
			}

			fn new_fields() -> Self::Fields {
				FixtureOrmOnlyPostFields
			}

			fn app_label() -> &'static str {
				"fixture_orm_only_ambiguous_auth"
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
	}

	#[cfg(not(feature = "migrations"))]
	mod fixture_orm_only_ambiguous_blog {
		use super::*;

		#[derive(Clone, Serialize, Deserialize)]
		pub(super) struct User {
			id: Option<i64>,
		}

		impl Model for User {
			type PrimaryKey = i64;
			type Fields = FixtureOrmOnlyPostFields;
			type Objects = Manager<Self>;

			fn table_name() -> &'static str {
				"fixture_orm_only_ambiguous_blog_user"
			}

			fn new_fields() -> Self::Fields {
				FixtureOrmOnlyPostFields
			}

			fn app_label() -> &'static str {
				"fixture_orm_only_ambiguous_blog"
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
	}

	#[cfg(not(feature = "migrations"))]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureOrmOnlyAmbiguousPost {
		id: Option<i64>,
		author_id: i64,
	}

	#[cfg(not(feature = "migrations"))]
	impl Model for FixtureOrmOnlyAmbiguousPost {
		type PrimaryKey = i64;
		type Fields = FixtureOrmOnlyPostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_orm_only_ambiguous_post"
		}

		fn new_fields() -> Self::Fields {
			FixtureOrmOnlyPostFields
		}

		fn app_label() -> &'static str {
			"fixture_orm_only_ambiguous_blog"
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
				fixture_field_info("author_id", "BigIntegerField", false),
			]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"author",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"User",
				)
				.with_foreign_key("author_id"),
			]
		}
	}

	#[cfg(not(feature = "migrations"))]
	struct FixtureRegistryReset;

	#[cfg(not(feature = "migrations"))]
	impl Drop for FixtureRegistryReset {
		fn drop(&mut self) {
			global_fixture_registry().clear();
		}
	}

	#[cfg(not(feature = "migrations"))]
	#[test]
	fn orm_only_fixture_fields_normalize_relationship_names() {
		let mut object = Map::new();
		object.insert("author".to_string(), Value::from(42));

		normalize_foreign_key_fixture_fields::<FixtureOrmOnlyPost>(&mut object)
			.expect("ORM-only fixtures must accept relation names");

		assert_eq!(object.get("author_id"), Some(&Value::from(42)));
		assert!(!object.contains_key("author"));

		denormalize_foreign_key_fixture_fields::<FixtureOrmOnlyPost>(&mut object)
			.expect("ORM-only fixture dumps must use relation names");

		assert_eq!(object.get("author"), Some(&Value::from(42)));
		assert!(!object.contains_key("author_id"));
	}

	#[cfg(not(feature = "migrations"))]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn orm_only_fixture_many_to_many_specs_use_relation_metadata() {
		let _registry_reset = FixtureRegistryReset;
		global_fixture_registry().clear();
		global_fixture_registry().register_model::<FixtureOrmOnlyM2mTag>();

		let spec = many_to_many_specs_for::<FixtureOrmOnlyM2mPost>()
			.expect("ORM-only many-to-many relations must resolve targets")
			.into_iter()
			.next()
			.expect("ORM-only many-to-many relations must produce fixture metadata");

		assert_eq!(spec.through_table, "fixture_orm_only_m2m_post_tags");
		assert_eq!(spec.source_field, "fixture_orm_only_m2m_post_id");
		assert_eq!(spec.target_field, "fixture_orm_only_custom_tags_id");
	}

	#[cfg(not(feature = "migrations"))]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn orm_only_fixture_many_to_many_assignments_are_extracted() {
		let _registry_reset = FixtureRegistryReset;
		global_fixture_registry().clear();
		global_fixture_registry().register_model::<FixtureOrmOnlyM2mTag>();
		let mut object = Map::from_iter([("tags".to_string(), Value::Array(vec![Value::from(1)]))]);

		let assignments = extract_many_to_many_assignments::<FixtureOrmOnlyM2mPost>(&mut object)
			.expect("ORM-only many-to-many fixture fields must be extracted");

		assert_eq!(assignments.len(), 1);
		assert_eq!(assignments[0].spec.field_name, "tags");
		assert_eq!(assignments[0].values, vec![Value::from(1)]);
		assert!(object.is_empty());
	}

	#[cfg(not(feature = "migrations"))]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn orm_only_fixture_upserts_bind_text_foreign_keys_as_strings() {
		let _registry_reset = FixtureRegistryReset;
		global_fixture_registry().clear();
		global_fixture_registry().register_model::<FixtureOrmOnlyTextAuthor>();
		let author_id = "123e4567-e89b-12d3-a456-426614174000";
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("author_id".to_string(), Value::from(author_id));

		let (_, values) = build_fixture_upsert_sql_values::<FixtureOrmOnlyTextForeignKeyPost>(
			DatabaseBackend::Postgres,
			&object,
		)
		.expect("ORM-only text foreign keys must build fixture statements");

		assert!(values.contains(&QueryValue::String(author_id.to_string())));
		assert!(
			!values
				.iter()
				.any(|value| matches!(value, QueryValue::Uuid(_) | QueryValue::Timestamp(_))),
			"ORM-only text foreign keys must not be rebound as UUID or timestamp parameters"
		);
	}

	#[cfg(not(feature = "migrations"))]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn orm_only_fixture_order_places_foreign_key_targets_first() {
		let _registry_reset = FixtureRegistryReset;
		global_fixture_registry().clear();
		global_fixture_registry().register_model::<FixtureOrmOnlyTextAuthor>();
		global_fixture_registry().register_model::<FixtureOrmOnlyTextForeignKeyPost>();
		let author_id = "author-1";
		let mut post_fields = Map::new();
		post_fields.insert("author".to_string(), Value::from(author_id));
		let records = vec![
			FixtureRecord::new(
				"fixture_orm_only.FixtureOrmOnlyTextForeignKeyPost",
				Some(Value::from(1)),
				post_fields,
			),
			FixtureRecord::new(
				"fixture_orm_only.FixtureOrmOnlyTextAuthor",
				Some(Value::from(author_id)),
				Map::new(),
			),
		];

		let ordered = order_records_by_dependencies(&records)
			.expect("ORM-only fixture records must be ordered by foreign keys");

		assert_eq!(
			ordered[0].model,
			"fixture_orm_only.FixtureOrmOnlyTextAuthor"
		);
		assert_eq!(
			ordered[1].model,
			"fixture_orm_only.FixtureOrmOnlyTextForeignKeyPost"
		);
	}

	#[cfg(not(feature = "migrations"))]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn orm_only_fixture_order_uses_primary_key_foreign_key_fixture_pk() {
		let _registry_reset = FixtureRegistryReset;
		global_fixture_registry().clear();
		global_fixture_registry().register_model::<FixtureOrmOnlyPrimaryKeyForeignKeyParent>();
		global_fixture_registry().register_model::<FixtureOrmOnlyPrimaryKeyForeignKeyChild>();
		let records = vec![
			FixtureRecord::new(
				"fixture_orm_only_primary_key_foreign_key.FixtureOrmOnlyPrimaryKeyForeignKeyChild",
				Some(Value::from(1)),
				Map::new(),
			),
			FixtureRecord::new(
				"fixture_orm_only_primary_key_foreign_key.FixtureOrmOnlyPrimaryKeyForeignKeyParent",
				Some(Value::from(1)),
				Map::new(),
			),
		];

		let ordered = order_records_by_dependencies(&records)
			.expect("ORM-only primary-key foreign keys must use fixture pk values");

		assert_eq!(
			ordered[0].model,
			"fixture_orm_only_primary_key_foreign_key.FixtureOrmOnlyPrimaryKeyForeignKeyParent"
		);
		assert_eq!(
			ordered[1].model,
			"fixture_orm_only_primary_key_foreign_key.FixtureOrmOnlyPrimaryKeyForeignKeyChild"
		);
	}

	#[cfg(not(feature = "migrations"))]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn orm_only_fixture_order_rejects_ambiguous_related_model_names() {
		let _registry_reset = FixtureRegistryReset;
		global_fixture_registry().clear();
		global_fixture_registry().register_model::<fixture_orm_only_ambiguous_auth::User>();
		global_fixture_registry().register_model::<fixture_orm_only_ambiguous_blog::User>();
		global_fixture_registry().register_model::<FixtureOrmOnlyAmbiguousPost>();
		let mut post_fields = Map::new();
		post_fields.insert("author".to_string(), Value::from(1));
		let records = vec![FixtureRecord::new(
			"fixture_orm_only_ambiguous_blog.FixtureOrmOnlyAmbiguousPost",
			Some(Value::from(1)),
			post_fields,
		)];

		let error = order_records_by_dependencies(&records)
			.expect_err("ambiguous ORM-only related models must not select a target by source app");

		assert!(error.to_string().contains("ambiguous model 'User'"));
	}

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
	struct FixtureTextM2mPost {
		id: Option<String>,
		title: String,
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureRegisteredM2mSource {
		id: Option<i64>,
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureRegisteredTextTarget {
		id: Option<String>,
	}

	#[cfg(feature = "migrations")]
	macro_rules! impl_registered_m2m_model {
		($model:ty, $pk:ty, $table:literal, $app:literal, $field_type:literal) => {
			impl Model for $model {
				type PrimaryKey = $pk;
				type Fields = FixturePostFields;
				type Objects = Manager<Self>;

				fn table_name() -> &'static str {
					$table
				}
				fn new_fields() -> Self::Fields {
					FixturePostFields
				}
				fn app_label() -> &'static str {
					$app
				}
				fn primary_key_field() -> &'static str {
					"id"
				}
				fn primary_key(&self) -> Option<Self::PrimaryKey> {
					self.id.clone()
				}
				fn set_primary_key(&mut self, value: Self::PrimaryKey) {
					self.id = Some(value);
				}
				fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
					let mut id = fixture_field_info("id", $field_type, false);
					id.primary_key = true;
					vec![id]
				}
			}
		};
	}

	#[cfg(feature = "migrations")]
	impl_registered_m2m_model!(
		FixtureRegisteredM2mSource,
		i64,
		"fixture_registered_m2m_source",
		"fixture_registered_m2m",
		"BigIntegerField"
	);

	#[cfg(feature = "migrations")]
	impl_registered_m2m_model!(
		FixtureRegisteredTextTarget,
		String,
		"fixture_registered_text_target",
		"fixture_registered_m2m",
		"CharField"
	);

	#[cfg(feature = "migrations")]
	impl Model for FixtureTextM2mPost {
		type PrimaryKey = String;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_text_m2m_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_text_m2m"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id.clone()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn field_metadata() -> Vec<crate::orm::inspection::FieldInfo> {
			let mut id = fixture_field_info("id", "CharField", false);
			id.primary_key = true;
			vec![id]
		}
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureBinaryPost {
		id: Option<i64>,
		payload: Vec<u8>,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureBinaryPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_binary_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_binary"
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
			vec![id, fixture_field_info("payload", "BinaryField", false)]
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
	struct FixtureMigrationMetadataFallbackPost {
		id: Option<i64>,
		author_id: i64,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureMigrationMetadataFallbackPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_migration_metadata_fallback_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_migration_metadata_fallback"
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
				fixture_field_info("author_id", "BigIntegerField", false),
			]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"author",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"FixtureMigrationMetadataFallbackAuthor",
				)
				.with_foreign_key("author_id"),
			]
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
	struct FixtureNonPrimaryIdentityAlwaysPost {
		id: Option<i64>,
		sequence_number: Option<i64>,
		payload: String,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureNonPrimaryIdentityAlwaysPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_non_primary_identity_always_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_non_primary_identity_always"
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
			let mut sequence_number =
				fixture_field_info("sequence_number", "BigIntegerField", true);
			sequence_number.attributes.insert(
				"identity_by_default".to_string(),
				crate::orm::fields::FieldKwarg::Bool(true),
			);
			vec![
				id,
				sequence_number,
				fixture_field_info("payload", "CharField", false),
			]
		}
	}

	#[cfg(feature = "migrations")]
	#[derive(Clone, Serialize, Deserialize)]
	struct FixtureTextForeignKeyPost {
		id: Option<i64>,
		author_id: String,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureTextForeignKeyPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_text_foreign_key_post"
		}

		fn new_fields() -> Self::Fields {
			FixturePostFields
		}

		fn app_label() -> &'static str {
			"fixture_text_foreign_key"
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
			vec![id, fixture_field_info("author_id", "IntegerField", false)]
		}

		fn relationship_metadata() -> Vec<crate::orm::inspection::RelationInfo> {
			vec![
				crate::orm::inspection::RelationInfo::new(
					"author",
					crate::orm::relationship::RelationshipType::ManyToOne,
					"FixtureTextForeignKeyAuthor",
				)
				.with_foreign_key("author_id"),
			]
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
	struct FixtureDefaultedPost {
		id: Option<i64>,
		status: Option<String>,
	}

	#[cfg(feature = "migrations")]
	impl Model for FixtureDefaultedPost {
		type PrimaryKey = i64;
		type Fields = FixturePostFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"fixture_defaulted_post"
		}
		fn new_fields() -> Self::Fields {
			FixturePostFields
		}
		fn app_label() -> &'static str {
			"fixture_defaulted"
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
			let mut status = fixture_field_info("status", "CharField", false);
			status.db_default = Some(crate::orm::fields::FieldKwarg::String(
				"pending".to_string(),
			));
			vec![id, status]
		}
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
	#[serial_test::serial(fixture_model_registry)]
	fn dependency_order_uses_referenced_foreign_key_columns() {
		let mut author = crate::migrations::ModelMetadata::new(
			"fixture_referenced_slug",
			"Author",
			"fixture_referenced_slug_author",
		);
		author.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("primary_key", "true"),
		);
		author.add_field(
			"slug".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::VarChar(64))
				.with_param("unique", "true"),
		);
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_referenced_slug",
			"Post",
			"fixture_referenced_slug_post",
		);
		post.add_field(
			"author_slug".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::VarChar(64))
				.with_param("fk_target", "Author")
				.with_param("fk_target_app", "fixture_referenced_slug")
				.with_foreign_key(crate::migrations::ForeignKeyInfo {
					referenced_table: "fixture_referenced_slug_author".to_string(),
					referenced_column: "slug".to_string(),
					on_delete: crate::migrations::ForeignKeyAction::Cascade,
					on_update: crate::migrations::ForeignKeyAction::Cascade,
				}),
		);
		crate::migrations::model_registry::global_registry().register_model(author);
		crate::migrations::model_registry::global_registry().register_model(post);

		let mut post_fields = Map::new();
		post_fields.insert("author_slug".to_string(), Value::from("author-one"));
		let mut author_fields = Map::new();
		author_fields.insert("slug".to_string(), Value::from("author-one"));
		let records = vec![
			FixtureRecord::new(
				"fixture_referenced_slug.Post",
				Some(Value::from(1)),
				post_fields,
			),
			FixtureRecord::new(
				"fixture_referenced_slug.Author",
				Some(Value::from(7)),
				author_fields,
			),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_referenced_slug.Author");
		assert_eq!(ordered[1].model, "fixture_referenced_slug.Post");
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn dependency_order_maps_referenced_database_columns_to_fixture_fields() {
		let mut target = crate::migrations::ModelMetadata::new(
			"fixture_database_column",
			"FixtureDatabaseColumnPost",
			"fixture_database_column_post",
		);
		target.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("primary_key", "true"),
		);
		target.add_field(
			"title".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::VarChar(64))
				.with_param("unique", "true"),
		);
		let mut source = crate::migrations::ModelMetadata::new(
			"fixture_database_column_fk",
			"Post",
			"fixture_database_column_fk_post",
		);
		source.add_field(
			"target_title".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::VarChar(64))
				.with_param("fk_target", "FixtureDatabaseColumnPost")
				.with_param("fk_target_app", "fixture_database_column")
				.with_foreign_key(crate::migrations::ForeignKeyInfo {
					referenced_table: "fixture_database_column_post".to_string(),
					referenced_column: "fixture_title".to_string(),
					on_delete: crate::migrations::ForeignKeyAction::Cascade,
					on_update: crate::migrations::ForeignKeyAction::Cascade,
				}),
		);
		crate::migrations::model_registry::global_registry().register_model(target);
		crate::migrations::model_registry::global_registry().register_model(source);
		global_fixture_registry().register_model::<FixtureDatabaseColumnPost>();

		let mut source_fields = Map::new();
		source_fields.insert("target_title".to_string(), Value::from("fixture target"));
		let mut target_fields = Map::new();
		target_fields.insert("title".to_string(), Value::from("fixture target"));
		let records = vec![
			FixtureRecord::new(
				"fixture_database_column_fk.Post",
				Some(Value::from(1)),
				source_fields,
			),
			FixtureRecord::new(
				"fixture_database_column.FixtureDatabaseColumnPost",
				Some(Value::from(2)),
				target_fields,
			),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(
			ordered[0].model,
			"fixture_database_column.FixtureDatabaseColumnPost"
		);
		assert_eq!(ordered[1].model, "fixture_database_column_fk.Post");
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
	fn migration_fixture_fields_fall_back_to_relationship_metadata() {
		let mut object = Map::new();
		object.insert("author".to_string(), Value::from(7));

		normalize_foreign_key_fixture_fields::<FixtureMigrationMetadataFallbackPost>(&mut object)
			.expect(
				"manual models must normalize fixture relation names without migration metadata",
			);

		assert_eq!(object.get("author_id"), Some(&Value::from(7)));
		assert!(object.get("author").is_none());
		assert_eq!(
			fixture_database_object::<FixtureMigrationMetadataFallbackPost>(&object)
				.expect("normalized relation names must map to database columns")
				.get("author_id"),
			Some(&Value::from(7))
		);

		denormalize_foreign_key_fixture_fields::<FixtureMigrationMetadataFallbackPost>(&mut object)
			.expect(
				"manual models must denormalize fixture relation names without migration metadata",
			);

		assert_eq!(object.get("author"), Some(&Value::from(7)));
		assert!(object.get("author_id").is_none());
		assert_eq!(
			TypedFixtureModel::<FixtureMigrationMetadataFallbackPost>::new()
				.fixture_foreign_key_fields(),
			vec![("author_id".to_string(), "author".to_string())]
		);
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
	fn fixture_default_only_inserts_use_database_defaults() {
		let (sql, values) = build_fixture_upsert_sql_values::<FixtureTextPost>(
			DatabaseBackend::Postgres,
			&Map::new(),
		)
		.expect("an empty fixture projection must be inserted with database defaults");

		assert_eq!(sql, "INSERT INTO \"fixture_text_post\" DEFAULT VALUES");
		assert!(values.is_empty());

		let (sql, values) =
			build_fixture_upsert_sql_values::<FixtureTextPost>(DatabaseBackend::MySql, &Map::new())
				.expect("MySQL must accept default-only fixture inserts");
		assert_eq!(sql, "INSERT INTO `fixture_text_post` () VALUES ()");
		assert!(values.is_empty());
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_upserts_require_omitted_database_defaults() {
		let object = Map::from_iter([(String::from("id"), Value::from(1))]);

		let error = build_fixture_upsert_sql_values::<FixtureDefaultedPost>(
			DatabaseBackend::Postgres,
			&object,
		)
		.expect_err("upserts must not leave existing database-default values unchanged");

		assert!(error.to_string().contains("status"));
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
	fn mysql_fixture_updates_clear_omitted_nullable_columns() {
		let object = Map::from_iter([(String::from("id"), Value::from(1))]);

		let (sql, values) = build_fixture_update_sql_values::<FixtureSerdeMappedPost>(
			DatabaseBackend::MySql,
			&object,
		)
		.unwrap()
		.expect("an omitted nullable column must produce an update");

		assert!(sql.contains("`fixture_note` = NULL"));
		assert_eq!(values, vec![QueryValue::Int(1)]);
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
	fn postgres_fixture_upserts_do_not_override_non_primary_identity_by_default_columns() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("sequence_number".to_string(), Value::from(99));
		object.insert("payload".to_string(), Value::from("fixture payload"));

		let (sql, values) = build_fixture_upsert_sql_values::<FixtureNonPrimaryIdentityAlwaysPost>(
			DatabaseBackend::Postgres,
			&object,
		)
		.unwrap();

		assert!(!sql.contains("OVERRIDING SYSTEM VALUE"));
		assert!(sql.contains("ON CONFLICT"));
		assert_eq!(values.len(), 3);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_sequence_reset_targets_include_written_non_primary_identity_by_default_columns() {
		let registry = FixtureRegistry::new();
		registry.register_model::<FixtureNonPrimaryIdentityAlwaysPost>();
		let mut fields = Map::new();
		fields.insert("sequence_number".to_string(), Value::from(99));
		fields.insert("payload".to_string(), Value::from("fixture payload"));
		let records = vec![FixtureRecord::new(
			"fixture_non_primary_identity_always.FixtureNonPrimaryIdentityAlwaysPost",
			Some(Value::from(1)),
			fields,
		)];

		let targets = fixture_identity_sequence_reset_targets(&registry, &records)
			.expect("written identity-by-default columns must select sequence reset targets");
		let targets = targets
			.into_iter()
			.map(|(handler, column)| (handler.label(), column))
			.collect::<Vec<_>>();

		assert_eq!(
			targets,
			vec![
				(
					"fixture_non_primary_identity_always.FixtureNonPrimaryIdentityAlwaysPost"
						.to_string(),
					"id".to_string(),
				),
				(
					"fixture_non_primary_identity_always.FixtureNonPrimaryIdentityAlwaysPost"
						.to_string(),
					"sequence_number".to_string(),
				),
			]
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn fixture_sequence_reset_ignores_many_to_many_fields() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_non_primary_identity_always",
			"FixtureNonPrimaryIdentityAlwaysPost",
			"fixture_non_primary_identity_always_post",
		);
		post.add_many_to_many(crate::migrations::ManyToManyMetadata::new(
			"tags",
			"FixtureSequenceResetTag",
		));
		crate::migrations::model_registry::global_registry().register_model(post);

		let registry = FixtureRegistry::new();
		registry.register_model::<FixtureNonPrimaryIdentityAlwaysPost>();
		let mut fields = Map::new();
		fields.insert("sequence_number".to_string(), Value::from(99));
		fields.insert("payload".to_string(), Value::from("fixture payload"));
		fields.insert("tags".to_string(), Value::Array(vec![Value::from(1)]));
		let records = vec![FixtureRecord::new(
			"fixture_non_primary_identity_always.FixtureNonPrimaryIdentityAlwaysPost",
			Some(Value::from(1)),
			fields,
		)];

		let targets = fixture_identity_sequence_reset_targets(&registry, &records)
			.expect("many-to-many fixture fields must not block sequence reset planning");
		let targets = targets
			.into_iter()
			.map(|(handler, column)| (handler.label(), column))
			.collect::<Vec<_>>();

		assert_eq!(
			targets,
			vec![
				(
					"fixture_non_primary_identity_always.FixtureNonPrimaryIdentityAlwaysPost"
						.to_string(),
					"id".to_string(),
				),
				(
					"fixture_non_primary_identity_always.FixtureNonPrimaryIdentityAlwaysPost"
						.to_string(),
					"sequence_number".to_string(),
				),
			]
		);
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
	#[serial_test::serial(fixture_model_registry)]
	fn fixture_upserts_bind_generated_text_foreign_keys_as_strings() {
		let mut author = crate::migrations::ModelMetadata::new(
			"fixture_text_foreign_key",
			"FixtureTextForeignKeyAuthor",
			"fixture_text_foreign_key_author",
		);
		author.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::Char(36))
				.with_param("primary_key", "true"),
		);
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_text_foreign_key",
			"FixtureTextForeignKeyPost",
			"fixture_text_foreign_key_post",
		);
		post.add_field(
			"author_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_foreign_key(crate::migrations::ForeignKeyInfo {
					referenced_table: "fixture_text_foreign_key_author".to_string(),
					referenced_column: "id".to_string(),
					on_delete: crate::migrations::ForeignKeyAction::Cascade,
					on_update: crate::migrations::ForeignKeyAction::Cascade,
				}),
		);
		crate::migrations::model_registry::global_registry().register_model(author);
		crate::migrations::model_registry::global_registry().register_model(post);

		let author_id = "123e4567-e89b-12d3-a456-426614174000";
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("author_id".to_string(), Value::from(author_id));

		let (_, values) = build_fixture_upsert_sql_values::<FixtureTextForeignKeyPost>(
			DatabaseBackend::Postgres,
			&object,
		)
		.unwrap();

		assert!(values.contains(&QueryValue::String(author_id.to_string())));
		assert!(
			!values
				.iter()
				.any(|value| matches!(value, QueryValue::Uuid(_))),
			"generated text foreign keys must not be rebound as UUID parameters"
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_binary_values_round_trip_between_dump_and_load_bindings() {
		let mut row = Map::new();
		row.insert("id".to_string(), Value::from(1));
		row.insert("payload".to_string(), Value::from("AAECA/8="));

		let fields = fixture_fields_from_database_row::<FixtureBinaryPost>(&row)
			.expect("binary query values should be representable in fixture JSON");
		assert_eq!(fields.get("payload"), Some(&Value::from("AAECA/8=")));

		let (_, values) = build_fixture_upsert_sql_values::<FixtureBinaryPost>(
			DatabaseBackend::Postgres,
			&fields,
		)
		.expect("binary fixture values should bind as bytes");
		assert!(values.contains(&QueryValue::Bytes(vec![0, 1, 2, 3, 255])));
	}

	#[cfg(feature = "migrations")]
	#[test]
	fn fixture_null_fill_skips_nullable_identity_columns() {
		let mut object = Map::new();
		object.insert("id".to_string(), Value::from(1));
		object.insert("payload".to_string(), Value::from("fixture payload"));

		let (insert_sql, insert_values) = build_fixture_upsert_sql_values::<
			FixtureNonPrimaryIdentityAlwaysPost,
		>(DatabaseBackend::Postgres, &object)
		.expect("fixture inserts should omit absent identity columns");
		assert!(!insert_sql.contains("sequence_number"));
		assert_eq!(insert_values.len(), 2);

		let (update_sql, update_values) = build_fixture_update_sql_values::<
			FixtureNonPrimaryIdentityAlwaysPost,
		>(DatabaseBackend::MySql, &object)
		.expect("fixture updates should omit absent identity columns")
		.expect("fixture updates should contain the payload column");
		assert!(!update_sql.contains("sequence_number"));
		assert_eq!(update_values.len(), 2);
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn many_to_many_fixture_keys_bind_text_primary_keys_as_strings() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_text_m2m",
			"FixtureTextM2mPost",
			"fixture_text_m2m_post",
		);
		post.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::Char(36))
				.with_param("primary_key", "true"),
		);
		post.add_many_to_many(crate::migrations::ManyToManyMetadata::new(
			"tags",
			"FixtureTextM2mTag",
		));
		let mut tag = crate::migrations::ModelMetadata::new(
			"fixture_text_m2m",
			"FixtureTextM2mTag",
			"fixture_text_m2m_tag",
		);
		tag.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::Char(36))
				.with_param("primary_key", "true"),
		);
		crate::migrations::model_registry::global_registry().register_model(post);
		crate::migrations::model_registry::global_registry().register_model(tag);

		let spec = many_to_many_specs_for::<FixtureTextM2mPost>()
			.expect("many-to-many fixture metadata must resolve targets")
			.into_iter()
			.next()
			.expect("registered many-to-many relation should produce fixture metadata");
		assert_eq!(
			spec.source_primary_key_binding,
			FixturePrimaryKeyBinding::Text
		);
		assert_eq!(
			spec.target_primary_key_binding,
			FixturePrimaryKeyBinding::Text
		);

		let uuid_like = Value::from("123e4567-e89b-12d3-a456-426614174000");
		let timestamp_like = Value::from("2026-07-13T08:25:26+00:00");
		let source_value = fixture_many_to_many_key_value::<FixtureTextM2mPost>(
			&uuid_like,
			spec.source_primary_key_binding,
		);
		let target_value = fixture_many_to_many_key_value::<FixtureTextM2mPost>(
			&timestamp_like,
			spec.target_primary_key_binding,
		);

		assert_eq!(
			Manager::<FixtureTextM2mPost>::sea_value_to_query_value(source_value),
			QueryValue::String("123e4567-e89b-12d3-a456-426614174000".to_string())
		);
		assert_eq!(
			Manager::<FixtureTextM2mPost>::sea_value_to_query_value(target_value),
			QueryValue::String("2026-07-13T08:25:26+00:00".to_string())
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn many_to_many_fixture_specs_resolve_qualified_cross_app_targets() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_text_m2m",
			"FixtureTextM2mPost",
			"fixture_text_m2m_post",
		);
		post.add_many_to_many(crate::migrations::ManyToManyMetadata::new(
			"tags",
			"fixture_external_m2m.FixtureTextM2mTag",
		));
		let mut local_tag = crate::migrations::ModelMetadata::new(
			"fixture_text_m2m",
			"FixtureTextM2mTag",
			"fixture_text_m2m_tag",
		);
		local_tag.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("primary_key", "true"),
		);
		let mut external_tag = crate::migrations::ModelMetadata::new(
			"fixture_external_m2m",
			"FixtureTextM2mTag",
			"fixture_external_m2m_tag",
		);
		external_tag.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::Char(36))
				.with_param("primary_key", "true"),
		);
		crate::migrations::model_registry::global_registry().register_model(post);
		crate::migrations::model_registry::global_registry().register_model(local_tag);
		crate::migrations::model_registry::global_registry().register_model(external_tag);

		let spec = many_to_many_specs_for::<FixtureTextM2mPost>()
			.expect("many-to-many fixture metadata must resolve targets")
			.into_iter()
			.next()
			.expect("qualified many-to-many target should resolve metadata");

		assert_eq!(spec.target_field, "fixture_external_m2m_tag_id");
		assert_eq!(
			spec.target_primary_key_binding,
			FixturePrimaryKeyBinding::Text
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn many_to_many_fixture_specs_use_registered_target_binding_without_metadata() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_registered_m2m",
			"FixtureRegisteredM2mSource",
			"fixture_registered_m2m_source",
		);
		post.add_many_to_many(crate::migrations::ManyToManyMetadata::new(
			"tags",
			"fixture_registered_m2m.FixtureRegisteredTextTarget",
		));
		crate::migrations::model_registry::global_registry().register_model(post);
		global_fixture_registry().register_model::<FixtureRegisteredTextTarget>();

		let spec = many_to_many_specs_for::<FixtureRegisteredM2mSource>()
			.expect("registered many-to-many target must resolve")
			.into_iter()
			.next()
			.expect("registered many-to-many target should produce fixture metadata");

		assert_eq!(
			spec.target_primary_key_binding,
			FixturePrimaryKeyBinding::Text
		);
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn custom_through_many_to_many_fixture_fields_are_supported() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_m2m",
			"FixtureM2mPost",
			"fixture_m2m_post",
		);
		post.add_many_to_many(
			crate::migrations::ManyToManyMetadata::new("tags", "Tag")
				.with_through("fixture_custom_m2m_post_tags")
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

		let assignments = extract_many_to_many_assignments::<FixtureM2mPost>(&mut object)
			.expect("custom through table names must not imply an explicit through model");

		assert_eq!(assignments.len(), 1);
		assert_eq!(object.get("title"), Some(&Value::from("Fixture")));
		assert!(object.get("tags").is_none());
	}

	#[cfg(feature = "migrations")]
	#[test]
	#[serial_test::serial(fixture_model_registry)]
	fn explicit_through_many_to_many_fixture_fields_are_rejected() {
		let mut post = crate::migrations::ModelMetadata::new(
			"fixture_m2m",
			"FixtureM2mPost",
			"fixture_m2m_post",
		);
		post.add_many_to_many(
			crate::migrations::ManyToManyMetadata::new("tags", "Tag")
				.with_through("fixture_explicit_m2m_post_tags")
				.with_source_field("post_id")
				.with_target_field("tag_id"),
		);
		let mut through = crate::migrations::ModelMetadata::new(
			"fixture_explicit_m2m",
			"PostTag",
			"fixture_explicit_m2m_post_tags",
		);
		through.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("primary_key", "true"),
		);
		through.add_field(
			"post_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger),
		);
		through.add_field(
			"tag_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger),
		);
		through.add_field(
			"position".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::Integer),
		);
		let registry = crate::migrations::model_registry::global_registry();
		registry.register_model(post);
		registry.register_model(through);

		let mut object = Map::new();
		object.insert("title".to_string(), Value::from("Fixture"));
		object.insert(
			"tags".to_string(),
			Value::Array(vec![Value::from(1), Value::from(2)]),
		);

		let error = extract_many_to_many_assignments::<FixtureM2mPost>(&mut object)
			.expect_err("explicit-through many-to-many fixture fields must be rejected");

		assert!(error.to_string().contains("explicit through"));
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
	fn dependency_order_uses_resolved_app_qualified_target_metadata() {
		let mut parent = crate::migrations::ModelMetadata::new(
			"fixture_qualified_parent",
			"Parent",
			"fixture_qualified_parent",
		);
		parent.add_field(
			"id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("primary_key", "true"),
		);
		let mut child = crate::migrations::ModelMetadata::new(
			"fixture_qualified_child",
			"Child",
			"fixture_qualified_child",
		);
		child.add_field(
			"parent_id".to_string(),
			crate::migrations::FieldMetadata::new(crate::migrations::FieldType::BigInteger)
				.with_param("fk_target", "fixture_qualified_parent.Parent"),
		);
		crate::migrations::model_registry::global_registry().register_model(parent);
		crate::migrations::model_registry::global_registry().register_model(child);

		let mut child_fields = Map::new();
		child_fields.insert("parent".to_string(), Value::from(1));
		let records = vec![
			FixtureRecord::new(
				"fixture_qualified_child.Child",
				Some(Value::from(1)),
				child_fields,
			),
			FixtureRecord::new(
				"fixture_qualified_parent.Parent",
				Some(Value::from(1)),
				Map::new(),
			),
		];

		let ordered = order_records_by_dependencies(&records).unwrap();

		assert_eq!(ordered[0].model, "fixture_qualified_parent.Parent");
		assert_eq!(ordered[1].model, "fixture_qualified_child.Child");
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
