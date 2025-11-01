use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core trait for database models
/// Uses composition instead of inheritance - models can implement multiple traits
pub trait Model: Serialize + for<'de> Deserialize<'de> + Send + Sync {
	/// The primary key type
	type PrimaryKey: Send + Sync + Clone + std::fmt::Display;

	/// Get the table name
	fn table_name() -> &'static str;

	/// Get the app label for this model
	///
	/// This is used by the migration system to organize models by application.
	/// Defaults to "default" if not specified.
	fn app_label() -> &'static str {
		"default"
	}

	/// Get the primary key field name
	fn primary_key_field() -> &'static str {
		"id"
	}

	/// Get the primary key value
	fn primary_key(&self) -> Option<&Self::PrimaryKey>;

	/// Set the primary key value
	fn set_primary_key(&mut self, value: Self::PrimaryKey);

	/// Get composite primary key definition if this model uses composite PK
	///
	/// Returns None for single primary key models, Some(CompositePrimaryKey) for composite PK models.
	fn composite_primary_key() -> Option<crate::composite_pk::CompositePrimaryKey> {
		None
	}

	/// Get composite primary key values for this instance
	///
	/// Only meaningful for models with composite primary keys.
	/// Returns empty HashMap for single primary key models.
	fn get_composite_pk_values(&self) -> HashMap<String, crate::composite_pk::PkValue> {
		HashMap::new()
	}

	/// Get field metadata for inspection
	///
	/// This method should be implemented to provide introspection capabilities.
	/// By default, returns an empty vector. Override this in derive macros or
	/// manual implementations to provide actual field metadata.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_orm::Model;
	///
	/// struct User {
	///     id: i32,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     // ... other required methods ...
	///
	///     fn field_metadata() -> Vec<crate::inspection::FieldInfo> {
	///         vec![
	///             // Field metadata would be generated here
	///         ]
	///     }
	/// }
	/// ```
	fn field_metadata() -> Vec<crate::inspection::FieldInfo> {
		Vec::new()
	}

	/// Get relationship metadata for inspection
	///
	/// This method should be implemented to provide relationship introspection.
	/// By default, returns an empty vector. Override this in derive macros or
	/// manual implementations to provide actual relationship metadata.
	fn relationship_metadata() -> Vec<crate::inspection::RelationInfo> {
		Vec::new()
	}

	/// Get index metadata for inspection
	///
	/// This method should be implemented to provide index introspection.
	/// By default, returns an empty vector. Override this in derive macros or
	/// manual implementations to provide actual index metadata.
	fn index_metadata() -> Vec<crate::inspection::IndexInfo> {
		Vec::new()
	}

	/// Get constraint metadata for inspection
	///
	/// This method should be implemented to provide constraint introspection.
	/// By default, returns an empty vector. Override this in derive macros or
	/// manual implementations to provide actual constraint metadata.
	fn constraint_metadata() -> Vec<crate::inspection::ConstraintInfo> {
		Vec::new()
	}
}

/// Trait for models with timestamps - compose this with Model
/// This follows Rust's composition pattern rather than Django's inheritance
pub trait Timestamped {
	fn created_at(&self) -> chrono::DateTime<chrono::Utc>;
	fn updated_at(&self) -> chrono::DateTime<chrono::Utc>;
	fn set_updated_at(&mut self, time: chrono::DateTime<chrono::Utc>);
}

/// Trait for soft-deletable models
/// Another composition trait instead of inheritance
pub trait SoftDeletable {
	fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>>;
	fn set_deleted_at(&mut self, time: Option<chrono::DateTime<chrono::Utc>>);
	fn is_deleted(&self) -> bool {
		self.deleted_at().is_some()
	}
}

/// Common timestamp fields that can be composed into structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
	pub created_at: chrono::DateTime<chrono::Utc>,
	pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Timestamps {
	/// Creates a new Timestamps instance with current time
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::model::Timestamps;
	///
	/// let timestamps = Timestamps::now();
	/// assert!(timestamps.created_at <= chrono::Utc::now());
	/// assert!(timestamps.updated_at <= chrono::Utc::now());
	/// ```
	pub fn now() -> Self {
		let now = chrono::Utc::now();
		Self {
			created_at: now,
			updated_at: now,
		}
	}
	/// Updates the updated_at timestamp to current time
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::model::Timestamps;
	/// use chrono::Utc;
	///
	/// let mut timestamps = Timestamps::now();
	/// let old_updated = timestamps.updated_at;
	///
	// Wait a small amount to ensure time difference
	/// std::thread::sleep(std::time::Duration::from_millis(1));
	/// timestamps.touch();
	///
	/// assert!(timestamps.updated_at > old_updated);
	/// ```
	pub fn touch(&mut self) {
		self.updated_at = chrono::Utc::now();
	}
}

/// Soft delete field that can be composed into structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftDelete {
	pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl SoftDelete {
	/// Creates a new SoftDelete instance with no deletion timestamp
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::model::SoftDelete;
	///
	/// let soft_delete = SoftDelete::new();
	/// assert!(soft_delete.deleted_at.is_none());
	/// ```
	pub fn new() -> Self {
		Self { deleted_at: None }
	}
	/// Marks the record as deleted by setting the deletion timestamp
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::model::SoftDelete;
	///
	/// let mut soft_delete = SoftDelete::new();
	/// assert!(!soft_delete.is_deleted());
	///
	/// soft_delete.delete();
	/// assert!(soft_delete.is_deleted());
	/// assert!(soft_delete.deleted_at.is_some());
	/// ```
	pub fn delete(&mut self) {
		self.deleted_at = Some(chrono::Utc::now());
	}
	/// Restores a soft-deleted record by clearing the deletion timestamp
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::model::SoftDelete;
	///
	/// let mut soft_delete = SoftDelete::new();
	/// soft_delete.delete();
	/// assert!(soft_delete.is_deleted());
	///
	/// soft_delete.restore();
	/// assert!(!soft_delete.is_deleted());
	/// assert!(soft_delete.deleted_at.is_none());
	/// ```
	pub fn restore(&mut self) {
		self.deleted_at = None;
	}

	/// Check if the record is soft-deleted
	pub fn is_deleted(&self) -> bool {
		self.deleted_at.is_some()
	}
}

impl Default for SoftDelete {
	fn default() -> Self {
		Self::new()
	}
}
