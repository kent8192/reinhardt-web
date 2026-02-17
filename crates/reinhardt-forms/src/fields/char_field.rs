//! Character field for text input

use crate::field::{FieldError, FieldResult, FormField, Widget};

/// Character field with length validation
#[derive(Debug, Clone)]
pub struct CharField {
	pub name: String,
	pub label: Option<String>,
	pub required: bool,
	pub help_text: Option<String>,
	pub widget: Widget,
	pub initial: Option<serde_json::Value>,
	pub max_length: Option<usize>,
	pub min_length: Option<usize>,
	pub strip: bool,
	pub empty_value: Option<String>,
}

impl CharField {
	/// Create a new CharField with the given name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("username".to_string());
	/// assert_eq!(field.name, "username");
	/// assert!(!field.required);
	/// assert_eq!(field.max_length, None);
	/// ```
	pub fn new(name: String) -> Self {
		Self {
			name,
			label: None,
			required: false,
			help_text: None,
			widget: Widget::TextInput,
			initial: None,
			max_length: None,
			min_length: None,
			strip: true,
			empty_value: None,
		}
	}
	/// Set the field as required
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("username".to_string()).required();
	/// assert!(field.required);
	/// ```
	pub fn required(mut self) -> Self {
		self.required = true;
		self
	}
	/// Set the maximum length for the field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("username".to_string()).with_max_length(100);
	/// assert_eq!(field.max_length, Some(100));
	/// ```
	pub fn with_max_length(mut self, max_length: usize) -> Self {
		self.max_length = Some(max_length);
		self
	}
	/// Set the minimum length for the field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("username".to_string()).with_min_length(5);
	/// assert_eq!(field.min_length, Some(5));
	/// ```
	pub fn with_min_length(mut self, min_length: usize) -> Self {
		self.min_length = Some(min_length);
		self
	}
	/// Set the label for the field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("username".to_string()).with_label("Username");
	/// assert_eq!(field.label, Some("Username".to_string()));
	/// ```
	pub fn with_label(mut self, label: impl Into<String>) -> Self {
		self.label = Some(label.into());
		self
	}
	/// Set the help text for the field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("username".to_string()).with_help_text("Enter your username");
	/// assert_eq!(field.help_text, Some("Enter your username".to_string()));
	/// ```
	pub fn with_help_text(mut self, help_text: impl Into<String>) -> Self {
		self.help_text = Some(help_text.into());
		self
	}
	/// Set the initial value for the field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("username".to_string()).with_initial("default value");
	/// assert_eq!(field.initial, Some(serde_json::json!("default value")));
	/// ```
	pub fn with_initial(mut self, initial: impl Into<String>) -> Self {
		self.initial = Some(serde_json::json!(initial.into()));
		self
	}
	/// Disable whitespace stripping for the field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	///
	/// let field = CharField::new("description".to_string()).no_strip();
	/// assert!(!field.strip);
	/// ```
	pub fn no_strip(mut self) -> Self {
		self.strip = false;
		self
	}

	/// Set the widget for the field
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::fields::CharField;
	/// use reinhardt_forms::field::Widget;
	///
	/// let field = CharField::new("bio".to_string()).with_widget(Widget::TextArea);
	/// ```
	pub fn with_widget(mut self, widget: Widget) -> Self {
		self.widget = widget;
		self
	}
}

// Note: Default trait is not implemented because CharField requires a name

impl FormField for CharField {
	fn name(&self) -> &str {
		&self.name
	}

	fn label(&self) -> Option<&str> {
		self.label.as_deref()
	}

	fn required(&self) -> bool {
		self.required
	}

	fn help_text(&self) -> Option<&str> {
		self.help_text.as_deref()
	}

	fn widget(&self) -> &Widget {
		&self.widget
	}

	fn initial(&self) -> Option<&serde_json::Value> {
		self.initial.as_ref()
	}

	fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value> {
		// Convert JSON value to string
		let str_value = match value {
			Some(v) => {
				if v.is_null() {
					None
				} else {
					Some(v.as_str().ok_or_else(|| {
						FieldError::Validation("Value must be a string".to_string())
					})?)
				}
			}
			None => None,
		};

		// Process string value
		let processed_value = match str_value {
			Some(v) => {
				let v = if self.strip { v.trim() } else { v };
				if v.is_empty() {
					if self.required {
						return Err(FieldError::Required(self.name.clone()));
					}
					return Ok(serde_json::Value::String(
						self.empty_value.clone().unwrap_or_default(),
					));
				}
				v.to_string()
			}
			None => {
				if self.required {
					return Err(FieldError::Required(self.name.clone()));
				}
				return Ok(serde_json::Value::String(
					self.empty_value.clone().unwrap_or_default(),
				));
			}
		};

		// Validate length
		if let Some(max_length) = self.max_length
			&& processed_value.len() > max_length
		{
			return Err(FieldError::Validation(format!(
				"Ensure this value has at most {} characters (it has {})",
				max_length,
				processed_value.len()
			)));
		}

		if let Some(min_length) = self.min_length
			&& processed_value.len() < min_length
		{
			return Err(FieldError::Validation(format!(
				"Ensure this value has at least {} characters (it has {})",
				min_length,
				processed_value.len()
			)));
		}

		Ok(serde_json::Value::String(processed_value))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serde_json::json;

	#[rstest]
	fn test_char_field_required() {
		let field = CharField::new("test".to_string()).required();
		assert!(field.clean(None).is_err());
		assert!(field.clean(Some(&json!(""))).is_err());
		assert!(field.clean(Some(&json!("  "))).is_err());
	}

	#[rstest]
	fn test_char_field_max_length() {
		let field = CharField::new("test".to_string()).with_max_length(5);
		assert!(field.clean(Some(&json!("12345"))).is_ok());
		assert!(field.clean(Some(&json!("123456"))).is_err());
	}

	#[rstest]
	fn test_char_field_min_length() {
		let field = CharField::new("test".to_string()).with_min_length(3);
		assert!(field.clean(Some(&json!("123"))).is_ok());
		assert!(field.clean(Some(&json!("12"))).is_err());
	}
}
