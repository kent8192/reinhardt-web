//! Form generation and validation for admin views
//!
//! This module provides form handling functionality for creating and editing
//! model instances in the admin interface.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Form field types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
	/// Text input
	Text,
	/// Textarea for long text
	TextArea,
	/// Number input
	Number,
	/// Boolean checkbox
	Boolean,
	/// Email input
	Email,
	/// Date input
	Date,
	/// DateTime input
	DateTime,
	/// Select dropdown
	Select { choices: Vec<(String, String)> },
	/// Multiple select
	MultiSelect { choices: Vec<(String, String)> },
	/// File upload
	File,
	/// Hidden field
	Hidden,
}

/// Form field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
	/// Field name
	pub name: String,
	/// Field label
	pub label: String,
	/// Field type
	pub field_type: FieldType,
	/// Whether the field is required
	pub required: bool,
	/// Whether the field is readonly
	pub readonly: bool,
	/// Help text
	pub help_text: Option<String>,
	/// Initial/current value
	pub value: Option<serde_json::Value>,
	/// Validation errors
	pub errors: Vec<String>,
}

impl FormField {
	/// Create a new form field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin::{FormField, FieldType};
	///
	/// let field = FormField::new("email", "Email Address", FieldType::Email);
	/// assert_eq!(field.name, "email");
	/// assert_eq!(field.label, "Email Address");
	/// ```
	pub fn new(name: impl Into<String>, label: impl Into<String>, field_type: FieldType) -> Self {
		Self {
			name: name.into(),
			label: label.into(),
			field_type,
			required: false,
			readonly: false,
			help_text: None,
			value: None,
			errors: Vec::new(),
		}
	}

	/// Mark field as required
	pub fn required(mut self) -> Self {
		self.required = true;
		self
	}

	/// Mark field as readonly
	pub fn readonly(mut self) -> Self {
		self.readonly = true;
		self
	}

	/// Set help text
	pub fn with_help_text(mut self, text: impl Into<String>) -> Self {
		self.help_text = Some(text.into());
		self
	}

	/// Set field value
	pub fn with_value(mut self, value: serde_json::Value) -> Self {
		self.value = Some(value);
		self
	}

	/// Add validation error
	pub fn add_error(&mut self, error: impl Into<String>) {
		self.errors.push(error.into());
	}

	/// Check if field has errors
	pub fn has_errors(&self) -> bool {
		!self.errors.is_empty()
	}
}

/// Admin form for model creation and editing
///
/// # Examples
///
/// ```
/// use reinhardt_admin::{AdminForm, FormField, FieldType};
///
/// let form = AdminForm::new("User")
///     .add_field(FormField::new("username", "Username", FieldType::Text).required())
///     .add_field(FormField::new("email", "Email", FieldType::Email).required())
///     .add_field(FormField::new("bio", "Biography", FieldType::TextArea));
///
/// assert_eq!(form.model_name(), "User");
/// assert_eq!(form.field_count(), 3);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminForm {
	/// Model name
	model_name: String,
	/// Form fields
	fields: Vec<FormField>,
	/// Form-level errors
	errors: Vec<String>,
	/// Whether this is for creation or update
	is_creation: bool,
}

impl AdminForm {
	/// Create a new form
	pub fn new(model_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			fields: Vec::new(),
			errors: Vec::new(),
			is_creation: true,
		}
	}

	/// Create a form for updating
	pub fn for_update(model_name: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			fields: Vec::new(),
			errors: Vec::new(),
			is_creation: false,
		}
	}

	/// Get the model name
	pub fn model_name(&self) -> &str {
		&self.model_name
	}

	/// Check if this is a creation form
	pub fn is_creation(&self) -> bool {
		self.is_creation
	}

	/// Add a field to the form
	pub fn add_field(mut self, field: FormField) -> Self {
		self.fields.push(field);
		self
	}

	/// Get all fields
	pub fn fields(&self) -> &[FormField] {
		&self.fields
	}

	/// Get mutable reference to fields
	pub fn fields_mut(&mut self) -> &mut [FormField] {
		&mut self.fields
	}

	/// Get a field by name
	pub fn get_field(&self, name: &str) -> Option<&FormField> {
		self.fields.iter().find(|f| f.name == name)
	}

	/// Get mutable reference to a field by name
	pub fn get_field_mut(&mut self, name: &str) -> Option<&mut FormField> {
		self.fields.iter_mut().find(|f| f.name == name)
	}

	/// Get number of fields
	pub fn field_count(&self) -> usize {
		self.fields.len()
	}

	/// Add a form-level error
	pub fn add_error(&mut self, error: impl Into<String>) {
		self.errors.push(error.into());
	}

	/// Get form-level errors
	pub fn errors(&self) -> &[String] {
		&self.errors
	}

	/// Check if form has any errors (form-level or field-level)
	pub fn has_errors(&self) -> bool {
		!self.errors.is_empty() || self.fields.iter().any(|f| f.has_errors())
	}

	/// Validate the form
	///
	/// Checks required fields and returns whether validation passed.
	pub fn validate(&mut self, data: &HashMap<String, serde_json::Value>) -> bool {
		let mut is_valid = true;

		for field in &mut self.fields {
			if field.readonly {
				continue;
			}

			if field.required {
				let value = data.get(&field.name);
				if value.is_none() || value == Some(&serde_json::Value::Null) {
					field.add_error(format!("{} is required", field.label));
					is_valid = false;
				} else if let Some(serde_json::Value::String(s)) = value
					&& s.trim().is_empty() {
						field.add_error(format!("{} cannot be empty", field.label));
						is_valid = false;
					}
			}
		}

		is_valid
	}

	/// Set form data from a HashMap
	pub fn set_data(&mut self, data: HashMap<String, serde_json::Value>) {
		for field in &mut self.fields {
			if let Some(value) = data.get(&field.name) {
				field.value = Some(value.clone());
			}
		}
	}

	/// Get form data as HashMap
	pub fn get_data(&self) -> HashMap<String, serde_json::Value> {
		self.fields
			.iter()
			.filter_map(|field| field.value.clone().map(|value| (field.name.clone(), value)))
			.collect()
	}

	/// Clear all errors
	pub fn clear_errors(&mut self) {
		self.errors.clear();
		for field in &mut self.fields {
			field.errors.clear();
		}
	}
}

/// Form builder for creating forms from model specifications
///
/// # Examples
///
/// ```
/// use reinhardt_admin::{FormBuilder, FieldType};
///
/// let form = FormBuilder::new("User")
///     .add_text_field("username", "Username", true)
///     .add_email_field("email", "Email Address", true)
///     .add_textarea_field("bio", "Biography", false)
///     .build();
///
/// assert_eq!(form.field_count(), 3);
/// ```
pub struct FormBuilder {
	form: AdminForm,
}

impl FormBuilder {
	/// Create a new form builder
	pub fn new(model_name: impl Into<String>) -> Self {
		Self {
			form: AdminForm::new(model_name),
		}
	}

	/// Create a form builder for updates
	pub fn for_update(model_name: impl Into<String>) -> Self {
		Self {
			form: AdminForm::for_update(model_name),
		}
	}

	/// Add a text field
	pub fn add_text_field(
		mut self,
		name: impl Into<String>,
		label: impl Into<String>,
		required: bool,
	) -> Self {
		let mut field = FormField::new(name, label, FieldType::Text);
		if required {
			field = field.required();
		}
		self.form = self.form.add_field(field);
		self
	}

	/// Add a textarea field
	pub fn add_textarea_field(
		mut self,
		name: impl Into<String>,
		label: impl Into<String>,
		required: bool,
	) -> Self {
		let mut field = FormField::new(name, label, FieldType::TextArea);
		if required {
			field = field.required();
		}
		self.form = self.form.add_field(field);
		self
	}

	/// Add an email field
	pub fn add_email_field(
		mut self,
		name: impl Into<String>,
		label: impl Into<String>,
		required: bool,
	) -> Self {
		let mut field = FormField::new(name, label, FieldType::Email);
		if required {
			field = field.required();
		}
		self.form = self.form.add_field(field);
		self
	}

	/// Add a number field
	pub fn add_number_field(
		mut self,
		name: impl Into<String>,
		label: impl Into<String>,
		required: bool,
	) -> Self {
		let mut field = FormField::new(name, label, FieldType::Number);
		if required {
			field = field.required();
		}
		self.form = self.form.add_field(field);
		self
	}

	/// Add a boolean field
	pub fn add_boolean_field(mut self, name: impl Into<String>, label: impl Into<String>) -> Self {
		let field = FormField::new(name, label, FieldType::Boolean);
		self.form = self.form.add_field(field);
		self
	}

	/// Add a select field
	pub fn add_select_field(
		mut self,
		name: impl Into<String>,
		label: impl Into<String>,
		choices: Vec<(String, String)>,
		required: bool,
	) -> Self {
		let mut field = FormField::new(name, label, FieldType::Select { choices });
		if required {
			field = field.required();
		}
		self.form = self.form.add_field(field);
		self
	}

	/// Build the form
	pub fn build(self) -> AdminForm {
		self.form
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_form_field_new() {
		let field = FormField::new("email", "Email", FieldType::Email);
		assert_eq!(field.name, "email");
		assert_eq!(field.label, "Email");
		assert!(!field.required);
		assert!(!field.readonly);
	}

	#[test]
	fn test_form_field_required() {
		let field = FormField::new("email", "Email", FieldType::Email).required();
		assert!(field.required);
	}

	#[test]
	fn test_form_field_readonly() {
		let field = FormField::new("id", "ID", FieldType::Number).readonly();
		assert!(field.readonly);
	}

	#[test]
	fn test_form_field_with_help_text() {
		let field =
			FormField::new("email", "Email", FieldType::Email).with_help_text("Enter your email");
		assert_eq!(field.help_text, Some("Enter your email".to_string()));
	}

	#[test]
	fn test_form_field_add_error() {
		let mut field = FormField::new("email", "Email", FieldType::Email);
		assert!(!field.has_errors());

		field.add_error("Invalid email format");
		assert!(field.has_errors());
		assert_eq!(field.errors.len(), 1);
	}

	#[test]
	fn test_admin_form_new() {
		let form = AdminForm::new("User");
		assert_eq!(form.model_name(), "User");
		assert!(form.is_creation());
		assert_eq!(form.field_count(), 0);
	}

	#[test]
	fn test_admin_form_for_update() {
		let form = AdminForm::for_update("User");
		assert!(!form.is_creation());
	}

	#[test]
	fn test_admin_form_add_field() {
		let form = AdminForm::new("User")
			.add_field(FormField::new("username", "Username", FieldType::Text))
			.add_field(FormField::new("email", "Email", FieldType::Email));

		assert_eq!(form.field_count(), 2);
	}

	#[test]
	fn test_admin_form_get_field() {
		let form = AdminForm::new("User").add_field(FormField::new(
			"username",
			"Username",
			FieldType::Text,
		));

		let field = form.get_field("username");
		assert!(field.is_some());
		assert_eq!(field.unwrap().name, "username");

		assert!(form.get_field("nonexistent").is_none());
	}

	#[test]
	fn test_admin_form_validate_required() {
		let mut form = AdminForm::new("User")
			.add_field(FormField::new("username", "Username", FieldType::Text).required());

		let data = HashMap::new();
		assert!(!form.validate(&data));
		assert!(form.has_errors());
	}

	#[test]
	fn test_admin_form_validate_success() {
		let mut form = AdminForm::new("User")
			.add_field(FormField::new("username", "Username", FieldType::Text).required());

		let mut data = HashMap::new();
		data.insert(
			"username".to_string(),
			serde_json::Value::String("alice".to_string()),
		);

		assert!(form.validate(&data));
		assert!(!form.has_errors());
	}

	#[test]
	fn test_admin_form_set_data() {
		let mut form = AdminForm::new("User")
			.add_field(FormField::new("username", "Username", FieldType::Text))
			.add_field(FormField::new("email", "Email", FieldType::Email));

		let mut data = HashMap::new();
		data.insert(
			"username".to_string(),
			serde_json::Value::String("alice".to_string()),
		);
		data.insert(
			"email".to_string(),
			serde_json::Value::String("alice@example.com".to_string()),
		);

		form.set_data(data);

		let field = form.get_field("username").unwrap();
		assert_eq!(
			field.value,
			Some(serde_json::Value::String("alice".to_string()))
		);
	}

	#[test]
	fn test_admin_form_get_data() {
		let form = AdminForm::new("User")
			.add_field(
				FormField::new("username", "Username", FieldType::Text)
					.with_value(serde_json::Value::String("alice".to_string())),
			)
			.add_field(
				FormField::new("email", "Email", FieldType::Email)
					.with_value(serde_json::Value::String("alice@example.com".to_string())),
			);

		let data = form.get_data();
		assert_eq!(data.len(), 2);
		assert_eq!(
			data.get("username"),
			Some(&serde_json::Value::String("alice".to_string()))
		);
	}

	#[test]
	fn test_form_builder() {
		let form = FormBuilder::new("User")
			.add_text_field("username", "Username", true)
			.add_email_field("email", "Email", true)
			.add_textarea_field("bio", "Biography", false)
			.build();

		assert_eq!(form.field_count(), 3);
		assert!(form.get_field("username").unwrap().required);
		assert!(!form.get_field("bio").unwrap().required);
	}

	#[test]
	fn test_form_builder_for_update() {
		let form = FormBuilder::for_update("User")
			.add_text_field("username", "Username", true)
			.build();

		assert!(!form.is_creation());
	}

	#[test]
	fn test_form_clear_errors() {
		let mut form =
			AdminForm::new("User").add_field(FormField::new("email", "Email", FieldType::Email));

		form.add_error("Form error");
		form.get_field_mut("email")
			.unwrap()
			.add_error("Field error");

		assert!(form.has_errors());

		form.clear_errors();
		assert!(!form.has_errors());
	}
}
