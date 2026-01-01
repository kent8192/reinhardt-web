//! Honeypot field for bot detection
//!
//! This module provides honeypot field functionality for detecting automated
//! form submissions. A honeypot field is a hidden form field that legitimate
//! users won't see or fill, but bots often auto-fill all form fields.
//!
//! ## How it works
//!
//! 1. Add a hidden field to your form (CSS hidden, not type="hidden")
//! 2. Legitimate users with CSS support won't see the field
//! 3. Bots that parse HTML and auto-fill forms will fill it
//! 4. If the field has any value, reject the submission as a bot
//!
//! ## Example
//!
//! ```
//! use reinhardt_middleware::honeypot::HoneypotField;
//!
//! let honeypot = HoneypotField::new("email_confirm".to_string());
//!
//! // Validate - empty value is valid (human)
//! assert!(honeypot.validate(None).is_ok());
//! assert!(honeypot.validate(Some("")).is_ok());
//!
//! // Non-empty value indicates bot
//! assert!(honeypot.validate(Some("bot-filled")).is_err());
//! ```

use std::collections::HashMap;

/// Honeypot validation error
#[derive(Debug, thiserror::Error)]
pub enum HoneypotError {
	/// Bot detected through honeypot field
	#[error("Bot detected: {0}")]
	BotDetected(String),
}

/// HoneypotField is a hidden field used to detect bots
///
/// Legitimate users won't see or fill this field, but bots often
/// auto-fill all form fields.
#[derive(Debug, Clone)]
pub struct HoneypotField {
	name: String,
	label: Option<String>,
}

impl HoneypotField {
	/// Create a new honeypot field
	///
	/// # Arguments
	///
	/// * `name` - The field name in the form
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_middleware::honeypot::HoneypotField;
	///
	/// let honeypot = HoneypotField::new("email_confirm".to_string());
	/// assert_eq!(honeypot.name(), "email_confirm");
	/// ```
	pub fn new(name: String) -> Self {
		Self { name, label: None }
	}

	/// Set the field label (for accessibility/screen readers)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_middleware::honeypot::HoneypotField;
	///
	/// let honeypot = HoneypotField::new("trap".to_string())
	///     .with_label("Please leave this field empty".to_string());
	/// assert_eq!(honeypot.label(), Some("Please leave this field empty"));
	/// ```
	pub fn with_label(mut self, label: String) -> Self {
		self.label = Some(label);
		self
	}

	/// Get the field name
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Get the field label
	pub fn label(&self) -> Option<&str> {
		self.label.as_deref()
	}

	/// Validate the honeypot field value
	///
	/// Returns Ok(()) if the field is empty (valid human submission),
	/// or Err if the field has a value (likely bot).
	///
	/// # Arguments
	///
	/// * `value` - The field value from form submission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_middleware::honeypot::HoneypotField;
	///
	/// let honeypot = HoneypotField::new("trap".to_string());
	///
	/// // Empty value is valid (not a bot)
	/// assert!(honeypot.validate(None).is_ok());
	/// assert!(honeypot.validate(Some("")).is_ok());
	///
	/// // Non-empty value indicates bot
	/// assert!(honeypot.validate(Some("bot-filled-this")).is_err());
	/// ```
	pub fn validate(&self, value: Option<&str>) -> Result<(), HoneypotError> {
		match value {
			None | Some("") => Ok(()),
			Some(_) => Err(HoneypotError::BotDetected(format!(
				"Honeypot field '{}' was filled",
				self.name
			))),
		}
	}

	/// Validate honeypot field from a HashMap of form data
	///
	/// # Arguments
	///
	/// * `data` - Form data as a HashMap
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_middleware::honeypot::HoneypotField;
	/// use std::collections::HashMap;
	///
	/// let honeypot = HoneypotField::new("email_confirm".to_string());
	///
	/// let mut data = HashMap::new();
	/// data.insert("email_confirm".to_string(), serde_json::json!(""));
	///
	/// assert!(honeypot.validate_form_data(&data).is_ok());
	///
	/// data.insert("email_confirm".to_string(), serde_json::json!("bot-value"));
	/// assert!(honeypot.validate_form_data(&data).is_err());
	/// ```
	pub fn validate_form_data(
		&self,
		data: &HashMap<String, serde_json::Value>,
	) -> Result<(), HoneypotError> {
		if let Some(value) = data.get(&self.name) {
			// Check if value is empty (null, empty string, or not present)
			let is_empty =
				value.is_null() || (value.is_string() && value.as_str().unwrap_or("").is_empty());

			if !is_empty {
				return Err(HoneypotError::BotDetected(format!(
					"Honeypot field '{}' was filled",
					self.name
				)));
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_honeypot_field_creation() {
		let honeypot = HoneypotField::new("trap".to_string());

		assert_eq!(honeypot.name(), "trap");
		assert_eq!(honeypot.label(), None);
	}

	#[test]
	fn test_honeypot_field_with_label() {
		let honeypot =
			HoneypotField::new("trap".to_string()).with_label("Leave this empty".to_string());

		assert_eq!(honeypot.label(), Some("Leave this empty"));
	}

	#[test]
	fn test_honeypot_field_validate_empty() {
		let honeypot = HoneypotField::new("trap".to_string());

		assert!(honeypot.validate(None).is_ok());
		assert!(honeypot.validate(Some("")).is_ok());
	}

	#[test]
	fn test_honeypot_field_validate_filled() {
		let honeypot = HoneypotField::new("trap".to_string());

		let result = honeypot.validate(Some("bot-value"));
		assert!(result.is_err());
	}

	#[test]
	fn test_honeypot_form_data_validation() {
		let honeypot = HoneypotField::new("email_confirm".to_string());

		let mut data = HashMap::new();
		data.insert("email_confirm".to_string(), serde_json::json!(""));

		assert!(honeypot.validate_form_data(&data).is_ok());

		data.insert("email_confirm".to_string(), serde_json::json!("bot-value"));
		assert!(honeypot.validate_form_data(&data).is_err());
	}

	#[test]
	fn test_honeypot_form_data_missing_field() {
		let honeypot = HoneypotField::new("nonexistent".to_string());

		let data = HashMap::new();
		assert!(honeypot.validate_form_data(&data).is_ok());
	}

	#[test]
	fn test_honeypot_form_data_null_value() {
		let honeypot = HoneypotField::new("trap".to_string());

		let mut data = HashMap::new();
		data.insert("trap".to_string(), serde_json::Value::Null);

		assert!(honeypot.validate_form_data(&data).is_ok());
	}
}
