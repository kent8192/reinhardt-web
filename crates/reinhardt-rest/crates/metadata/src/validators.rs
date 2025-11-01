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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
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

	#[test]
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
