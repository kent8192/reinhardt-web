use crate::bound_field::BoundField;
use crate::csrf::CsrfToken;
use crate::field::{FieldError, FormField};
use crate::media::Media;
use std::collections::HashMap;
use std::ops::Index;

#[derive(Debug, thiserror::Error)]
pub enum FormError {
	#[error("Field error in {field}: {error}")]
	Field { field: String, error: FieldError },
	#[error("Validation error: {0}")]
	Validation(String),
}

pub type FormResult<T> = Result<T, FormError>;

type CleanFunction =
	Box<dyn Fn(&HashMap<String, serde_json::Value>) -> FormResult<()> + Send + Sync>;
type FieldCleanFunction =
	Box<dyn Fn(&serde_json::Value) -> FormResult<serde_json::Value> + Send + Sync>;

/// Form data structure
pub struct Form {
	fields: Vec<Box<dyn FormField>>,
	data: HashMap<String, serde_json::Value>,
	initial: HashMap<String, serde_json::Value>,
	errors: HashMap<String, Vec<String>>,
	is_bound: bool,
	clean_functions: Vec<CleanFunction>,
	field_clean_functions: HashMap<String, FieldCleanFunction>,
	prefix: String,
	use_csrf: bool,
	csrf_token: Option<CsrfToken>,
}

impl Form {
	/// Create a new empty form
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	///
	/// let form = Form::new();
	/// assert!(!form.is_bound());
	/// assert!(form.fields().is_empty());
	/// ```
	pub fn new() -> Self {
		Self {
			fields: vec![],
			data: HashMap::new(),
			initial: HashMap::new(),
			errors: HashMap::new(),
			is_bound: false,
			clean_functions: vec![],
			field_clean_functions: HashMap::new(),
			prefix: String::new(),
			use_csrf: false,
			csrf_token: None,
		}
	}
	/// Create a new form with initial data
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut initial = HashMap::new();
	/// initial.insert("name".to_string(), json!("John"));
	///
	/// let form = Form::with_initial(initial);
	/// assert_eq!(form.initial().get("name"), Some(&json!("John")));
	/// ```
	pub fn with_initial(initial: HashMap<String, serde_json::Value>) -> Self {
		Self {
			fields: vec![],
			data: HashMap::new(),
			initial,
			errors: HashMap::new(),
			is_bound: false,
			clean_functions: vec![],
			field_clean_functions: HashMap::new(),
			prefix: String::new(),
			use_csrf: false,
			csrf_token: None,
		}
	}
	/// Create a new form with a field prefix
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	///
	/// let form = Form::with_prefix("user".to_string());
	/// assert_eq!(form.prefix(), "user");
	/// assert_eq!(form.add_prefix_to_field_name("email"), "user-email");
	/// ```
	pub fn with_prefix(prefix: String) -> Self {
		Self {
			fields: vec![],
			data: HashMap::new(),
			initial: HashMap::new(),
			errors: HashMap::new(),
			is_bound: false,
			clean_functions: vec![],
			field_clean_functions: HashMap::new(),
			prefix,
			use_csrf: false,
			csrf_token: None,
		}
	}
	/// Enable CSRF protection for this form
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	///
	/// let mut form = Form::new();
	/// form.enable_csrf(Some("secret-key".to_string()));
	/// assert!(form.csrf_token().is_some());
	/// ```
	pub fn enable_csrf(&mut self, secret: Option<String>) {
		self.use_csrf = true;
		self.csrf_token = Some(if let Some(s) = secret {
			CsrfToken::new(s)
		} else {
			CsrfToken::default()
		});
	}
	pub fn csrf_token(&self) -> Option<&CsrfToken> {
		self.csrf_token.as_ref()
	}
	pub fn csrf_token_html(&self) -> String {
		if self.use_csrf {
			self.csrf_token
				.as_ref()
				.map(|t| t.as_hidden_input())
				.unwrap_or_default()
		} else {
			String::new()
		}
	}
	pub fn media(&self) -> Media {
		// Collect media from all fields' widgets
		// This is a simplified implementation
		// In a full implementation, widgets would define their own media
		Media::new()
	}
	/// Add a field to the form
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{Form, CharField, Field};
	///
	/// let mut form = Form::new();
	/// let field = CharField::new("username".to_string());
	/// form.add_field(Box::new(field));
	/// assert_eq!(form.fields().len(), 1);
	/// ```
	pub fn add_field(&mut self, field: Box<dyn FormField>) {
		self.fields.push(field);
	}
	/// Bind form data for validation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut form = Form::new();
	/// let mut data = HashMap::new();
	/// data.insert("username".to_string(), json!("john"));
	///
	/// form.bind(data);
	/// assert!(form.is_bound());
	/// ```
	pub fn bind(&mut self, data: HashMap<String, serde_json::Value>) {
		self.data = data;
		self.is_bound = true;
	}
	/// Validate the form and return true if all fields are valid
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{Form, CharField, Field};
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut form = Form::new();
	/// form.add_field(Box::new(CharField::new("username".to_string())));
	///
	/// let mut data = HashMap::new();
	/// data.insert("username".to_string(), json!("john"));
	/// form.bind(data);
	///
	/// assert!(form.is_valid());
	/// assert!(form.errors().is_empty());
	/// assert_eq!(form.cleaned_data().get("username"), Some(&json!("john")));
	/// ```
	pub fn is_valid(&mut self) -> bool {
		if !self.is_bound {
			return false;
		}

		self.errors.clear();

		for field in &self.fields {
			let value = self.data.get(field.name());

			match field.clean(value) {
				Ok(mut cleaned) => {
					// Run field-specific clean function if exists
					if let Some(field_clean) = self.field_clean_functions.get(field.name()) {
						match field_clean(&cleaned) {
							Ok(further_cleaned) => {
								cleaned = further_cleaned;
							}
							Err(e) => {
								self.errors
									.entry(field.name().to_string())
									.or_default()
									.push(e.to_string());
								continue;
							}
						}
					}
					self.data.insert(field.name().to_string(), cleaned);
				}
				Err(e) => {
					self.errors
						.entry(field.name().to_string())
						.or_default()
						.push(e.to_string());
				}
			}
		}

		// Run custom clean functions
		for clean_fn in &self.clean_functions {
			if let Err(e) = clean_fn(&self.data) {
				match e {
					FormError::Field { field, error } => {
						self.errors
							.entry(field)
							.or_default()
							.push(error.to_string());
					}
					FormError::Validation(msg) => {
						self.errors
							.entry("__all__".to_string())
							.or_default()
							.push(msg);
					}
				}
			}
		}

		self.errors.is_empty()
	}
	pub fn cleaned_data(&self) -> &HashMap<String, serde_json::Value> {
		&self.data
	}
	pub fn errors(&self) -> &HashMap<String, Vec<String>> {
		&self.errors
	}
	pub fn is_bound(&self) -> bool {
		self.is_bound
	}
	pub fn fields(&self) -> &[Box<dyn FormField>] {
		&self.fields
	}
	pub fn initial(&self) -> &HashMap<String, serde_json::Value> {
		&self.initial
	}
	/// Set initial data for the form
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut form = Form::new();
	/// let mut initial = HashMap::new();
	/// initial.insert("name".to_string(), json!("John"));
	/// form.set_initial(initial);
	/// ```
	pub fn set_initial(&mut self, initial: HashMap<String, serde_json::Value>) {
		self.initial = initial;
	}
	/// Check if any field has changed from its initial value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{Form, CharField, Field};
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut initial = HashMap::new();
	/// initial.insert("name".to_string(), json!("John"));
	///
	/// let mut form = Form::with_initial(initial);
	/// form.add_field(Box::new(CharField::new("name".to_string())));
	///
	/// let mut data = HashMap::new();
	/// data.insert("name".to_string(), json!("Jane"));
	/// form.bind(data);
	///
	/// assert!(form.has_changed());
	/// ```
	pub fn has_changed(&self) -> bool {
		if !self.is_bound {
			return false;
		}

		for field in &self.fields {
			let initial_val = self.initial.get(field.name());
			let data_val = self.data.get(field.name());
			if field.has_changed(initial_val, data_val) {
				return true;
			}
		}
		false
	}
	pub fn get_field(&self, name: &str) -> Option<&dyn FormField> {
		self.fields
			.iter()
			.find(|f| f.name() == name)
			.map(|f| f.as_ref())
	}
	pub fn remove_field(&mut self, name: &str) -> Option<Box<dyn FormField>> {
		let pos = self.fields.iter().position(|f| f.name() == name)?;
		Some(self.fields.remove(pos))
	}
	pub fn field_count(&self) -> usize {
		self.fields.len()
	}
	/// Add a custom clean function for form validation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut form = Form::new();
	/// form.add_clean_function(|data| {
	///     if data.get("password") != data.get("confirm_password") {
	///         Err(reinhardt_forms::FormError::Validation("Passwords do not match".to_string()))
	///     } else {
	///         Ok(())
	///     }
	/// });
	/// ```
	pub fn add_clean_function<F>(&mut self, f: F)
	where
		F: Fn(&HashMap<String, serde_json::Value>) -> FormResult<()> + Send + Sync + 'static,
	{
		self.clean_functions.push(Box::new(f));
	}
	/// Add a custom clean function for a specific field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Form;
	/// use serde_json::json;
	///
	/// let mut form = Form::new();
	/// form.add_field_clean_function("email", |value| {
	///     if let Some(email) = value.as_str() {
	///         if email.contains("@") {
	///             Ok(value.clone())
	///         } else {
	///             Err(reinhardt_forms::FormError::Validation("Invalid email".to_string()))
	///         }
	///     } else {
	///         Ok(value.clone())
	///     }
	/// });
	/// ```
	pub fn add_field_clean_function<F>(&mut self, field_name: &str, f: F)
	where
		F: Fn(&serde_json::Value) -> FormResult<serde_json::Value> + Send + Sync + 'static,
	{
		self.field_clean_functions
			.insert(field_name.to_string(), Box::new(f));
	}
	pub fn prefix(&self) -> &str {
		&self.prefix
	}
	pub fn set_prefix(&mut self, prefix: String) {
		self.prefix = prefix;
	}
	pub fn add_prefix_to_field_name(&self, field_name: &str) -> String {
		if self.prefix.is_empty() {
			field_name.to_string()
		} else {
			format!("{}-{}", self.prefix, field_name)
		}
	}
	pub fn get_bound_field<'a>(&'a self, name: &str) -> Option<BoundField<'a>> {
		let field = self.get_field(name)?;
		let data = self.data.get(name);
		let errors = self.errors.get(name).map(|e| e.as_slice()).unwrap_or(&[]);

		Some(BoundField::new(
			"form".to_string(),
			field,
			data,
			errors,
			&self.prefix,
		))
	}
	/// Render form as HTML table rows
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{Form, CharField, Field};
	///
	/// let mut form = Form::new();
	/// form.add_field(Box::new(CharField::new("username".to_string())));
	/// let html = form.as_table();
	/// assert!(html.contains("username"));
	/// ```
	pub fn as_table(&self) -> String {
		let mut html = String::new();

		// Non-field errors
		if let Some(errors) = self.errors.get("__all__") {
			html.push_str(r#"<tr><td colspan="2"><ul class="errorlist">"#);
			for error in errors {
				html.push_str(&format!("<li>{}</li>", html_escape(error)));
			}
			html.push_str("</ul></td></tr>");
		}

		// Fields
		for field in &self.fields {
			let bound = self.get_bound_field(field.name()).unwrap();
			let label = bound.label().unwrap_or(field.name());
			let required = if bound.is_required() { " *" } else { "" };

			html.push_str("<tr>");
			html.push_str(&format!(
				r#"<th><label for="{}">{}{}</label></th>"#,
				bound.id_for_label(),
				html_escape(label),
				required
			));
			html.push_str("<td>");

			if bound.has_errors() {
				html.push_str(r#"<ul class="errorlist">"#);
				for error in bound.errors() {
					html.push_str(&format!("<li>{}</li>", html_escape(error)));
				}
				html.push_str("</ul>");
			}

			html.push_str(&bound.as_widget());

			if let Some(help_text) = bound.help_text() {
				html.push_str(&format!(
					r#"<br><span class="helptext">{}</span>"#,
					html_escape(help_text)
				));
			}

			html.push_str("</td></tr>");
		}

		html
	}
	/// Render form as HTML paragraphs
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{Form, CharField, Field};
	///
	/// let mut form = Form::new();
	/// form.add_field(Box::new(CharField::new("email".to_string())));
	/// let html = form.as_p();
	/// assert!(html.contains("email"));
	/// ```
	pub fn as_p(&self) -> String {
		let mut html = String::new();

		// Non-field errors
		if let Some(errors) = self.errors.get("__all__") {
			html.push_str(r#"<ul class="errorlist">"#);
			for error in errors {
				html.push_str(&format!("<li>{}</li>", html_escape(error)));
			}
			html.push_str("</ul>");
		}

		// Fields
		for field in &self.fields {
			let bound = self.get_bound_field(field.name()).unwrap();
			let label = bound.label().unwrap_or(field.name());
			let required = if bound.is_required() { " *" } else { "" };

			html.push_str("<p>");

			if bound.has_errors() {
				html.push_str(r#"<ul class="errorlist">"#);
				for error in bound.errors() {
					html.push_str(&format!("<li>{}</li>", html_escape(error)));
				}
				html.push_str("</ul>");
			}

			html.push_str(&format!(
				r#"<label for="{}">{}{}</label> "#,
				bound.id_for_label(),
				html_escape(label),
				required
			));
			html.push_str(&bound.as_widget());

			if let Some(help_text) = bound.help_text() {
				html.push_str(&format!(
					r#" <span class="helptext">{}</span>"#,
					html_escape(help_text)
				));
			}

			html.push_str("</p>");
		}

		html
	}
	/// Render form as HTML list items
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{Form, CharField, Field};
	///
	/// let mut form = Form::new();
	/// form.add_field(Box::new(CharField::new("name".to_string())));
	/// let html = form.as_ul();
	/// assert!(html.contains("name"));
	/// ```
	pub fn as_ul(&self) -> String {
		let mut html = String::new();

		// Non-field errors
		if let Some(errors) = self.errors.get("__all__") {
			html.push_str(r#"<li><ul class="errorlist">"#);
			for error in errors {
				html.push_str(&format!("<li>{}</li>", html_escape(error)));
			}
			html.push_str("</ul></li>");
		}

		// Fields
		for field in &self.fields {
			let bound = self.get_bound_field(field.name()).unwrap();
			let label = bound.label().unwrap_or(field.name());
			let required = if bound.is_required() { " *" } else { "" };

			html.push_str("<li>");

			if bound.has_errors() {
				html.push_str(r#"<ul class="errorlist">"#);
				for error in bound.errors() {
					html.push_str(&format!("<li>{}</li>", html_escape(error)));
				}
				html.push_str("</ul>");
			}

			html.push_str(&format!(
				r#"<label for="{}">{}{}</label> "#,
				bound.id_for_label(),
				html_escape(label),
				required
			));
			html.push_str(&bound.as_widget());

			if let Some(help_text) = bound.help_text() {
				html.push_str(&format!(
					r#" <span class="helptext">{}</span>"#,
					html_escape(help_text)
				));
			}

			html.push_str("</li>");
		}

		html
	}
}

fn html_escape(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#x27;")
}

impl Default for Form {
	fn default() -> Self {
		Self::new()
	}
}

impl Index<&str> for Form {
	type Output = Box<dyn FormField>;

	fn index(&self, name: &str) -> &Self::Output {
		self.fields
			.iter()
			.find(|f| f.name() == name)
			.unwrap_or_else(|| panic!("Field '{}' not found", name))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field::CharField;

	#[test]
	fn test_form_validation() {
		let mut form = Form::new();

		let mut name_field = CharField::new("name".to_string());
		name_field.max_length = Some(50);
		form.add_field(Box::new(name_field));

		let mut data = HashMap::new();
		data.insert("name".to_string(), serde_json::json!("John Doe"));

		form.bind(data);
		assert!(form.is_valid());
		assert!(form.errors().is_empty());
	}

	#[test]
	fn test_form_validation_error() {
		let mut form = Form::new();

		let mut name_field = CharField::new("name".to_string());
		name_field.max_length = Some(5);
		form.add_field(Box::new(name_field));

		let mut data = HashMap::new();
		data.insert("name".to_string(), serde_json::json!("Very Long Name"));

		form.bind(data);
		assert!(!form.is_valid());
		assert!(!form.errors().is_empty());
	}

	// Additional tests based on Django forms tests

	#[test]
	fn test_form_basic() {
		// Test based on Django FormsTestCase.test_form
		use crate::field::CharField;

		let mut form = Form::new();
		form.add_field(Box::new(CharField::new("first_name".to_string())));
		form.add_field(Box::new(CharField::new("last_name".to_string())));

		let mut data = HashMap::new();
		data.insert("first_name".to_string(), serde_json::json!("John"));
		data.insert("last_name".to_string(), serde_json::json!("Lennon"));

		form.bind(data);

		assert!(form.is_bound());
		assert!(form.is_valid());
		assert!(form.errors().is_empty());

		// Check cleaned data
		let cleaned = form.cleaned_data();
		assert_eq!(
			cleaned.get("first_name").unwrap(),
			&serde_json::json!("John")
		);
		assert_eq!(
			cleaned.get("last_name").unwrap(),
			&serde_json::json!("Lennon")
		);
	}

	#[test]
	fn test_form_missing_required_fields() {
		// Form with missing required fields should have errors
		use crate::field::CharField;

		let mut form = Form::new();
		form.add_field(Box::new(CharField::new("username".to_string())));
		form.add_field(Box::new(CharField::new("email".to_string())));

		let data = HashMap::new(); // Empty data

		form.bind(data);

		assert!(form.is_bound());
		assert!(!form.is_valid());
		assert!(form.errors().contains_key("username"));
		assert!(form.errors().contains_key("email"));
	}

	#[test]
	fn test_form_optional_fields() {
		// Form with optional fields should validate even if they're missing
		use crate::field::CharField;

		let mut form = Form::new();

		let username_field = CharField::new("username".to_string());
		form.add_field(Box::new(username_field));

		let mut bio_field = CharField::new("bio".to_string());
		bio_field.required = false;
		form.add_field(Box::new(bio_field));

		let mut data = HashMap::new();
		data.insert("username".to_string(), serde_json::json!("john"));
		// bio is omitted

		form.bind(data);

		assert!(form.is_bound());
		assert!(form.is_valid());
		assert!(form.errors().is_empty());
	}

	#[test]
	fn test_form_unbound() {
		// Unbound form (no data provided)
		use crate::field::CharField;

		let mut form = Form::new();
		form.add_field(Box::new(CharField::new("name".to_string())));

		assert!(!form.is_bound());
		assert!(!form.is_valid()); // Unbound forms are not valid
	}

	#[test]
	fn test_form_extra_data() {
		// Form should ignore extra data not defined in fields
		use crate::field::CharField;

		let mut form = Form::new();
		form.add_field(Box::new(CharField::new("name".to_string())));

		let mut data = HashMap::new();
		data.insert("name".to_string(), serde_json::json!("John"));
		data.insert(
			"extra_field".to_string(),
			serde_json::json!("should be ignored"),
		);

		form.bind(data);

		assert!(form.is_valid());
		let cleaned = form.cleaned_data();
		assert_eq!(cleaned.get("name").unwrap(), &serde_json::json!("John"));
		// extra_field is still in data but not validated
		assert!(cleaned.contains_key("extra_field"));
	}

	#[test]
	fn test_forms_form_multiple_fields() {
		// Test form with multiple field types
		use crate::field::{CharField, IntegerField};

		let mut form = Form::new();
		form.add_field(Box::new(CharField::new("username".to_string())));

		let mut age_field = IntegerField::new("age".to_string());
		age_field.min_value = Some(0);
		age_field.max_value = Some(150);
		form.add_field(Box::new(age_field));

		let mut data = HashMap::new();
		data.insert("username".to_string(), serde_json::json!("alice"));
		data.insert("age".to_string(), serde_json::json!(30));

		form.bind(data);

		assert!(form.is_valid());
		assert!(form.errors().is_empty());
	}

	#[test]
	fn test_form_multiple_fields_invalid() {
		// Test form with multiple field types, some invalid
		use crate::field::{CharField, IntegerField};

		let mut form = Form::new();

		let mut username_field = CharField::new("username".to_string());
		username_field.min_length = Some(3);
		form.add_field(Box::new(username_field));

		let mut age_field = IntegerField::new("age".to_string());
		age_field.min_value = Some(0);
		age_field.max_value = Some(150);
		form.add_field(Box::new(age_field));

		let mut data = HashMap::new();
		data.insert("username".to_string(), serde_json::json!("ab")); // Too short
		data.insert("age".to_string(), serde_json::json!(200)); // Too large

		form.bind(data);

		assert!(!form.is_valid());
		assert!(form.errors().contains_key("username"));
		assert!(form.errors().contains_key("age"));
	}

	#[test]
	fn test_form_multiple_instances() {
		// Multiple form instances should be independent
		use crate::field::CharField;

		let mut form1 = Form::new();
		form1.add_field(Box::new(CharField::new("name".to_string())));

		let mut form2 = Form::new();
		form2.add_field(Box::new(CharField::new("name".to_string())));

		let mut data1 = HashMap::new();
		data1.insert("name".to_string(), serde_json::json!("Form1"));
		form1.bind(data1);

		let mut data2 = HashMap::new();
		data2.insert("name".to_string(), serde_json::json!("Form2"));
		form2.bind(data2);

		assert!(form1.is_valid());
		assert!(form2.is_valid());

		assert_eq!(
			form1.cleaned_data().get("name").unwrap(),
			&serde_json::json!("Form1")
		);
		assert_eq!(
			form2.cleaned_data().get("name").unwrap(),
			&serde_json::json!("Form2")
		);
	}

	#[test]
	fn test_form_with_initial_data() {
		let mut initial = HashMap::new();
		initial.insert("name".to_string(), serde_json::json!("Initial Name"));
		initial.insert("age".to_string(), serde_json::json!(25));

		let mut form = Form::with_initial(initial);

		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		let age_field = crate::field::IntegerField::new("age".to_string());
		form.add_field(Box::new(age_field));

		assert_eq!(
			form.initial().get("name").unwrap(),
			&serde_json::json!("Initial Name")
		);
		assert_eq!(form.initial().get("age").unwrap(), &serde_json::json!(25));
	}

	#[test]
	fn test_form_has_changed() {
		let mut initial = HashMap::new();
		initial.insert("name".to_string(), serde_json::json!("John"));

		let mut form = Form::with_initial(initial);

		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		// Same data as initial - should not have changed
		let mut data1 = HashMap::new();
		data1.insert("name".to_string(), serde_json::json!("John"));
		form.bind(data1);
		assert!(!form.has_changed());

		// Different data - should have changed
		let mut data2 = HashMap::new();
		data2.insert("name".to_string(), serde_json::json!("Jane"));
		form.bind(data2);
		assert!(form.has_changed());
	}

	#[test]
	fn test_form_index_access() {
		let mut form = Form::new();

		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		let field = &form["name"];
		assert_eq!(field.name(), "name");
	}

	#[test]
	#[should_panic(expected = "Field 'nonexistent' not found")]
	fn test_form_index_access_nonexistent() {
		let form = Form::new();
		let _ = &form["nonexistent"];
	}

	#[test]
	fn test_form_get_field() {
		let mut form = Form::new();

		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		assert!(form.get_field("name").is_some());
		assert!(form.get_field("nonexistent").is_none());
	}

	#[test]
	fn test_form_remove_field() {
		let mut form = Form::new();

		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		assert_eq!(form.field_count(), 1);

		let removed = form.remove_field("name");
		assert!(removed.is_some());
		assert_eq!(form.field_count(), 0);

		let not_removed = form.remove_field("nonexistent");
		assert!(not_removed.is_none());
	}

	#[test]
	fn test_form_custom_validation() {
		let mut form = Form::new();

		let mut password_field = CharField::new("password".to_string());
		password_field.min_length = Some(8);
		form.add_field(Box::new(password_field));

		let mut confirm_field = CharField::new("confirm".to_string());
		confirm_field.min_length = Some(8);
		form.add_field(Box::new(confirm_field));

		// Add custom validation to check passwords match
		form.add_clean_function(|data| {
			let password = data.get("password").and_then(|v| v.as_str());
			let confirm = data.get("confirm").and_then(|v| v.as_str());

			if password != confirm {
				return Err(FormError::Validation("Passwords do not match".to_string()));
			}

			Ok(())
		});

		// Test with matching passwords
		let mut data1 = HashMap::new();
		data1.insert("password".to_string(), serde_json::json!("secret123"));
		data1.insert("confirm".to_string(), serde_json::json!("secret123"));
		form.bind(data1);
		assert!(form.is_valid());

		// Test with non-matching passwords
		let mut data2 = HashMap::new();
		data2.insert("password".to_string(), serde_json::json!("secret123"));
		data2.insert("confirm".to_string(), serde_json::json!("different"));
		form.bind(data2);
		assert!(!form.is_valid());
		assert!(form.errors().contains_key("__all__"));
	}

	#[test]
	fn test_form_prefix() {
		let mut form = Form::with_prefix("profile".to_string());
		assert_eq!(form.prefix(), "profile");
		assert_eq!(form.add_prefix_to_field_name("name"), "profile-name");

		form.set_prefix("user".to_string());
		assert_eq!(form.prefix(), "user");
		assert_eq!(form.add_prefix_to_field_name("email"), "user-email");
	}

	#[test]
	fn test_form_field_clean_function() {
		let mut form = Form::new();

		let mut name_field = CharField::new("name".to_string());
		name_field.required = true;
		form.add_field(Box::new(name_field));

		// Add field-specific clean function to uppercase the name
		form.add_field_clean_function("name", |value| {
			if let Some(s) = value.as_str() {
				Ok(serde_json::json!(s.to_uppercase()))
			} else {
				Err(FormError::Validation("Expected string".to_string()))
			}
		});

		let mut data = HashMap::new();
		data.insert("name".to_string(), serde_json::json!("john doe"));
		form.bind(data);

		assert!(form.is_valid());
		assert_eq!(
			form.cleaned_data().get("name").unwrap(),
			&serde_json::json!("JOHN DOE")
		);
	}

	#[test]
	fn test_form_rendering_as_table() {
		let mut form = Form::new();
		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		let html = form.as_table();
		assert!(html.contains("<tr>"));
		assert!(html.contains("<th>"));
		assert!(html.contains("name"));
	}

	#[test]
	fn test_form_rendering_as_p() {
		let mut form = Form::new();
		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		let html = form.as_p();
		assert!(html.contains("<p>"));
		assert!(html.contains("<label"));
		assert!(html.contains("name"));
	}

	#[test]
	fn test_form_rendering_as_ul() {
		let mut form = Form::new();
		let name_field = CharField::new("name".to_string());
		form.add_field(Box::new(name_field));

		let html = form.as_ul();
		assert!(html.contains("<li>"));
		assert!(html.contains("<label"));
		assert!(html.contains("name"));
	}
}
