//! Inline editing functionality for related models
//!
//! This module provides inline editing capabilities, allowing users to edit
//! related models directly within the parent model's form.

use crate::{AdminError, AdminResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of inline formset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlineType {
	/// Stacked inline - displays each related object in a stacked block
	Stacked,
	/// Tabular inline - displays related objects in a table
	Tabular,
}

/// Configuration for inline model admin
///
/// # Examples
///
/// ```
/// use reinhardt_admin::{InlineModelAdmin, InlineType};
///
/// let inline = InlineModelAdmin::new("Comment", "post")
///     .with_type(InlineType::Tabular)
///     .with_extra(3)
///     .with_max_num(10);
///
/// assert_eq!(inline.model_name(), "Comment");
/// assert_eq!(inline.fk_name(), "post");
/// ```
#[derive(Debug, Clone)]
pub struct InlineModelAdmin {
	/// Name of the related model
	model_name: String,
	/// Foreign key field name pointing to parent
	fk_name: String,
	/// Type of inline display
	inline_type: InlineType,
	/// Fields to display
	fields: Vec<String>,
	/// Number of extra empty forms to display
	extra: usize,
	/// Maximum number of forms
	max_num: Option<usize>,
	/// Minimum number of forms
	min_num: usize,
	/// Whether to show delete checkbox
	can_delete: bool,
	/// Whether forms can be ordered
	can_order: bool,
	/// Readonly fields
	readonly_fields: Vec<String>,
}

impl InlineModelAdmin {
	/// Create a new inline model admin
	pub fn new(model_name: impl Into<String>, fk_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			fk_name: fk_name.into(),
			inline_type: InlineType::Stacked,
			fields: Vec::new(),
			extra: 3,
			max_num: None,
			min_num: 0,
			can_delete: true,
			can_order: false,
			readonly_fields: Vec::new(),
		}
	}

	/// Get the model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Get the foreign key field name
	pub fn fk_name(&self) -> &str {
		&self.fk_name
	}

	/// Set inline display type
	pub fn with_type(mut self, inline_type: InlineType) -> Self {
		self.inline_type = inline_type;
		self
	}

	/// Get inline type
	pub fn inline_type(&self) -> InlineType {
		self.inline_type
	}

	/// Set fields to display
	pub fn with_fields(mut self, fields: Vec<String>) -> Self {
		self.fields = fields;
		self
	}

	/// Get fields
	pub fn fields(&self) -> &[String] {
		&self.fields
	}

	/// Set number of extra forms
	pub fn with_extra(mut self, extra: usize) -> Self {
		self.extra = extra;
		self
	}

	/// Get number of extra forms
	pub fn extra(&self) -> usize {
		self.extra
	}

	/// Set maximum number of forms
	pub fn with_max_num(mut self, max_num: usize) -> Self {
		self.max_num = Some(max_num);
		self
	}

	/// Get maximum number of forms
	pub fn max_num(&self) -> Option<usize> {
		self.max_num
	}

	/// Set minimum number of forms
	pub fn with_min_num(mut self, min_num: usize) -> Self {
		self.min_num = min_num;
		self
	}

	/// Get minimum number of forms
	pub fn min_num(&self) -> usize {
		self.min_num
	}

	/// Enable/disable deletion
	pub fn with_can_delete(mut self, can_delete: bool) -> Self {
		self.can_delete = can_delete;
		self
	}

	/// Check if deletion is allowed
	pub fn can_delete(&self) -> bool {
		self.can_delete
	}

	/// Enable/disable ordering
	pub fn with_can_order(mut self, can_order: bool) -> Self {
		self.can_order = can_order;
		self
	}

	/// Check if ordering is allowed
	pub fn can_order(&self) -> bool {
		self.can_order
	}

	/// Set readonly fields
	pub fn with_readonly_fields(mut self, fields: Vec<String>) -> Self {
		self.readonly_fields = fields;
		self
	}

	/// Get readonly fields
	pub fn readonly_fields(&self) -> &[String] {
		&self.readonly_fields
	}
}

/// Inline formset for managing related objects
///
/// # Examples
///
/// ```
/// use reinhardt_admin::InlineFormset;
///
/// let formset = InlineFormset::new("Comment", "post", "123");
/// assert_eq!(formset.model_name(), "Comment");
/// assert_eq!(formset.parent_id(), "123");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineFormset {
	/// Related model name
	model_name: String,
	/// Foreign key field name
	fk_name: String,
	/// Parent object ID
	parent_id: String,
	/// Forms in the formset
	forms: Vec<InlineForm>,
	/// Management form data
	management_form: ManagementForm,
}

impl InlineFormset {
	/// Create a new inline formset
	pub fn new(
		model_name: impl Into<String>,
		fk_name: impl Into<String>,
		parent_id: impl Into<String>,
	) -> Self {
		Self {
			model_name: model_name.into(),
			fk_name: fk_name.into(),
			parent_id: parent_id.into(),
			forms: Vec::new(),
			management_form: ManagementForm::default(),
		}
	}

	/// Get model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Get foreign key field name
	pub fn fk_name(&self) -> &str {
		&self.fk_name
	}

	/// Get parent object ID
	pub fn parent_id(&self) -> &str {
		&self.parent_id
	}

	/// Add a form to the formset
	pub fn add_form(&mut self, form: InlineForm) {
		self.forms.push(form);
		self.management_form.total_forms += 1;
	}

	/// Get all forms
	pub fn forms(&self) -> &[InlineForm] {
		&self.forms
	}

	/// Get mutable forms
	pub fn forms_mut(&mut self) -> &mut [InlineForm] {
		&mut self.forms
	}

	/// Get management form
	pub fn management_form(&self) -> &ManagementForm {
		&self.management_form
	}

	/// Set management form
	pub fn set_management_form(&mut self, management_form: ManagementForm) {
		self.management_form = management_form;
	}

	/// Validate the formset
	pub fn validate(&mut self) -> AdminResult<()> {
		let mut valid_forms = 0;
		let mut errors = Vec::new();

		for (idx, form) in self.forms.iter_mut().enumerate() {
			if form.is_valid() {
				valid_forms += 1;
			} else {
				errors.push(format!("Form {} is invalid", idx));
			}
		}

		if valid_forms < self.management_form.min_num {
			return Err(AdminError::ValidationError(format!(
				"At least {} forms are required",
				self.management_form.min_num
			)));
		}

		if !errors.is_empty() {
			return Err(AdminError::ValidationError(errors.join(", ")));
		}

		Ok(())
	}

	/// Get changed forms
	pub fn changed_forms(&self) -> Vec<&InlineForm> {
		self.forms.iter().filter(|f| f.has_changed()).collect()
	}

	/// Get deleted forms
	pub fn deleted_forms(&self) -> Vec<&InlineForm> {
		self.forms
			.iter()
			.filter(|f| f.is_marked_for_deletion())
			.collect()
	}
}

/// Individual form within an inline formset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineForm {
	/// Form data
	data: HashMap<String, serde_json::Value>,
	/// Original data (for change detection)
	initial_data: HashMap<String, serde_json::Value>,
	/// Validation errors
	errors: Vec<String>,
	/// Whether this form is marked for deletion
	delete: bool,
	/// Order (if ordering is enabled)
	order: Option<usize>,
}

impl InlineForm {
	/// Create a new inline form
	pub fn new() -> Self {
		Self {
			data: HashMap::new(),
			initial_data: HashMap::new(),
			errors: Vec::new(),
			delete: false,
			order: None,
		}
	}

	/// Create a form with initial data
	pub fn with_initial(initial_data: HashMap<String, serde_json::Value>) -> Self {
		Self {
			data: initial_data.clone(),
			initial_data,
			errors: Vec::new(),
			delete: false,
			order: None,
		}
	}

	/// Set form data
	pub fn set_data(&mut self, data: HashMap<String, serde_json::Value>) {
		self.data = data;
	}

	/// Get form data
	pub fn data(&self) -> &HashMap<String, serde_json::Value> {
		&self.data
	}

	/// Check if form is valid
	pub fn is_valid(&self) -> bool {
		self.errors.is_empty()
	}

	/// Add error
	pub fn add_error(&mut self, error: impl Into<String>) {
		self.errors.push(error.into());
	}

	/// Get errors
	pub fn errors(&self) -> &[String] {
		&self.errors
	}

	/// Mark for deletion
	pub fn mark_for_deletion(&mut self) {
		self.delete = true;
	}

	/// Check if marked for deletion
	pub fn is_marked_for_deletion(&self) -> bool {
		self.delete
	}

	/// Set order
	pub fn set_order(&mut self, order: usize) {
		self.order = Some(order);
	}

	/// Get order
	pub fn order(&self) -> Option<usize> {
		self.order
	}

	/// Check if form has changed
	pub fn has_changed(&self) -> bool {
		self.data != self.initial_data
	}

	/// Get changed fields
	pub fn changed_fields(&self) -> Vec<String> {
		self.data
			.iter()
			.filter(|(key, value)| self.initial_data.get(*key) != Some(*value))
			.map(|(key, _)| key.clone())
			.collect()
	}
}

impl Default for InlineForm {
	fn default() -> Self {
		Self::new()
	}
}

/// Management form for tracking formset state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagementForm {
	/// Total number of forms in formset
	pub total_forms: usize,
	/// Number of initial forms (existing objects)
	pub initial_forms: usize,
	/// Minimum number of required forms
	pub min_num: usize,
	/// Maximum number of allowed forms
	pub max_num: Option<usize>,
}

impl Default for ManagementForm {
	fn default() -> Self {
		Self {
			total_forms: 0,
			initial_forms: 0,
			min_num: 0,
			max_num: Some(1000),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_inline_model_admin_new() {
		let inline = InlineModelAdmin::new("Comment", "post");
		assert_eq!(inline.model_name(), "Comment");
		assert_eq!(inline.fk_name(), "post");
		assert_eq!(inline.inline_type(), InlineType::Stacked);
		assert_eq!(inline.extra(), 3);
		assert!(inline.can_delete());
	}

	#[test]
	fn test_inline_model_admin_configuration() {
		let inline = InlineModelAdmin::new("Comment", "post")
			.with_type(InlineType::Tabular)
			.with_extra(5)
			.with_max_num(10)
			.with_min_num(1)
			.with_can_delete(false)
			.with_can_order(true);

		assert_eq!(inline.inline_type(), InlineType::Tabular);
		assert_eq!(inline.extra(), 5);
		assert_eq!(inline.max_num(), Some(10));
		assert_eq!(inline.min_num(), 1);
		assert!(!inline.can_delete());
		assert!(inline.can_order());
	}

	#[test]
	fn test_inline_formset_new() {
		let formset = InlineFormset::new("Comment", "post", "123");
		assert_eq!(formset.model_name(), "Comment");
		assert_eq!(formset.fk_name(), "post");
		assert_eq!(formset.parent_id(), "123");
		assert_eq!(formset.forms().len(), 0);
	}

	#[test]
	fn test_inline_formset_add_form() {
		let mut formset = InlineFormset::new("Comment", "post", "123");
		let form = InlineForm::new();
		formset.add_form(form);

		assert_eq!(formset.forms().len(), 1);
		assert_eq!(formset.management_form().total_forms, 1);
	}

	#[test]
	fn test_inline_form_new() {
		let form = InlineForm::new();
		assert!(form.is_valid());
		assert!(!form.is_marked_for_deletion());
		assert!(!form.has_changed());
	}

	#[test]
	fn test_inline_form_with_initial() {
		let mut initial = HashMap::new();
		initial.insert(
			"content".to_string(),
			serde_json::Value::String("Test".to_string()),
		);

		let form = InlineForm::with_initial(initial);
		assert_eq!(form.data().len(), 1);
		assert!(!form.has_changed());
	}

	#[test]
	fn test_inline_form_has_changed() {
		let mut initial = HashMap::new();
		initial.insert(
			"content".to_string(),
			serde_json::Value::String("Original".to_string()),
		);

		let mut form = InlineForm::with_initial(initial);
		assert!(!form.has_changed());

		let mut new_data = HashMap::new();
		new_data.insert(
			"content".to_string(),
			serde_json::Value::String("Modified".to_string()),
		);
		form.set_data(new_data);

		assert!(form.has_changed());
	}

	#[test]
	fn test_inline_form_changed_fields() {
		let mut initial = HashMap::new();
		initial.insert(
			"title".to_string(),
			serde_json::Value::String("Title".to_string()),
		);
		initial.insert(
			"content".to_string(),
			serde_json::Value::String("Content".to_string()),
		);

		let mut form = InlineForm::with_initial(initial);

		let mut new_data = HashMap::new();
		new_data.insert(
			"title".to_string(),
			serde_json::Value::String("New Title".to_string()),
		);
		new_data.insert(
			"content".to_string(),
			serde_json::Value::String("Content".to_string()),
		);
		form.set_data(new_data);

		let changed = form.changed_fields();
		assert_eq!(changed.len(), 1);
		assert!(changed.contains(&"title".to_string()));
	}

	#[test]
	fn test_inline_form_mark_for_deletion() {
		let mut form = InlineForm::new();
		assert!(!form.is_marked_for_deletion());

		form.mark_for_deletion();
		assert!(form.is_marked_for_deletion());
	}

	#[test]
	fn test_inline_form_order() {
		let mut form = InlineForm::new();
		assert_eq!(form.order(), None);

		form.set_order(5);
		assert_eq!(form.order(), Some(5));
	}

	#[test]
	fn test_inline_formset_validate_min_num() {
		let mut formset = InlineFormset::new("Comment", "post", "123");
		formset.management_form.min_num = 2;

		let form = InlineForm::new();
		formset.add_form(form);

		let result = formset.validate();
		assert!(result.is_err());
	}

	#[test]
	fn test_inline_formset_changed_forms() {
		let mut formset = InlineFormset::new("Comment", "post", "123");

		let mut initial = HashMap::new();
		initial.insert(
			"content".to_string(),
			serde_json::Value::String("Original".to_string()),
		);

		let mut form1 = InlineForm::with_initial(initial);
		let mut new_data = HashMap::new();
		new_data.insert(
			"content".to_string(),
			serde_json::Value::String("Modified".to_string()),
		);
		form1.set_data(new_data);

		let form2 = InlineForm::new();

		formset.add_form(form1);
		formset.add_form(form2);

		let changed = formset.changed_forms();
		assert_eq!(changed.len(), 1);
	}

	#[test]
	fn test_inline_formset_deleted_forms() {
		let mut formset = InlineFormset::new("Comment", "post", "123");

		let mut form1 = InlineForm::new();
		form1.mark_for_deletion();

		let form2 = InlineForm::new();

		formset.add_form(form1);
		formset.add_form(form2);

		let deleted = formset.deleted_forms();
		assert_eq!(deleted.len(), 1);
	}
}
