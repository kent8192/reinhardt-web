//! Integration tests for typed string coercion (issue #4226).

use reinhardt_conf::settings::typed_deserializer::{CoercionError, TypedSettingsDeserializer};

use rstest::rstest;
use serde::Deserialize;
use serde_json::json;

// --- bool / int coercion ----------------------------------------------

#[derive(Debug, Deserialize, PartialEq)]
struct Scalars {
	flag: bool,
	port: u16,
	count: i64,
}

#[rstest]
#[case::all_strings(
	json!({ "flag": "true", "port": "5432", "count": "-42" }),
	Scalars { flag: true, port: 5432, count: -42 }
)]
#[case::mixed(
	json!({ "flag": true, "port": "5432", "count": -42 }),
	Scalars { flag: true, port: 5432, count: -42 }
)]
#[case::all_native(
	json!({ "flag": false, "port": 8080, "count": 0 }),
	Scalars { flag: false, port: 8080, count: 0 }
)]
fn scalar_coerce_happy(#[case] v: serde_json::Value, #[case] expected: Scalars) {
	// Arrange
	let de = TypedSettingsDeserializer::new(&v);

	// Act
	let got: Scalars = Scalars::deserialize(de).expect("coerce should succeed");

	// Assert
	assert_eq!(got, expected);
}

#[rstest]
#[case::bad_int(json!({ "flag": "true", "port": "five-thousand", "count": "0" }), "port", "u16")]
#[case::bad_bool(json!({ "flag": "yep", "port": "1", "count": "0" }), "flag", "bool")]
fn scalar_coerce_error(
	#[case] v: serde_json::Value,
	#[case] expected_key: &str,
	#[case] expected_target: &str,
) {
	// Arrange
	let de = TypedSettingsDeserializer::new(&v);

	// Act
	let err = Scalars::deserialize(de).unwrap_err();

	// Assert
	let msg = err.to_string();
	assert!(matches!(err, CoercionError::Parse { .. }), "got: {err:?}");
	assert!(msg.contains(expected_key), "msg = {msg}");
	assert!(msg.contains(expected_target), "msg = {msg}");
}
