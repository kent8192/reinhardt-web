//! i18n forms tests
//!
//! Tests for form field localization

use reinhardt_forms::{DateField, DecimalField, FormField};
use rstest::rstest;
use serde_json::json;

#[rstest]
fn test_date_field_basic() {
	let field = DateField::new("birth_date".to_string());

	let result = field.clean(Some(&json!("2025-01-15")));
	assert!(result.is_ok());
}

#[rstest]
fn test_date_field_with_localize() {
	let field = DateField::new("birth_date".to_string())
		.with_localize(true)
		.with_locale("en_us".to_string());

	assert!(field.localize);
	assert_eq!(field.locale, Some("en_us".to_string()));
}

#[rstest]
fn test_date_field_multiple_formats() {
	let field = DateField::new("birth_date".to_string());

	// ISO format
	assert!(field.clean(Some(&json!("2025-01-15"))).is_ok());

	// US format
	assert!(field.clean(Some(&json!("01/15/2025"))).is_ok());
}

#[rstest]
fn test_decimal_field_basic() {
	let field = DecimalField::new("amount".to_string());

	let result = field.clean(Some(&json!(1234.56)));
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), json!(1234.56));
}

#[rstest]
fn test_decimal_field_with_localize() {
	let field = DecimalField::new("amount".to_string())
		.with_localize(true)
		.with_locale("en_us".to_string());

	assert!(field.localize);
	assert_eq!(field.locale, Some("en_us".to_string()));
}

#[rstest]
fn test_decimal_field_with_thousands_separator() {
	let field = DecimalField::new("amount".to_string()).with_thousands_separator(true);

	assert!(field.use_thousands_separator);
}

#[rstest]
fn test_decimal_field_localized_input() {
	let field = DecimalField::new("amount".to_string())
		.with_localize(true)
		.with_locale("en_us".to_string());

	// Should accept standard numeric format
	let result = field.clean(Some(&json!("1234.56")));
	assert!(result.is_ok());
}

#[rstest]
fn test_date_field_invalid_format() {
	let field = DateField::new("birth_date".to_string());

	let result = field.clean(Some(&json!("invalid-date")));
	assert!(result.is_err());
}

#[rstest]
fn test_decimal_field_max_digits() {
	let mut field = DecimalField::new("amount".to_string());
	field.max_digits = Some(5);
	field.decimal_places = Some(2);

	// Valid: 123.45 (5 total digits)
	assert!(field.clean(Some(&json!("123.45"))).is_ok());

	// Invalid: 1234.567 (7 total digits)
	assert!(field.clean(Some(&json!("1234.567"))).is_err());
}

#[rstest]
fn test_decimal_field_range_validation() {
	let mut field = DecimalField::new("amount".to_string());
	field.min_value = Some(0.0);
	field.max_value = Some(100.0);

	assert!(field.clean(Some(&json!(50.0))).is_ok());
	assert!(field.clean(Some(&json!(-1.0))).is_err());
	assert!(field.clean(Some(&json!(101.0))).is_err());
}

#[rstest]
fn test_date_field_required() {
	let field = DateField::new("birth_date".to_string());

	let result = field.clean(None);
	assert!(result.is_err());
}

#[rstest]
fn test_date_field_optional() {
	let mut field = DateField::new("birth_date".to_string());
	field.required = false;

	let result = field.clean(None);
	assert!(result.is_ok());
}

#[rstest]
fn test_decimal_field_required() {
	let field = DecimalField::new("amount".to_string());

	let result = field.clean(None);
	assert!(result.is_err());
}

#[rstest]
fn test_decimal_field_optional() {
	let mut field = DecimalField::new("amount".to_string());
	field.required = false;

	let result = field.clean(None);
	assert!(result.is_ok());
}

#[rstest]
fn test_decimal_field_zero_value() {
	let field = DecimalField::new("amount".to_string());

	let result = field.clean(Some(&json!(0.0)));
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), json!(0.0));
}

#[rstest]
fn test_decimal_field_negative_value() {
	let field = DecimalField::new("amount".to_string());

	let result = field.clean(Some(&json!(-100.50)));
	assert!(result.is_ok());
}

#[rstest]
fn test_date_field_builder_pattern() {
	let field = DateField::new("date".to_string())
		.with_localize(true)
		.with_locale("ja_jp".to_string());

	assert!(field.localize);
	assert_eq!(field.locale.unwrap(), "ja_jp");
}

#[rstest]
fn test_decimal_field_builder_pattern() {
	let field = DecimalField::new("price".to_string())
		.with_localize(true)
		.with_locale("fr_fr".to_string())
		.with_thousands_separator(true);

	assert!(field.localize);
	assert!(field.use_thousands_separator);
	assert_eq!(field.locale.unwrap(), "fr_fr");
}
