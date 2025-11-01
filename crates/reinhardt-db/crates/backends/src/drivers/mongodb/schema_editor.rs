//! MongoDB schema editor
//!
//! MongoDB is schema-less, but this module provides operations for:
//! - Collection creation and deletion
//! - Index creation and deletion
//! - Validation rules
//!
//! # Example
//!
//! ```rust,no_run
//! use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
//! use bson::doc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
//!
//! // Create collection with validation
//! editor.create_collection("users", Some(doc! {
//!     "$jsonSchema": {
//!         "required": ["name", "email"],
//!         "properties": {
//!             "name": { "bsonType": "string" },
//!             "email": { "bsonType": "string" }
//!         }
//!     }
//! })).await?;
//!
//! // Create index
//! editor.create_index("users", "idx_email", &["email"], true).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use bson::{Document, doc};
use mongodb::{
	Client, Database, IndexModel,
	options::{CreateCollectionOptions, IndexOptions},
};
use std::sync::Arc;

use crate::schema::{BaseDatabaseSchemaEditor, SchemaEditorError, SchemaEditorResult};

/// MongoDB schema editor
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
/// # Ok(())
/// # }
/// ```
pub struct MongoDBSchemaEditor {
	#[allow(dead_code)]
	client: Arc<Client>,
	database: Database,
}

impl MongoDBSchemaEditor {
	/// Create a new MongoDB schema editor
	///
	/// # Arguments
	///
	/// * `url` - MongoDB connection string
	/// * `database_name` - Database name to use
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(url: &str, database_name: &str) -> SchemaEditorResult<Self> {
		let client = Client::with_uri_str(url).await.map_err(|e| {
			SchemaEditorError::DatabaseError(format!("Failed to connect to MongoDB: {}", e))
		})?;

		let database = client.database(database_name);

		Ok(Self {
			client: Arc::new(client),
			database,
		})
	}

	/// Create a collection with optional validation rules
	///
	/// # Arguments
	///
	/// * `collection_name` - Name of the collection to create
	/// * `validator` - Optional validation rules as a BSON document
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
	///
	/// editor.create_collection("users", Some(doc! {
	///     "$jsonSchema": {
	///         "required": ["name", "email"],
	///         "properties": {
	///             "name": { "bsonType": "string" },
	///             "email": { "bsonType": "string" }
	///         }
	///     }
	/// })).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn create_collection(
		&self,
		collection_name: &str,
		validator: Option<Document>,
	) -> SchemaEditorResult<()> {
		let mut options = CreateCollectionOptions::default();
		if let Some(validator) = validator {
			options.validator = Some(validator);
		}

		self.database
			.create_collection(collection_name)
			.with_options(options)
			.await
			.map_err(|e| {
				SchemaEditorError::ExecutionError(format!(
					"Failed to create collection {}: {}",
					collection_name, e
				))
			})?;

		Ok(())
	}

	/// Drop a collection
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
	/// editor.drop_collection("users").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn drop_collection(&self, collection_name: &str) -> SchemaEditorResult<()> {
		let collection = self.database.collection::<Document>(collection_name);
		collection.drop().await.map_err(|e| {
			SchemaEditorError::ExecutionError(format!(
				"Failed to drop collection {}: {}",
				collection_name, e
			))
		})?;

		Ok(())
	}

	/// Create an index on a collection
	///
	/// # Arguments
	///
	/// * `collection_name` - Name of the collection
	/// * `index_name` - Name of the index
	/// * `fields` - Fields to index
	/// * `unique` - Whether the index should be unique
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
	/// editor.create_index("users", "idx_email", &["email"], true).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn create_index(
		&self,
		collection_name: &str,
		index_name: &str,
		fields: &[&str],
		unique: bool,
	) -> SchemaEditorResult<()> {
		let collection = self.database.collection::<Document>(collection_name);

		// Build index keys document
		let mut keys = Document::new();
		for field in fields {
			keys.insert(*field, 1); // 1 for ascending order
		}

		let mut options = IndexOptions::default();
		options.name = Some(index_name.to_string());
		options.unique = Some(unique);

		let index = IndexModel::builder().keys(keys).options(options).build();

		collection.create_index(index).await.map_err(|e| {
			SchemaEditorError::ExecutionError(format!(
				"Failed to create index {} on {}: {}",
				index_name, collection_name, e
			))
		})?;

		Ok(())
	}

	/// Drop an index from a collection
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
	/// editor.drop_index("users", "idx_email").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn drop_index(
		&self,
		collection_name: &str,
		index_name: &str,
	) -> SchemaEditorResult<()> {
		let collection = self.database.collection::<Document>(collection_name);

		collection.drop_index(index_name).await.map_err(|e| {
			SchemaEditorError::ExecutionError(format!(
				"Failed to drop index {} from {}: {}",
				index_name, collection_name, e
			))
		})?;

		Ok(())
	}

	/// List all indexes in a collection
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
	/// let indexes = editor.list_indexes("users").await?;
	/// for index in indexes {
	///     println!("Index: {:?}", index);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn list_indexes(&self, collection_name: &str) -> SchemaEditorResult<Vec<Document>> {
		use futures::stream::TryStreamExt;

		let collection = self.database.collection::<Document>(collection_name);

		let cursor = collection.list_indexes().await.map_err(|e| {
			SchemaEditorError::ExecutionError(format!(
				"Failed to list indexes for {}: {}",
				collection_name, e
			))
		})?;

		let index_models: Vec<mongodb::IndexModel> = cursor.try_collect().await.map_err(|e| {
			SchemaEditorError::ExecutionError(format!("Error reading indexes: {}", e))
		})?;

		// Convert IndexModel to Document
		let indexes: Vec<Document> = index_models
			.into_iter()
			.map(|model| {
				let mut doc = Document::new();
				doc.insert("keys", model.keys);
				if let Some(options) = model.options {
					if let Some(name) = options.name {
						doc.insert("name", name);
					}
					if let Some(unique) = options.unique {
						doc.insert("unique", unique);
					}
				}
				doc
			})
			.collect();

		Ok(indexes)
	}

	/// Update validation rules for a collection
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBSchemaEditor;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
	///
	/// editor.update_validation("users", doc! {
	///     "$jsonSchema": {
	///         "required": ["name", "email", "age"],
	///         "properties": {
	///             "name": { "bsonType": "string" },
	///             "email": { "bsonType": "string" },
	///             "age": { "bsonType": "int", "minimum": 0 }
	///         }
	///     }
	/// }).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn update_validation(
		&self,
		collection_name: &str,
		validator: Document,
	) -> SchemaEditorResult<()> {
		self.database
			.run_command(doc! {
				"collMod": collection_name,
				"validator": validator
			})
			.await
			.map_err(|e| {
				SchemaEditorError::ExecutionError(format!(
					"Failed to update validation for {}: {}",
					collection_name, e
				))
			})?;

		Ok(())
	}
}

#[async_trait]
impl BaseDatabaseSchemaEditor for MongoDBSchemaEditor {
	async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
		// MongoDB doesn't execute SQL
		// Operations should use the specific methods instead
		Err(SchemaEditorError::InvalidOperation(
			"MongoDB does not support SQL execution. Use collection and index methods instead."
				.to_string(),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_schema_editor_requires_connection() {
		// This test just verifies that the types compile correctly
		// Actual connection tests would require a running MongoDB instance
		// and should be done in integration tests
	}
}
