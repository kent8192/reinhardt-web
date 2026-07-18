//! Nested serializer ORM integration tests
//!
//! Tests for `NestedSaveContext` and `ManyToManyManager` from reinhardt-rest.

use reinhardt_core::exception::Result;
use reinhardt_db::orm::{
	DatabaseBackend, FieldSelector, Manager, Model, OrmExecutor, QueryResult, QueryValue, Row,
};
use reinhardt_rest::serializers::nested_orm::{ManyToManyManager, NestedSaveContext};
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct RecordingExecutor {
	statements: Vec<RecordedStatement>,
}

#[derive(Debug, PartialEq)]
struct RecordedStatement {
	sql: String,
	params: Vec<QueryValue>,
}

#[async_trait::async_trait]
impl OrmExecutor for RecordingExecutor {
	fn backend(&self) -> DatabaseBackend {
		DatabaseBackend::Sqlite
	}

	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		self.statements.push(RecordedStatement {
			sql: sql.to_string(),
			params,
		});
		Ok(QueryResult {
			rows_affected: 1,
			last_insert_id: None,
		})
	}

	async fn fetch_one(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
		Ok(Row::new())
	}

	async fn fetch_all(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
		Ok(Vec::new())
	}

	async fn fetch_optional(
		&mut self,
		_sql: &str,
		_params: Vec<QueryValue>,
	) -> Result<Option<Row>> {
		Ok(None)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestUser {
	id: Option<i64>,
	name: String,
}

reinhardt_test::impl_test_model!(TestUser, i64, "users");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestGroup {
	id: Option<i64>,
	name: String,
}

reinhardt_test::impl_test_model!(TestGroup, i64, "groups");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StringKeyUser {
	id: Option<String>,
	name: String,
}

#[derive(Clone)]
struct StringKeyUserFields;

impl FieldSelector for StringKeyUserFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for StringKeyUser {
	type PrimaryKey = String;
	type Fields = StringKeyUserFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"string_key_users"
	}

	fn new_fields() -> Self::Fields {
		StringKeyUserFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id.clone()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StringKeyGroup {
	id: Option<String>,
	name: String,
}

#[derive(Clone)]
struct StringKeyGroupFields;

impl FieldSelector for StringKeyGroupFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for StringKeyGroup {
	type PrimaryKey = String;
	type Fields = StringKeyGroupFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"string_key_groups"
	}

	fn new_fields() -> Self::Fields {
		StringKeyGroupFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id.clone()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[test]
fn test_nested_save_context_creation() {
	let context = NestedSaveContext::new();
	assert_eq!(context.depth, 0);
	assert_eq!(context.max_depth, 10);
	assert!(context.parent_data.is_empty());
}

#[test]
fn test_nested_save_context_with_parent_data() {
	let context = NestedSaveContext::new()
		.with_parent_data("user_id".to_string(), serde_json::json!(123))
		.with_parent_data("tenant_id".to_string(), serde_json::json!(456));

	assert_eq!(context.parent_data.len(), 2);
	assert_eq!(
		context.get_parent_value("user_id").unwrap(),
		&serde_json::json!(123)
	);
	assert_eq!(
		context.get_parent_value("tenant_id").unwrap(),
		&serde_json::json!(456)
	);
}

#[test]
fn test_nested_save_context_depth_limit() {
	let mut context = NestedSaveContext::new().with_max_depth(2);

	assert!(context.increment_depth().is_ok());
	assert_eq!(context.depth, 1);

	assert!(context.increment_depth().is_ok());
	assert_eq!(context.depth, 2);

	// Exceeds limit
	let result = context.increment_depth();
	assert!(result.is_err());
}

#[test]
fn test_nested_save_context_child_context() {
	let parent = NestedSaveContext::new()
		.with_parent_data("key".to_string(), serde_json::json!("value"))
		.with_max_depth(5);

	let child = parent.child_context().unwrap();
	assert_eq!(child.depth, 1);
	assert_eq!(child.max_depth, 5);
	assert_eq!(child.parent_data.len(), 1);
	assert_eq!(
		child.get_parent_value("key").unwrap(),
		&serde_json::json!("value")
	);
}

#[test]
fn test_nested_save_context_child_exceeds_depth() {
	let mut parent = NestedSaveContext::new().with_max_depth(1);
	parent.depth = 1;

	let result = parent.child_context();
	assert!(result.is_err());
}

#[test]
fn test_many_to_many_manager_creation() {
	let manager =
		ManyToManyManager::<TestUser, TestGroup>::new("user_groups", "user_id", "group_id");

	assert_eq!(manager.junction_table, "user_groups");
	assert_eq!(manager.source_fk, "user_id");
	assert_eq!(manager.target_fk, "group_id");
}

#[tokio::test]
async fn test_many_to_many_manager_with_conn_uses_caller_owned_executor() {
	let manager =
		ManyToManyManager::<TestUser, TestGroup>::new("user_groups", "user_id", "group_id");
	let mut executor = RecordingExecutor::default();

	manager
		.add_bulk_with_conn(&mut executor, &1, vec![2, 3])
		.await
		.unwrap();
	manager
		.remove_bulk_with_conn(&mut executor, &1, vec![2])
		.await
		.unwrap();
	manager.clear_with_conn(&mut executor, &1).await.unwrap();
	manager
		.set_with_conn(&mut executor, &1, vec![4])
		.await
		.unwrap();

	assert_eq!(executor.statements.len(), 5);
	assert_eq!(
		executor.statements[0].params,
		vec![
			QueryValue::Int(1),
			QueryValue::Int(2),
			QueryValue::Int(1),
			QueryValue::Int(3),
		]
	);
	assert_eq!(
		executor.statements[1].params,
		vec![QueryValue::Int(1), QueryValue::Int(2)]
	);
	assert_eq!(executor.statements[2].params, vec![QueryValue::Int(1)]);
	assert_eq!(executor.statements[3].params, vec![QueryValue::Int(1)]);
	assert_eq!(
		executor.statements[4].params,
		vec![QueryValue::Int(1), QueryValue::Int(4)]
	);
}

#[tokio::test]
async fn test_many_to_many_manager_with_conn_binds_string_primary_keys() {
	let manager = ManyToManyManager::<StringKeyUser, StringKeyGroup>::new(
		"user_groups",
		"user_id",
		"group_id",
	);
	let source_id = "user' OR 1 = 1 --".to_string();
	let target_id = "550e8400-e29b-41d4-a716-446655440000' OR 1 = 1 --".to_string();
	let mut executor = RecordingExecutor::default();

	manager
		.add_bulk_with_conn(&mut executor, &source_id, vec![target_id.clone()])
		.await
		.unwrap();
	manager
		.remove_bulk_with_conn(&mut executor, &source_id, vec![target_id.clone()])
		.await
		.unwrap();
	manager
		.clear_with_conn(&mut executor, &source_id)
		.await
		.unwrap();

	assert_eq!(executor.statements.len(), 3);
	for statement in &executor.statements {
		assert!(!statement.sql.contains(&source_id));
		assert!(!statement.sql.contains(&target_id));
	}
	assert_eq!(
		executor.statements[0].params,
		vec![
			QueryValue::String(source_id.clone()),
			QueryValue::String(target_id.clone()),
		]
	);
	assert_eq!(
		executor.statements[1].params,
		vec![
			QueryValue::String(source_id.clone()),
			QueryValue::String(target_id),
		]
	);
	assert_eq!(
		executor.statements[2].params,
		vec![QueryValue::String(source_id)]
	);
}
