//! Basic operation tests for association proxies
//!
//! These tests verify the basic functionality of association proxies,
//! based on SQLAlchemy's tests.
//!
//! Note: These tests currently test the API surface because reinhardt-proxy's
//! actual ORM integration is not yet complete.

use reinhardt_proxy::{AssociationProxy, ProxyBuilder, ScalarValue};

#[test]
fn test_constructor() {
	// Test: Verify that the constructor works correctly
	// Based on: test_constructor from SQLAlchemy

	let proxy: AssociationProxy<(), ()> = AssociationProxy::new("children", "name");
	assert_eq!(
		proxy.relationship, "children",
		"Proxy relationship should match the constructor argument. Got: {}",
		proxy.relationship
	);
	assert_eq!(
		proxy.attribute, "name",
		"Proxy attribute should match the constructor argument. Got: {}",
		proxy.attribute
	);
	assert!(proxy.creator.is_none());
}

#[test]
fn test_with_creator() {
	// Test: Verify that proxy with creator works correctly

	fn creator(_name: String) -> i32 {
		42
	}

	let proxy = AssociationProxy::<i32, String>::new("children", "name").with_creator(creator);

	assert_eq!(
		proxy.relationship, "children",
		"Proxy relationship should match the constructor argument even with creator. Got: {}",
		proxy.relationship
	);
	assert_eq!(
		proxy.attribute, "name",
		"Proxy attribute should match the constructor argument even with creator. Got: {}",
		proxy.attribute
	);
	assert!(proxy.creator.is_some());
}

#[test]
fn test_proxy_builder_basic() {
	// Test: Verify that builder pattern works correctly

	let proxy: AssociationProxy<(), ()> = ProxyBuilder::new()
		.relationship("rel")
		.attribute("attr")
		.build();

	assert_eq!(
		proxy.relationship, "rel",
		"Builder should set relationship correctly. Got: {}",
		proxy.relationship
	);
	assert_eq!(
		proxy.attribute, "attr",
		"Builder should set attribute correctly. Got: {}",
		proxy.attribute
	);
}

#[test]
fn test_builder_with_creator() {
	// Test: Verify that builder with creator works correctly

	fn creator(_: String) -> i32 {
		42
	}

	let proxy = ProxyBuilder::new()
		.relationship("rel")
		.attribute("attr")
		.creator(creator)
		.build();

	assert!(proxy.creator.is_some());
}

#[test]
fn test_try_build_success() {
	// Test: Verify that try_build succeeds

	let proxy: Option<AssociationProxy<(), ()>> = ProxyBuilder::new()
		.relationship("rel")
		.attribute("attr")
		.try_build();

	assert!(proxy.is_some());
}

#[test]
fn test_try_build_failure() {
	// Test: Verify that try_build fails

	let proxy: Option<AssociationProxy<(), ()>> =
		ProxyBuilder::new().relationship("rel").try_build();

	assert!(proxy.is_none());
}

#[test]
#[should_panic(expected = "Attribute must be set")]
fn test_builder_panic_no_attribute() {
	// Test: Verify that builder panics when attribute is not set

	let _proxy: AssociationProxy<(), ()> = ProxyBuilder::new().relationship("rel").build();
}

#[test]
#[should_panic(expected = "Relationship must be set")]
fn test_builder_panic_no_relationship() {
	// Test: Verify that builder panics when relationship is not set

	let _proxy: AssociationProxy<(), ()> = ProxyBuilder::new().attribute("attr").build();
}

#[test]
fn test_proxy_basic_scalar_conversions() {
	// Test: Verify that ScalarValue type conversions work correctly

	let s = ScalarValue::String("test".to_string());
	assert_eq!(
		s.as_string().unwrap(),
		"test",
		"String ScalarValue should convert to string correctly. Got: {:?}",
		s.as_string().unwrap()
	);
	assert!(s.as_integer().is_err());

	let i = ScalarValue::Integer(42);
	assert_eq!(
		i.as_integer().unwrap(),
		42,
		"Integer ScalarValue should convert to i64 correctly. Got: {}",
		i.as_integer().unwrap()
	);
	assert!(i.as_string().is_err());

	let f = ScalarValue::Float(3.14);
	assert_eq!(
		f.as_float().unwrap(),
		3.14,
		"Float ScalarValue should convert to f64 correctly. Got: {}",
		f.as_float().unwrap()
	);
	assert!(f.as_boolean().is_err());

	let b = ScalarValue::Boolean(true);
	assert_eq!(
		b.as_boolean().unwrap(),
		true,
		"Boolean ScalarValue should convert to bool correctly. Got: {}",
		b.as_boolean().unwrap()
	);
	assert!(b.as_float().is_err());

	let n = ScalarValue::Null;
	assert!(n.is_null());
	assert!(n.as_string().is_err());
}

#[test]
fn test_scalar_value_from_conversions() {
	// Test: Verify that conversions to ScalarValue work correctly

	let s: ScalarValue = "test".into();
	assert_eq!(
		s.as_string().unwrap(),
		"test",
		"From<&str> conversion should create valid String ScalarValue. Got: {:?}",
		s.as_string().unwrap()
	);

	let s: ScalarValue = String::from("test").into();
	assert_eq!(
		s.as_string().unwrap(),
		"test",
		"From<String> conversion should create valid String ScalarValue. Got: {:?}",
		s.as_string().unwrap()
	);

	let i: ScalarValue = 42i64.into();
	assert_eq!(
		i.as_integer().unwrap(),
		42,
		"From<i64> conversion should create valid Integer ScalarValue. Got: {}",
		i.as_integer().unwrap()
	);

	let f: ScalarValue = 3.14f64.into();
	assert_eq!(
		f.as_float().unwrap(),
		3.14,
		"From<f64> conversion should create valid Float ScalarValue. Got: {}",
		f.as_float().unwrap()
	);

	let b: ScalarValue = true.into();
	assert_eq!(
		b.as_boolean().unwrap(),
		true,
		"From<bool> conversion should create valid Boolean ScalarValue. Got: {}",
		b.as_boolean().unwrap()
	);
}

#[test]
fn test_proxy_basic_scalar_type_mismatch() {
	// Test: Verify that type mismatch errors work correctly

	let s = ScalarValue::String("test".to_string());
	let result = s.as_integer();
	assert!(result.is_err());

	match result {
		Err(e) => {
			let error_msg = e.to_string();
			assert_eq!(
				error_msg, "Type mismatch: expected Integer, got String(\"test\")",
				"Proxy type conversion should return exact error format. Expected 'Type mismatch: expected Integer, got String(\"test\")' but got: {}",
				error_msg
			);
		}
		Ok(_) => panic!("Expected error"),
	}
}

#[test]
fn test_association_proxy_helper() {
	// Test: Verify that association_proxy helper function works correctly

	use reinhardt_proxy::builder::association_proxy;

	let proxy: AssociationProxy<(), ()> = association_proxy("users", "name");
	assert_eq!(
		proxy.relationship, "users",
		"association_proxy helper should set relationship correctly. Got: {}",
		proxy.relationship
	);
	assert_eq!(
		proxy.attribute, "name",
		"association_proxy helper should set attribute correctly. Got: {}",
		proxy.attribute
	);
}
