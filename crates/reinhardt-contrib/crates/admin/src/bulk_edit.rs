//! Bulk edit functionality for admin
//!
//! This module provides the ability to edit multiple records at once,
//! updating common fields across selected items.

use crate::{AdminError, AdminResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bulk edit operation
///
/// # Examples
///
/// ```
/// use reinhardt_admin::BulkEdit;
/// use serde_json::json;
///
/// let bulk_edit = BulkEdit::new("User")
///     .add_item_id("1")
///     .add_item_id("2")
///     .set_field("status", json!("active"));
///
/// assert_eq!(bulk_edit.item_count(), 2);
/// assert_eq!(bulk_edit.field_count(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEdit {
	/// Model name
	model_name: String,
	/// IDs of items to edit
	item_ids: Vec<String>,
	/// Fields to update with new values
	fields: HashMap<String, serde_json::Value>,
	/// Validation errors
	errors: Vec<String>,
}

impl BulkEdit {
	/// Create a new bulk edit operation
	pub fn new(model_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			item_ids: Vec::new(),
			fields: HashMap::new(),
			errors: Vec::new(),
		}
	}

	/// Get model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Add an item ID
	pub fn add_item_id(mut self, id: impl Into<String>) -> Self {
		self.item_ids.push(id.into());
		self
	}

	/// Set item IDs
	pub fn with_item_ids(mut self, ids: Vec<String>) -> Self {
		self.item_ids = ids;
		self
	}

	/// Get item IDs
	pub fn item_ids(&self) -> &[String] {
		&self.item_ids
	}

	/// Get item count
	pub fn item_count(&self) -> usize {
		self.item_ids.len()
	}

	/// Set a field value
	pub fn set_field(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
		self.fields.insert(field.into(), value);
		self
	}

	/// Get fields
	pub fn fields(&self) -> &HashMap<String, serde_json::Value> {
		&self.fields
	}

	/// Get field count
	pub fn field_count(&self) -> usize {
		self.fields.len()
	}

	/// Validate the bulk edit operation
	pub fn validate(&mut self) -> AdminResult<()> {
		self.errors.clear();

		if self.item_ids.is_empty() {
			self.errors
				.push("No items selected for bulk edit".to_string());
		}

		if self.fields.is_empty() {
			self.errors
				.push("No fields specified for bulk edit".to_string());
		}

		if !self.errors.is_empty() {
			return Err(AdminError::ValidationError(self.errors.join(", ")));
		}

		Ok(())
	}

	/// Get validation errors
	pub fn errors(&self) -> &[String] {
		&self.errors
	}

	/// Check if there are errors
	pub fn has_errors(&self) -> bool {
		!self.errors.is_empty()
	}
}

/// Result of a bulk edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEditResult {
	/// Number of items successfully updated
	pub updated_count: usize,
	/// Number of items that failed to update
	pub failed_count: usize,
	/// IDs of failed items
	pub failed_ids: Vec<String>,
	/// Error messages for failed items
	pub errors: Vec<String>,
}

impl BulkEditResult {
	/// Create a new result
	pub fn new() -> Self {
		Self {
			updated_count: 0,
			failed_count: 0,
			failed_ids: Vec::new(),
			errors: Vec::new(),
		}
	}

	/// Check if all items were successfully updated
	pub fn is_complete_success(&self) -> bool {
		self.failed_count == 0
	}

	/// Check if any items were updated
	pub fn has_updates(&self) -> bool {
		self.updated_count > 0
	}

	/// Get total items processed
	pub fn total_processed(&self) -> usize {
		self.updated_count + self.failed_count
	}

	/// Get success rate as percentage
	pub fn success_rate(&self) -> f64 {
		if self.total_processed() == 0 {
			0.0
		} else {
			(self.updated_count as f64 / self.total_processed() as f64) * 100.0
		}
	}
}

impl Default for BulkEditResult {
	fn default() -> Self {
		Self::new()
	}
}

/// Bulk edit form for collecting field updates
///
/// # Examples
///
/// ```
/// use reinhardt_admin::{BulkEditForm, BulkEditField};
///
/// let form = BulkEditForm::new("User")
///     .add_field(BulkEditField::new("status", "Status", "text"))
///     .add_field(BulkEditField::new("is_active", "Active", "checkbox"));
///
/// assert_eq!(form.field_count(), 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEditForm {
	/// Model name
	model_name: String,
	/// Fields available for bulk editing
	fields: Vec<BulkEditField>,
	/// Number of items to be edited
	item_count: usize,
}

impl BulkEditForm {
	/// Create a new bulk edit form
	pub fn new(model_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			fields: Vec::new(),
			item_count: 0,
		}
	}

	/// Get model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Add a field
	pub fn add_field(mut self, field: BulkEditField) -> Self {
		self.fields.push(field);
		self
	}

	/// Get fields
	pub fn fields(&self) -> &[BulkEditField] {
		&self.fields
	}

	/// Get field count
	pub fn field_count(&self) -> usize {
		self.fields.len()
	}

	/// Set item count
	pub fn with_item_count(mut self, count: usize) -> Self {
		self.item_count = count;
		self
	}

	/// Get item count
	pub fn item_count(&self) -> usize {
		self.item_count
	}

	/// Get field by name
	pub fn get_field(&self, name: &str) -> Option<&BulkEditField> {
		self.fields.iter().find(|f| f.name == name)
	}
}

/// Field definition for bulk editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEditField {
	/// Field name
	pub name: String,
	/// Field label
	pub label: String,
	/// Field type
	pub field_type: String,
	/// Whether to update this field
	pub enabled: bool,
	/// New value for the field
	pub value: Option<serde_json::Value>,
	/// Help text
	pub help_text: Option<String>,
}

impl BulkEditField {
	/// Create a new bulk edit field
	pub fn new(
		name: impl Into<String>,
		label: impl Into<String>,
		field_type: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			field_type: field_type.into(),
			enabled: false,
			value: None,
			help_text: None,
		}
	}

	/// Enable this field for editing
	pub fn enable(mut self) -> Self {
		self.enabled = true;
		self
	}

	/// Set field value
	pub fn with_value(mut self, value: serde_json::Value) -> Self {
		self.value = Some(value);
		self.enabled = true;
		self
	}

	/// Set help text
	pub fn with_help_text(mut self, text: impl Into<String>) -> Self {
		self.help_text = Some(text.into());
		self
	}
}

/// Bulk edit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEditConfig {
	/// Fields that can be bulk edited
	pub allowed_fields: Vec<String>,
	/// Fields that cannot be bulk edited
	pub readonly_fields: Vec<String>,
	/// Maximum number of items that can be edited at once
	pub max_items: Option<usize>,
	/// Whether to require confirmation
	pub require_confirmation: bool,
}

impl BulkEditConfig {
	/// Create a new configuration
	pub fn new() -> Self {
		Self {
			allowed_fields: Vec::new(),
			readonly_fields: Vec::new(),
			max_items: Some(100),
			require_confirmation: true,
		}
	}

	/// Add an allowed field
	pub fn allow_field(mut self, field: impl Into<String>) -> Self {
		self.allowed_fields.push(field.into());
		self
	}

	/// Add a readonly field
	pub fn readonly_field(mut self, field: impl Into<String>) -> Self {
		self.readonly_fields.push(field.into());
		self
	}

	/// Set maximum items
	pub fn with_max_items(mut self, max: usize) -> Self {
		self.max_items = Some(max);
		self
	}

	/// Set whether confirmation is required
	pub fn with_confirmation(mut self, require: bool) -> Self {
		self.require_confirmation = require;
		self
	}

	/// Check if a field can be bulk edited
	pub fn can_edit_field(&self, field: &str) -> bool {
		!self.readonly_fields.contains(&field.to_string())
			&& (self.allowed_fields.is_empty() || self.allowed_fields.contains(&field.to_string()))
	}

	/// Check if item count is within limits
	pub fn is_item_count_valid(&self, count: usize) -> bool {
		self.max_items.map_or(true, |max| count <= max)
	}
}

impl Default for BulkEditConfig {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_bulk_edit_new() {
		let bulk_edit = BulkEdit::new("User");
		assert_eq!(bulk_edit.model_name(), "User");
		assert_eq!(bulk_edit.item_count(), 0);
		assert_eq!(bulk_edit.field_count(), 0);
	}

	#[test]
	fn test_bulk_edit_add_item_id() {
		let bulk_edit = BulkEdit::new("User")
			.add_item_id("1")
			.add_item_id("2")
			.add_item_id("3");

		assert_eq!(bulk_edit.item_count(), 3);
	}

	#[test]
	fn test_bulk_edit_set_field() {
		let bulk_edit = BulkEdit::new("User")
			.set_field("status", json!("active"))
			.set_field("role", json!("admin"));

		assert_eq!(bulk_edit.field_count(), 2);
		assert_eq!(bulk_edit.fields().get("status"), Some(&json!("active")));
	}

	#[test]
	fn test_bulk_edit_validate_empty_items() {
		let mut bulk_edit = BulkEdit::new("User").set_field("status", json!("active"));

		let result = bulk_edit.validate();
		assert!(result.is_err());
		assert!(bulk_edit.has_errors());
	}

	#[test]
	fn test_bulk_edit_validate_empty_fields() {
		let mut bulk_edit = BulkEdit::new("User").add_item_id("1").add_item_id("2");

		let result = bulk_edit.validate();
		assert!(result.is_err());
		assert!(bulk_edit.has_errors());
	}

	#[test]
	fn test_bulk_edit_validate_success() {
		let mut bulk_edit = BulkEdit::new("User")
			.add_item_id("1")
			.set_field("status", json!("active"));

		let result = bulk_edit.validate();
		assert!(result.is_ok());
		assert!(!bulk_edit.has_errors());
	}

	#[test]
	fn test_bulk_edit_result_new() {
		let result = BulkEditResult::new();
		assert_eq!(result.updated_count, 0);
		assert_eq!(result.failed_count, 0);
		assert!(result.is_complete_success());
	}

	#[test]
	fn test_bulk_edit_result_success_rate() {
		let result = BulkEditResult {
			updated_count: 8,
			failed_count: 2,
			failed_ids: vec!["3".to_string(), "7".to_string()],
			errors: vec![],
		};

		assert_eq!(result.total_processed(), 10);
		assert_eq!(result.success_rate(), 80.0);
		assert!(!result.is_complete_success());
		assert!(result.has_updates());
	}

	#[test]
	fn test_bulk_edit_form_new() {
		let form = BulkEditForm::new("User");
		assert_eq!(form.model_name(), "User");
		assert_eq!(form.field_count(), 0);
	}

	#[test]
	fn test_bulk_edit_form_add_field() {
		let form = BulkEditForm::new("User")
			.add_field(BulkEditField::new("status", "Status", "select"))
			.add_field(BulkEditField::new("is_active", "Active", "checkbox"));

		assert_eq!(form.field_count(), 2);
	}

	#[test]
	fn test_bulk_edit_form_get_field() {
		let form =
			BulkEditForm::new("User").add_field(BulkEditField::new("status", "Status", "select"));

		let field = form.get_field("status");
		assert!(field.is_some());
		assert_eq!(field.unwrap().name, "status");

		assert!(form.get_field("nonexistent").is_none());
	}

	#[test]
	fn test_bulk_edit_field_new() {
		let field = BulkEditField::new("status", "Status", "select");
		assert_eq!(field.name, "status");
		assert_eq!(field.label, "Status");
		assert!(!field.enabled);
	}

	#[test]
	fn test_bulk_edit_field_enable() {
		let field = BulkEditField::new("status", "Status", "select").enable();
		assert!(field.enabled);
	}

	#[test]
	fn test_bulk_edit_field_with_value() {
		let field = BulkEditField::new("status", "Status", "select").with_value(json!("active"));

		assert!(field.enabled);
		assert_eq!(field.value, Some(json!("active")));
	}

	#[test]
	fn test_bulk_edit_config_new() {
		let config = BulkEditConfig::new();
		assert!(config.allowed_fields.is_empty());
		assert_eq!(config.max_items, Some(100));
		assert!(config.require_confirmation);
	}

	#[test]
	fn test_bulk_edit_config_allow_field() {
		let config = BulkEditConfig::new()
			.allow_field("status")
			.allow_field("role");

		assert_eq!(config.allowed_fields.len(), 2);
	}

	#[test]
	fn test_bulk_edit_config_can_edit_field() {
		let config = BulkEditConfig::new()
			.allow_field("status")
			.allow_field("role")
			.readonly_field("id");

		assert!(config.can_edit_field("status"));
		assert!(config.can_edit_field("role"));
		assert!(!config.can_edit_field("id"));
	}

	#[test]
	fn test_bulk_edit_config_is_item_count_valid() {
		let config = BulkEditConfig::new().with_max_items(10);

		assert!(config.is_item_count_valid(5));
		assert!(config.is_item_count_valid(10));
		assert!(!config.is_item_count_valid(15));
	}
}
