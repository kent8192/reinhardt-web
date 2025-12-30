//! Schema creation fixtures for tests
//!
//! This module provides low-level utilities to create database tables
//! directly from Model metadata, eliminating the need for duplicate
//! schema definitions in tests.
//!
//! ## Overview
//!
//! The main goal is to bridge the gap between:
//! - `#[model(...)]` macro definitions (which generate `Model` trait implementations)
//! - Database table creation (previously done manually with SeaQuery)
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use reinhardt_test::fixtures::schema::{create_table_for_model, create_tables_for_models};
//! use reinhardt_orm::Model;
//!
//! #[model(app_label = "test", table_name = "articles")]
//! struct Article {
//!     #[field(primary_key = true)]
//!     id: Option<i64>,
//!     #[field(max_length = 200)]
//!     title: String,
//! }
//!
//! #[rstest]
//! #[tokio::test]
//! async fn test_article(
//!     #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
//! ) {
//!     let (_, pool, _, url) = postgres_container.await;
//!     let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
//!
//!     // Create table directly from model metadata
//!     create_table_for_model::<Article>(&connection).await.unwrap();
//!
//!     // Test execution...
//! }
//! ```

use std::collections::HashMap;

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::orm::Model;
use reinhardt_db::orm::fields::FieldKwarg;
use reinhardt_db::orm::inspection::{FieldInfo, RelationInfo};
use reinhardt_db::orm::relationship::RelationshipType;
use reinhardt_migrations::{
	ColumnDefinition, Constraint, Migration, Operation, executor::DatabaseMigrationExecutor,
	field_type_string_to_field_type,
};

/// Error type for schema operations
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
	#[error("Field conversion error: {0}")]
	FieldConversion(String),

	#[error("Migration execution error: {0}")]
	MigrationExecution(String),

	#[error("Dependency resolution error: {0}")]
	DependencyResolution(String),

	#[error("Circular dependency detected: {0}")]
	CircularDependency(String),
}

/// Convert FieldKwarg HashMap to String HashMap for field_type_string_to_field_type
fn convert_attributes(attributes: &HashMap<String, FieldKwarg>) -> HashMap<String, String> {
	attributes
		.iter()
		.filter_map(|(k, v)| {
			let value_str = match v {
				FieldKwarg::String(s) => Some(s.clone()),
				FieldKwarg::Int(n) => Some(n.to_string()),
				FieldKwarg::Uint(n) => Some(n.to_string()),
				FieldKwarg::Bool(b) => Some(b.to_string()),
				FieldKwarg::Float(f) => Some(f.to_string()),
				FieldKwarg::Choices(_) => None,
				FieldKwarg::Callable(s) => Some(s.clone()),
			};
			value_str.map(|v| (k.clone(), v))
		})
		.collect()
}

/// Convert FieldInfo to ColumnDefinition
///
/// This function transforms the metadata extracted from a Model's field_metadata()
/// into a ColumnDefinition that can be used in migration operations.
///
/// # Arguments
///
/// * `field_info` - Field information from Model::field_metadata()
///
/// # Returns
///
/// * `Ok(ColumnDefinition)` - The column definition for migration
/// * `Err(SchemaError)` - Error if field type is unsupported
pub fn field_info_to_column_definition(
	field_info: &FieldInfo,
) -> Result<ColumnDefinition, SchemaError> {
	let attributes = convert_attributes(&field_info.attributes);
	let field_type = field_type_string_to_field_type(&field_info.field_type, &attributes)
		.map_err(SchemaError::FieldConversion)?;

	// Leak the name string to get 'static lifetime (acceptable for test fixtures)
	let name: &'static str = Box::leak(field_info.name.clone().into_boxed_str());

	// Handle default value
	let default: Option<&'static str> = field_info.default.as_ref().map(|d| {
		let default_str = match d {
			FieldKwarg::String(s) => format!("'{}'", s),
			FieldKwarg::Int(n) => n.to_string(),
			FieldKwarg::Uint(n) => n.to_string(),
			FieldKwarg::Bool(b) => b.to_string(),
			FieldKwarg::Float(f) => f.to_string(),
			FieldKwarg::Choices(_) => "NULL".to_string(),
			FieldKwarg::Callable(s) => s.clone(),
		};
		Box::leak(default_str.into_boxed_str()) as &'static str
	});

	// Determine auto_increment from attributes
	let auto_increment = field_info
		.attributes
		.get("auto_increment")
		.map(|v| matches!(v, FieldKwarg::Bool(true)))
		.unwrap_or(false)
		|| field_info
			.attributes
			.get("identity_by_default")
			.map(|v| matches!(v, FieldKwarg::Bool(true)))
			.unwrap_or(false)
		|| field_info.primary_key; // Auto increment for primary keys by default

	Ok(ColumnDefinition {
		name,
		type_definition: field_type,
		not_null: !field_info.nullable,
		unique: field_info.unique,
		primary_key: field_info.primary_key,
		auto_increment,
		default,
	})
}

/// Extract model dependencies from relationship metadata
///
/// This function analyzes the relationship metadata to determine which other
/// models this model depends on (via ForeignKey or OneToOne relationships).
///
/// # Arguments
///
/// * `relationship_metadata` - Relationship information from Model::relationship_metadata()
///
/// # Returns
///
/// A list of model names that this model depends on
pub fn extract_model_dependencies(relationship_metadata: &[RelationInfo]) -> Vec<String> {
	relationship_metadata
		.iter()
		.filter_map(|rel| match rel.relationship_type {
			RelationshipType::ManyToOne | RelationshipType::OneToOne => {
				Some(rel.related_model.clone())
			}
			_ => None,
		})
		.collect()
}

/// Resolve model creation order using topological sort
///
/// This function takes a list of (model_name, dependencies) pairs and returns
/// the models sorted in an order where dependencies are created before dependents.
///
/// # Arguments
///
/// * `models` - List of (model_name, Vec<dependency_names>)
///
/// # Returns
///
/// * `Ok(Vec<String>)` - Model names in creation order
/// * `Err(SchemaError)` - If circular dependency is detected
pub fn resolve_model_order(models: &[(String, Vec<String>)]) -> Result<Vec<String>, SchemaError> {
	use std::collections::{HashSet, VecDeque};

	let model_names: HashSet<String> = models.iter().map(|(name, _)| name.clone()).collect();
	let mut in_degree: HashMap<String, usize> = HashMap::new();
	let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

	// Initialize in-degree and adjacency list
	for (name, _) in models {
		in_degree.insert(name.clone(), 0);
		adjacency.insert(name.clone(), Vec::new());
	}

	// Build the graph
	for (name, deps) in models {
		for dep in deps {
			// Only count dependencies that are in our model set
			if model_names.contains(dep) {
				*in_degree.get_mut(name).unwrap() += 1;
				adjacency.get_mut(dep).unwrap().push(name.clone());
			}
		}
	}

	// Kahn's algorithm for topological sort
	let mut queue: VecDeque<String> = in_degree
		.iter()
		.filter(|&(_, &degree)| degree == 0)
		.map(|(name, _)| name.clone())
		.collect();

	let mut sorted = Vec::new();

	while let Some(node) = queue.pop_front() {
		sorted.push(node.clone());

		if let Some(neighbors) = adjacency.get(&node) {
			for neighbor in neighbors {
				if let Some(degree) = in_degree.get_mut(neighbor) {
					*degree -= 1;
					if *degree == 0 {
						queue.push_back(neighbor.clone());
					}
				}
			}
		}
	}

	// Check for circular dependency
	if sorted.len() != models.len() {
		let sorted_set: std::collections::HashSet<_> = sorted.iter().cloned().collect();
		let remaining: Vec<_> = model_names.difference(&sorted_set).collect();
		return Err(SchemaError::CircularDependency(format!(
			"Circular dependency detected involving: {:?}",
			remaining
		)));
	}

	Ok(sorted)
}

/// Model schema information for batch table creation
pub struct ModelSchemaInfo {
	/// Model name (used for dependency resolution)
	pub name: String,
	/// Table name in the database
	pub table_name: &'static str,
	/// App label for the model
	pub app_label: &'static str,
	/// Field metadata from the model
	pub fields: Vec<FieldInfo>,
	/// Relationship metadata from the model
	pub relationships: Vec<RelationInfo>,
}

impl ModelSchemaInfo {
	/// Create ModelSchemaInfo from a Model type
	pub fn from_model<M: Model>() -> Self {
		Self {
			name: std::any::type_name::<M>()
				.split("::")
				.last()
				.unwrap_or("Unknown")
				.to_string(),
			table_name: Box::leak(M::table_name().to_string().into_boxed_str()),
			app_label: Box::leak(M::app_label().to_string().into_boxed_str()),
			fields: M::field_metadata(),
			relationships: M::relationship_metadata(),
		}
	}

	/// Get dependencies for this model
	pub fn dependencies(&self) -> Vec<String> {
		extract_model_dependencies(&self.relationships)
	}
}

/// Create a CreateTable Operation from a Model type
///
/// This function extracts metadata from the Model trait implementation
/// and generates a CreateTable operation that can be executed by MigrationExecutor.
///
/// # Type Parameters
///
/// * `M` - A type implementing the Model trait
///
/// # Returns
///
/// * `Ok(Operation)` - The CreateTable operation
/// * `Err(SchemaError)` - Error if field conversion fails
pub fn create_table_operation_from_model<M: Model>() -> Result<Operation, SchemaError> {
	let table_name: &'static str = Box::leak(M::table_name().to_string().into_boxed_str());

	// Convert field metadata to column definitions
	let columns: Vec<ColumnDefinition> = M::field_metadata()
		.iter()
		.map(field_info_to_column_definition)
		.collect::<Result<Vec<_>, _>>()?;

	// For now, we don't automatically convert constraints
	// FK constraints can be added separately after all tables are created
	let constraints: Vec<Constraint> = Vec::new();

	Ok(Operation::CreateTable {
		name: table_name,
		columns,
		constraints,
	})
}

/// Create a Migration containing CreateTable for a single model
///
/// # Type Parameters
///
/// * `M` - A type implementing the Model trait
///
/// # Arguments
///
/// * `migration_name` - Name for the migration (e.g., "0001_create_users")
///
/// # Returns
///
/// * `Ok(Migration)` - The migration containing the CreateTable operation
/// * `Err(SchemaError)` - Error if operation creation fails
pub fn create_migration_from_model<M: Model>(
	migration_name: &str,
) -> Result<Migration, SchemaError> {
	let operation = create_table_operation_from_model::<M>()?;

	Ok(Migration {
		name: Box::leak(migration_name.to_string().into_boxed_str()),
		app_label: Box::leak(M::app_label().to_string().into_boxed_str()),
		operations: vec![operation],
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: Some(true),
		state_only: false,
		database_only: false,
	})
}

/// Create table operations from multiple models with dependency resolution
///
/// This function creates CreateTable operations for multiple models,
/// ordering them based on foreign key dependencies.
///
/// # Arguments
///
/// * `model_infos` - List of ModelSchemaInfo for each model
///
/// # Returns
///
/// * `Ok(Vec<Operation>)` - Operations in dependency-resolved order
/// * `Err(SchemaError)` - Error if resolution or conversion fails
pub fn create_table_operations_from_models(
	model_infos: Vec<ModelSchemaInfo>,
) -> Result<Vec<Operation>, SchemaError> {
	// Build dependency graph
	let models_with_deps: Vec<(String, Vec<String>)> = model_infos
		.iter()
		.map(|info| (info.name.clone(), info.dependencies()))
		.collect();

	// Resolve order
	let ordered_names = resolve_model_order(&models_with_deps)?;

	// Create operations in resolved order
	let name_to_info: HashMap<String, &ModelSchemaInfo> = model_infos
		.iter()
		.map(|info| (info.name.clone(), info))
		.collect();

	let mut operations = Vec::new();
	for name in ordered_names {
		if let Some(info) = name_to_info.get(&name) {
			let columns: Vec<ColumnDefinition> = info
				.fields
				.iter()
				.map(field_info_to_column_definition)
				.collect::<Result<Vec<_>, _>>()?;

			operations.push(Operation::CreateTable {
				name: info.table_name,
				columns,
				constraints: Vec::new(),
			});
		}
	}

	Ok(operations)
}

/// Create a database table for a single model
///
/// This is the main entry point for creating a table from a model definition.
/// It creates the migration and executes it against the provided database connection.
///
/// # Type Parameters
///
/// * `M` - A type implementing the Model trait
///
/// # Arguments
///
/// * `connection` - Database connection to execute the migration on
///
/// # Returns
///
/// * `Ok(())` - Table was created successfully
/// * `Err(SchemaError)` - Error during migration creation or execution
pub async fn create_table_for_model<M: Model>(
	connection: &DatabaseConnection,
) -> Result<(), SchemaError> {
	let migration = create_migration_from_model::<M>("0001_auto_create")?;

	let mut executor = DatabaseMigrationExecutor::new(connection.clone());
	executor
		.apply_migrations(&[migration])
		.await
		.map_err(|e| SchemaError::MigrationExecution(e.to_string()))?;

	Ok(())
}

/// Create database tables for multiple models with dependency resolution
///
/// This function creates tables for all provided models, automatically
/// resolving foreign key dependencies to ensure tables are created in the correct order.
///
/// # Arguments
///
/// * `connection` - Database connection to execute migrations on
/// * `model_infos` - List of ModelSchemaInfo for each model to create
///
/// # Returns
///
/// * `Ok(())` - All tables were created successfully
/// * `Err(SchemaError)` - Error during resolution, creation, or execution
pub async fn create_tables_for_models(
	connection: &DatabaseConnection,
	model_infos: Vec<ModelSchemaInfo>,
) -> Result<(), SchemaError> {
	let operations = create_table_operations_from_models(model_infos)?;

	if operations.is_empty() {
		return Ok(());
	}

	let migration = Migration {
		name: "0001_auto_batch_create",
		app_label: "test",
		operations,
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: Some(true),
		state_only: false,
		database_only: false,
	};

	let mut executor = DatabaseMigrationExecutor::new(connection.clone());
	executor
		.apply_migrations(&[migration])
		.await
		.map_err(|e| SchemaError::MigrationExecution(e.to_string()))?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_resolve_model_order_simple() {
		let models = vec![
			("User".to_string(), vec![]),
			("Post".to_string(), vec!["User".to_string()]),
			(
				"Comment".to_string(),
				vec!["Post".to_string(), "User".to_string()],
			),
		];

		let order = resolve_model_order(&models).unwrap();

		// User must come before Post and Comment
		let user_idx = order.iter().position(|n| n == "User").unwrap();
		let post_idx = order.iter().position(|n| n == "Post").unwrap();
		let comment_idx = order.iter().position(|n| n == "Comment").unwrap();

		assert!(user_idx < post_idx);
		assert!(user_idx < comment_idx);
		assert!(post_idx < comment_idx);
	}

	#[test]
	fn test_resolve_model_order_circular() {
		let models = vec![
			("A".to_string(), vec!["B".to_string()]),
			("B".to_string(), vec!["A".to_string()]),
		];

		let result = resolve_model_order(&models);
		assert!(result.is_err());
	}

	#[test]
	fn test_resolve_model_order_external_deps() {
		// Dependencies on models not in the set should be ignored
		let models = vec![
			("User".to_string(), vec!["ExternalModel".to_string()]),
			("Post".to_string(), vec!["User".to_string()]),
		];

		let order = resolve_model_order(&models).unwrap();
		assert_eq!(order.len(), 2);
	}

	#[test]
	fn test_extract_model_dependencies() {
		use reinhardt_db::orm::inspection::RelationInfo;

		let relations = vec![
			RelationInfo::new(
				"author".to_string(),
				RelationshipType::ManyToOne,
				"User".to_string(),
			),
			RelationInfo::new(
				"tags".to_string(),
				RelationshipType::ManyToMany,
				"Tag".to_string(),
			),
		];

		let deps = extract_model_dependencies(&relations);

		// Only ManyToOne creates a dependency
		assert_eq!(deps.len(), 1);
		assert!(deps.contains(&"User".to_string()));
	}
}
