//! Integration tests for validator support in Model derive macro

use reinhardt_core::validators::Validate;
use reinhardt_db::orm::Model as ModelTrait;
use reinhardt_macros::model;
use rstest::rstest;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test_app", table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: i64,

	#[field(max_length = 100, email = true)]
	email: String,

	#[field(max_length = 200, url = true)]
	website: String,

	#[field(max_length = 100, min_length = 3)]
	username: String,

	#[field(min_value = 0, max_value = 120)]
	age: i32,
}

#[test]
fn test_email_validator() {
	let fields = User::field_metadata();

	let email_field = fields
		.iter()
		.find(|f| f.name == "email")
		.expect("email field should exist");

	assert!(email_field.attributes.contains_key("email"));
	assert_eq!(
		email_field.attributes.get("email"),
		Some(&reinhardt_db::orm::fields::FieldKwarg::Bool(true))
	);
}

#[test]
fn test_url_validator() {
	let fields = User::field_metadata();

	let website_field = fields
		.iter()
		.find(|f| f.name == "website")
		.expect("website field should exist");

	assert!(website_field.attributes.contains_key("url"));
	assert_eq!(
		website_field.attributes.get("url"),
		Some(&reinhardt_db::orm::fields::FieldKwarg::Bool(true))
	);
}

#[test]
fn test_min_length_validator() {
	let fields = User::field_metadata();

	let username_field = fields
		.iter()
		.find(|f| f.name == "username")
		.expect("username field should exist");

	assert!(username_field.attributes.contains_key("min_length"));
	assert_eq!(
		username_field.attributes.get("min_length"),
		Some(&reinhardt_db::orm::fields::FieldKwarg::Uint(3))
	);
}

#[test]
fn test_min_max_value_validators() {
	let fields = User::field_metadata();

	let age_field = fields
		.iter()
		.find(|f| f.name == "age")
		.expect("age field should exist");

	assert!(age_field.attributes.contains_key("min_value"));
	assert_eq!(
		age_field.attributes.get("min_value"),
		Some(&reinhardt_db::orm::fields::FieldKwarg::Int(0))
	);

	assert!(age_field.attributes.contains_key("max_value"));
	assert_eq!(
		age_field.attributes.get("max_value"),
		Some(&reinhardt_db::orm::fields::FieldKwarg::Int(120))
	);
}

#[test]
fn test_multiple_validators_on_single_field() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "products")]
	struct Product {
		#[field(primary_key = true)]
		id: i64,

		#[field(max_length = 200, min_length = 10, url = true)]
		product_url: String,

		#[field(min_value = 1, max_value = 9999)]
		price: i32,
	}

	let fields = Product::field_metadata();

	// Check product_url has multiple validators
	let url_field = fields
		.iter()
		.find(|f| f.name == "product_url")
		.expect("product_url field should exist");

	assert!(url_field.attributes.contains_key("url"));
	assert!(url_field.attributes.contains_key("min_length"));
	assert!(url_field.attributes.contains_key("max_length"));

	// Check price has min and max value validators
	let price_field = fields
		.iter()
		.find(|f| f.name == "price")
		.expect("price field should exist");

	assert!(price_field.attributes.contains_key("min_value"));
	assert!(price_field.attributes.contains_key("max_value"));
}

#[test]
fn test_no_validators() {
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "simple_model")]
	struct SimpleModel {
		#[field(primary_key = true)]
		id: i64,

		#[field(max_length = 100)]
		name: String,
	}

	let fields = SimpleModel::field_metadata();

	let name_field = fields
		.iter()
		.find(|f| f.name == "name")
		.expect("name field should exist");

	// Should only have max_length, no validator attributes
	assert!(name_field.attributes.contains_key("max_length"));
	assert!(!name_field.attributes.contains_key("email"));
	assert!(!name_field.attributes.contains_key("url"));
	assert!(!name_field.attributes.contains_key("min_length"));
	assert!(!name_field.attributes.contains_key("min_value"));
	assert!(!name_field.attributes.contains_key("max_value"));
}

// ============================================================================
// Generated validator execution
//
// `field_metadata()` only proves the validator attributes were recorded as
// metadata. The tests below exercise the `Validate` impl `#[model]` generates
// for `{Model}Info` — the code path that regressed in issue #6295, where a bound
// carrying a type suffix pinned the validator's type parameter and failed to
// compile.
//
// That impl only exists when `cfg(native)` is defined, which this package's
// `build.rs` now does. These tests are deliberately NOT `#[cfg]`-gated: if the
// alias stops being defined they must fail to compile rather than silently
// disappear, which is exactly how #6295 stayed hidden.
// ============================================================================

/// `UserInfo` with values satisfying every validator except the field under test.
fn valid_user_info() -> UserInfo {
	UserInfo {
		id: 1,
		email: "user@example.com".to_string(),
		website: "https://example.com".to_string(),
		username: "user".to_string(),
		age: 30,
	}
}

#[rstest]
fn generated_validators_accept_in_bounds_values() {
	// Arrange
	let info = valid_user_info();

	// Act
	let result = info.validate();

	// Assert
	assert!(
		result.is_ok(),
		"expected in-bounds info to validate: {result:?}"
	);
}

#[rstest]
#[case(0, true)]
#[case(120, true)]
#[case(-1, false)]
#[case(121, false)]
fn generated_range_validator_enforces_bounds(#[case] age: i32, #[case] expected_ok: bool) {
	// Arrange
	let mut info = valid_user_info();
	info.age = age;

	// Act
	let result = info.validate();

	// Assert
	assert_eq!(result.is_ok(), expected_ok, "age {age} => {result:?}");
}

#[rstest]
#[case("abc", true)]
#[case("user", true)]
#[case("ab", false)]
fn generated_length_validator_enforces_min_bound(
	#[case] username: &str,
	#[case] expected_ok: bool,
) {
	// Arrange
	let mut info = valid_user_info();
	info.username = username.to_string();

	// Act
	let result = info.validate();

	// Assert
	assert_eq!(
		result.is_ok(),
		expected_ok,
		"username {username:?} => {result:?}"
	);
}

/// Regression test for issue #6295: a `min_value` / `max_value` bound applied to
/// a 32-bit field. The pre-fix macro emitted `validate(range(min = 0i64, ...))`,
/// which pinned `MinValueValidator<T>` to `i64` and failed to compile against an
/// `Option<i32>` field with `E0308: expected &i64, found &i32`.
#[rstest]
#[case(Some(0), true)]
#[case(Some(2147483647), true)]
#[case(Some(-1), false)]
#[case(None, true)]
fn generated_range_validator_supports_32_bit_fields(
	#[case] quantity: Option<i32>,
	#[case] expected_ok: bool,
) {
	// Arrange
	#[derive(Serialize, Deserialize)]
	#[model(app_label = "test_app", table_name = "validator_range_bound_items")]
	struct RangeBoundItem {
		#[field(primary_key = true)]
		id: i64,

		#[field(null = true, min_value = 0, max_value = 2147483647)]
		quantity: Option<i32>,
	}

	let info = RangeBoundItemInfo { id: 1, quantity };

	// Act
	let result = info.validate();

	// Assert
	assert_eq!(
		result.is_ok(),
		expected_ok,
		"quantity {quantity:?} => {result:?}"
	);
}
