use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorType {
	Required,
	Invalid,
	MinLength,
	MaxLength,
	MinValue,
	MaxValue,
	Custom(String),
}

#[derive(Debug, thiserror::Error)]
pub enum FieldError {
	#[error("{0}")]
	Required(String),
	#[error("{0}")]
	Invalid(String),
	#[error("{0}")]
	Validation(String),
}

pub type FieldResult<T> = Result<T, FieldError>;

impl FieldError {
	/// Creates a required field error
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FieldError;
	///
	/// let error = FieldError::required(None);
	/// assert_eq!(error.to_string(), "This field is required.");
	///
	/// let custom_error = FieldError::required(Some("Name is mandatory"));
	/// assert_eq!(custom_error.to_string(), "Name is mandatory");
	/// ```
	pub fn required(custom_msg: Option<&str>) -> Self {
		FieldError::Required(custom_msg.unwrap_or("This field is required.").to_string())
	}
	/// Creates an invalid field error
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FieldError;
	///
	/// let error = FieldError::invalid(None, "Invalid input format");
	/// assert_eq!(error.to_string(), "Invalid input format");
	///
	/// let custom_error = FieldError::invalid(Some("Must be a number"), "Invalid");
	/// assert_eq!(custom_error.to_string(), "Must be a number");
	/// ```
	pub fn invalid(custom_msg: Option<&str>, default_msg: &str) -> Self {
		FieldError::Invalid(custom_msg.unwrap_or(default_msg).to_string())
	}
	/// Creates a validation field error
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FieldError;
	///
	/// let error = FieldError::validation(None, "Value out of range");
	/// assert_eq!(error.to_string(), "Value out of range");
	///
	/// let custom_error = FieldError::validation(Some("Too long"), "Length exceeded");
	/// assert_eq!(custom_error.to_string(), "Too long");
	/// ```
	pub fn validation(custom_msg: Option<&str>, default_msg: &str) -> Self {
		FieldError::Validation(custom_msg.unwrap_or(default_msg).to_string())
	}
}

/// Field widget type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Widget {
	TextInput,
	PasswordInput,
	EmailInput,
	NumberInput,
	TextArea,
	Select { choices: Vec<(String, String)> },
	CheckboxInput,
	RadioSelect { choices: Vec<(String, String)> },
	DateInput,
	DateTimeInput,
	FileInput,
	HiddenInput,
}

impl Widget {
	/// Renders the widget as HTML
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::Widget;
	///
	/// let widget = Widget::TextInput;
	/// let html = widget.render_html("username", Some("john_doe"), None);
	/// assert!(html.contains("<input"));
	/// assert!(html.contains("type=\"text\""));
	/// assert!(html.contains("name=\"username\""));
	/// assert!(html.contains("value=\"john_doe\""));
	/// ```
	pub fn render_html(
		&self,
		name: &str,
		value: Option<&str>,
		attrs: Option<&HashMap<String, String>>,
	) -> String {
		let mut html = String::new();
		let default_attrs = HashMap::new();
		let attrs = attrs.unwrap_or(&default_attrs);

		// Build common attributes
		let mut common_attrs = String::new();
		for (key, val) in attrs {
			common_attrs.push_str(&format!(" {}=\"{}\"", key, val));
		}

		match self {
			Widget::TextInput => {
				html.push_str(&format!(
					"<input type=\"text\" name=\"{}\" value=\"{}\"{}",
					name,
					value.unwrap_or(""),
					common_attrs
				));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::PasswordInput => {
				html.push_str(&format!(
					"<input type=\"password\" name=\"{}\" value=\"{}\"{}",
					name,
					value.unwrap_or(""),
					common_attrs
				));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::EmailInput => {
				html.push_str(&format!(
					"<input type=\"email\" name=\"{}\" value=\"{}\"{}",
					name,
					value.unwrap_or(""),
					common_attrs
				));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::NumberInput => {
				html.push_str(&format!(
					"<input type=\"number\" name=\"{}\" value=\"{}\"{}",
					name,
					value.unwrap_or(""),
					common_attrs
				));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::TextArea => {
				html.push_str(&format!("<textarea name=\"{}\"{}", name, common_attrs));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push('>');
				html.push_str(value.unwrap_or(""));
				html.push_str("</textarea>");
			}
			Widget::Select { choices } => {
				html.push_str(&format!("<select name=\"{}\"{}", name, common_attrs));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push('>');
				for (choice_value, choice_label) in choices {
					let selected = if Some(choice_value.as_str()) == value {
						" selected"
					} else {
						""
					};
					html.push_str(&format!(
						"<option value=\"{}\"{}>{}</option>",
						choice_value, selected, choice_label
					));
				}
				html.push_str("</select>");
			}
			Widget::CheckboxInput => {
				html.push_str(&format!("<input type=\"checkbox\" name=\"{}\"", name));
				if value == Some("true") || value == Some("on") {
					html.push_str(" checked");
				}
				html.push_str(&common_attrs);
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::RadioSelect { choices } => {
				for (i, (choice_value, choice_label)) in choices.iter().enumerate() {
					let checked = if Some(choice_value.as_str()) == value {
						" checked"
					} else {
						""
					};
					html.push_str(&format!(
						"<input type=\"radio\" name=\"{}\" value=\"{}\" id=\"id_{}_{}\"{}{} />",
						name, choice_value, name, i, checked, common_attrs
					));
					html.push_str(&format!(
						"<label for=\"id_{}_{}\">{}</label>",
						name, i, choice_label
					));
				}
			}
			Widget::DateInput => {
				html.push_str(&format!(
					"<input type=\"date\" name=\"{}\" value=\"{}\"{}",
					name,
					value.unwrap_or(""),
					common_attrs
				));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::DateTimeInput => {
				html.push_str(&format!(
					"<input type=\"datetime-local\" name=\"{}\" value=\"{}\"{}",
					name,
					value.unwrap_or(""),
					common_attrs
				));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::FileInput => {
				html.push_str(&format!(
					"<input type=\"file\" name=\"{}\"{}",
					name, common_attrs
				));
				if !attrs.contains_key("id") {
					html.push_str(&format!(" id=\"id_{}\"", name));
				}
				html.push_str(" />");
			}
			Widget::HiddenInput => {
				html.push_str(&format!(
					"<input type=\"hidden\" name=\"{}\" value=\"{}\" />",
					name,
					value.unwrap_or("")
				));
			}
		}

		html
	}
}

/// Base field trait for forms
///
/// This trait is specifically for form fields. For ORM fields, use `reinhardt_db::orm::Field`.
pub trait FormField: Send + Sync {
	fn name(&self) -> &str;
	fn label(&self) -> Option<&str>;
	fn required(&self) -> bool;
	fn help_text(&self) -> Option<&str>;
	fn widget(&self) -> &Widget;
	fn initial(&self) -> Option<&serde_json::Value>;

	fn clean(&self, value: Option<&serde_json::Value>) -> FieldResult<serde_json::Value>;

	/// Check if the field value has changed from its initial value
	fn has_changed(
		&self,
		initial: Option<&serde_json::Value>,
		data: Option<&serde_json::Value>,
	) -> bool {
		// Default implementation: compare values directly
		match (initial, data) {
			(None, None) => false,
			(Some(_), None) | (None, Some(_)) => true,
			(Some(i), Some(d)) => i != d,
		}
	}

	/// Get custom error messages for this field
	fn error_messages(&self) -> HashMap<ErrorType, String> {
		HashMap::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	// Note: Field-specific tests have been moved to their respective field modules
	// in the fields/ directory. Only FormField trait tests remain here.

	#[rstest]
	fn test_field_has_changed() {
		use crate::fields::CharField;

		let field = CharField::new("name".to_string());

		// No change: both None
		assert!(!field.has_changed(None, None));

		// Change: initial None, data Some
		assert!(field.has_changed(None, Some(&serde_json::json!("John"))));

		// Change: initial Some, data None
		assert!(field.has_changed(Some(&serde_json::json!("John")), None));

		// No change: same value
		assert!(!field.has_changed(
			Some(&serde_json::json!("John")),
			Some(&serde_json::json!("John"))
		));

		// Change: different value
		assert!(field.has_changed(
			Some(&serde_json::json!("John")),
			Some(&serde_json::json!("Jane"))
		));
	}

	#[rstest]
	fn test_field_error_messages() {
		use crate::fields::CharField;

		let field = CharField::new("name".to_string());

		// Default implementation returns empty HashMap
		assert!(field.error_messages().is_empty());
	}
}
