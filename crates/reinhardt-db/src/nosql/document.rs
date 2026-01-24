//! Core `Document` trait definition.

use async_trait::async_trait;
use bson::Document as BsonDocument;
use serde::{de::DeserializeOwned, Serialize};

use super::error::{OdmResult, ValidationError};

/// Core trait for MongoDB documents.
///
/// This trait is typically implemented automatically via the `#[document(...)]` macro.
///
/// ## Example
///
/// ```ignore
/// #[document(collection = "users", backend = "mongodb")]
/// struct User {
///     #[field(primary_key)]
///     id: ObjectId,
///     name: String,
/// }
/// ```
#[async_trait]
pub trait Document: Serialize + DeserializeOwned + Send + Sync + 'static {
	/// Primary key type (e.g., `ObjectId`, `Uuid`, `i64`).
	type Id: Serialize + DeserializeOwned + Send + Sync;

	/// Collection name in MongoDB.
	const COLLECTION_NAME: &'static str;

	/// Database name in MongoDB.
	const DATABASE_NAME: &'static str;

	/// Get the document's ID.
	///
	/// Returns `None` if the document hasn't been persisted yet.
	fn id(&self) -> Option<&Self::Id>;

	/// Set the document's ID.
	///
	/// This is typically called after insertion.
	fn set_id(&mut self, id: Self::Id);

	/// Get index definitions for this document.
	///
	/// Generated from `#[field(index)]` and `#[field(unique)]` attributes.
	fn indexes() -> Vec<IndexModel> {
		Vec::new()
	}

	/// Get the MongoDB validation schema.
	///
	/// Generated from `#[field(...)]` attributes.
	fn validation_schema() -> Option<BsonDocument> {
		None
	}

	/// Validate this document at the application layer.
	///
	/// Generated from `#[field(validate = "...")]` attributes.
	fn validate(&self) -> OdmResult<()> {
		Ok(())
	}
}

/// Index model placeholder.
///
/// This will be fully implemented in TASK-4.1.
#[derive(Debug, Clone)]
pub struct IndexModel {
	// TODO: TASK-4.1: Implement index model
}

impl IndexModel {
	/// Create a new index model builder.
	pub fn builder() -> IndexModelBuilder {
		IndexModelBuilder
	}
}

/// Index model builder placeholder.
#[derive(Debug)]
pub struct IndexModelBuilder;

impl IndexModelBuilder {
	/// Build the index model.
	pub fn build(self) -> IndexModel {
		IndexModel {}
	}
}
