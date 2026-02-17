//! Field validator specifications for metadata

use serde::{Deserialize, Serialize};

/// Field validator specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValidator {
	/// Type of validator (e.g., "email", "url", "regex", "min_length", "max_length")
	pub validator_type: String,
	/// Optional validator configuration (e.g., regex pattern, min/max values)
	#[serde(skip_serializing_if = "Option::is_none")]
	pub options: Option<serde_json::Value>,
	/// Optional error message for validation failures
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message: Option<String>,
}

impl FieldValidator {
	/// Extracts regex pattern from validator options for OpenAPI schema generation
	///
	/// # Returns
	///
	/// Returns `Some(String)` containing the regex pattern if this is a regex validator
	/// with a valid pattern option, otherwise returns `None`.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::metadata::FieldValidator;
	/// use serde_json::json;
	///
	/// let validator = FieldValidator {
	///     validator_type: "regex".to_string(),
	///     options: Some(json!({"pattern": "^[a-zA-Z0-9_]+$"})),
	///     message: None,
	/// };
	///
	/// assert_eq!(validator.extract_pattern(), Some("^[a-zA-Z0-9_]+$".to_string()));
	/// ```
	pub fn extract_pattern(&self) -> Option<String> {
		if self.validator_type == "regex" {
			self.options
				.as_ref()
				.and_then(|opts| opts.get("pattern"))
				.and_then(|p| p.as_str())
				.map(|s| s.to_string())
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_validator_with_options() {
		let validator = FieldValidator {
			validator_type: "range".to_string(),
			options: Some(serde_json::json!({"min": 1, "max": 100})),
			message: Some("Value must be between 1 and 100".to_string()),
		};

		assert_eq!(validator.validator_type, "range");
		assert!(validator.options.is_some());

		let options = validator.options.as_ref().unwrap();
		assert_eq!(options["min"], 1);
		assert_eq!(options["max"], 100);
	}

	#[rstest]
	fn test_validator_serialization() {
		let validator = FieldValidator {
			validator_type: "custom".to_string(),
			options: Some(serde_json::json!({"key": "value"})),
			message: Some("Custom validation failed".to_string()),
		};

		let json = serde_json::to_string(&validator).unwrap();
		assert!(json.contains("custom"));
		assert!(json.contains("Custom validation failed"));
	}
}
