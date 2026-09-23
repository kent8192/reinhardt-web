//! Patch nullability must be independent of create defaults and blank permission.

use reinhardt::model;
use json as serde_json;
use serde::{Deserialize, Serialize};

fn unused_create_default() -> String {
	panic!("patch validation must not evaluate create defaults")
}

#[model(app_label = "patch_nulls", info = false, form(name = EditNulls, fields(blank_text, defaulted_text, enabled, count, nullable, config, optional_config)))]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct NullRecord {
	#[field(primary_key = true)]
	id: i64,
	#[field(null = false, blank = true, max_length = 64)]
	blank_text: Option<String>,
	#[field(null = false, default = unused_create_default(), max_length = 64)]
	defaulted_text: Option<String>,
	#[field(null = false, default = true)]
	enabled: Option<bool>,
	#[field(null = false, default = 1)]
	count: Option<i64>,
	#[field(blank = true, max_length = 64)]
	nullable: Option<String>,
	config: serde_json::Value,
	#[field(null = false)]
	optional_config: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_core::model_form::{ModelFormPatchPayload, PatchValidationError};
	use reinhardt_core::validators::ValidationError;
	use rstest::rstest;

	#[rstest]
	#[case::blank_text("blank_text")]
	#[case::defaulted_text("defaulted_text")]
	#[case::boolean("enabled")]
	#[case::integer("count")]
	#[case::optional_json("optional_config")]
	#[cfg_attr(
		all(target_family = "wasm", target_os = "unknown"),
		wasm_bindgen_test::wasm_bindgen_test
	)]
	fn patch_rejects_nonnullable_null_before_cleaning(#[case] field: &str) {
		// Arrange
		let data: EditNullsData = json::from_value(json::json!({field: null})).unwrap();
		// Act
		let result = data.clean_and_validate_patch(None);
		// Assert
		let Err(PatchValidationError::Validation(errors)) = result else {
			panic!("null=false must reject explicit null before field cleaning");
		};
		assert_eq!(errors.field_errors().len(), 1);
		assert_eq!(
			errors.field_errors().get(field).unwrap(),
			&vec![ValidationError::Custom(
				"This field may not be null.".to_owned()
			)]
		);
	}

	#[rstest]
	#[case::omitted(json::json!({}))]
	#[case::nullable_null(json::json!({"nullable": null}))]
	#[case::json_null(json::json!({"config": null}))]
	#[case::allowed_blank(json::json!({"blank_text": ""}))]
	#[case::defaulted_value(json::json!({"defaulted_text": "updated"}))]
	#[case::false_value(json::json!({"enabled": false}))]
	#[case::zero_value(json::json!({"count": 0}))]
	#[cfg_attr(
		all(target_family = "wasm", target_os = "unknown"),
		wasm_bindgen_test::wasm_bindgen_test
	)]
	fn patch_nullability_preserves_other_submissions(#[case] expected: json::Value) {
		// Arrange
		let data: EditNullsData = json::from_value(expected.clone()).unwrap();
		// Act
		let cleaned = data.clean_and_validate_patch(None).unwrap();
		// Assert
		assert_eq!(json::to_value(cleaned.into_raw()).unwrap(), expected);
	}
}
