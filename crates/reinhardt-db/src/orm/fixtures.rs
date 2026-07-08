//! Django-compatible model fixture loading and dumping.
//!
//! The fixture runtime is type-erased at the registry boundary, while each
//! registered model still loads through its generated `Model` implementation
//! and `serde` validation.

use super::manager::get_connection;
use super::transaction::TransactionScope;
use super::{DatabaseConnection, Manager, Model};
use async_trait::async_trait;
use once_cell::sync::Lazy;
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
		if let Some(pk) = &record.pk {
			object.insert(M::primary_key_field().to_string(), pk.clone());
		}
		let model: M = serde_json::from_value(Value::Object(object))?;
		let manager = Manager::<M>::new();
		let (sql, values) = manager.create_insert_sql_values(conn, &model)?;
		tx.query_one(&sql, values).await?;
		Ok(())
	}

	async fn dump_records(&self) -> FixtureResult<Vec<FixtureRecord>> {
		let manager = Manager::<M>::new();
		let rows = manager.order_by(&[M::primary_key_field()]).all().await?;
		let mut records = Vec::with_capacity(rows.len());
		let model_label = self.label();
		for row in rows {
			let value = serde_json::to_value(row)?;
			let mut object = value.as_object().cloned().ok_or_else(|| {
				FixtureError::Database("model must serialize to a JSON object".to_string())
			})?;
			let pk = object.remove(M::primary_key_field());
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

	tx.commit().await?;
	Ok(ordered_records.len())
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
		.filter(|handler| handler.app_label() == selector)
		.collect();
	handlers.sort_by_key(|handler| handler.label());
	if handlers.is_empty() {
		return Err(FixtureError::SelectorMatchedNoModels(selector.to_string()));
	}
	Ok(handlers)
}

fn order_records_by_dependencies(records: &[FixtureRecord]) -> FixtureResult<Vec<FixtureRecord>> {
	let present_models: HashSet<String> = records
		.iter()
		.map(|record| canonical_record_label(&record.model))
		.collect::<FixtureResult<_>>()?;
	let mut dependencies = HashMap::<String, HashSet<String>>::new();

	for model_key in &present_models {
		let (app_label, model_name) = parse_model_label(model_key)?;
		let mut model_dependencies = HashSet::new();
		if let Some(metadata) =
			crate::migrations::model_registry::global_registry().get_model(&app_label, &model_name)
		{
			for field in metadata.fields.values() {
				if let Some(target_model) = field.params.get("fk_target") {
					let target_app = field
						.params
						.get("fk_target_app")
						.map(String::as_str)
						.unwrap_or(&app_label);
					let target_key = canonical_label(target_app, target_model);
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
	Ok(indexed_records
		.into_iter()
		.map(|(_, record)| record)
		.collect())
}

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

fn canonical_record_label(label: &str) -> FixtureResult<String> {
	let (app_label, model_name) = parse_model_label(label)?;
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
	expected_app == actual_app && expected_model.eq_ignore_ascii_case(&actual_model)
}

fn find_case_insensitive(
	models: &HashMap<String, Arc<dyn FixtureModelHandler>>,
	app_label: &str,
	model_name: &str,
) -> Option<Arc<dyn FixtureModelHandler>> {
	models
		.values()
		.find(|handler| {
			handler.app_label() == app_label
				&& handler.model_name().eq_ignore_ascii_case(model_name)
		})
		.cloned()
}

#[cfg(test)]
mod tests {
	use super::*;

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
}
