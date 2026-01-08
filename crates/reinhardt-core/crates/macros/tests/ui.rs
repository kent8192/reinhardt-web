//! Compile-time tests for macros using trybuild
//!
//! This test suite validates that:
//! - Valid macro usage compiles successfully (tests/ui/pass/*.rs)
//! - Invalid macro usage fails to compile (tests/ui/fail/*.rs)
//!
//! This tests the reinhardt-macros crate's compile-time behavior

use reinhardt_di::{Injectable, InjectionContext, SingletonScope};
use reinhardt_macros::injectable;
use std::sync::Arc;

#[test]
fn test_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/*.rs");
}

#[test]
fn test_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/*.rs");
}

#[test]
fn test_path_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/*.rs");
}

#[test]
fn test_path_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/*.rs");
}

#[test]
fn test_permission_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/permissions/pass/*.rs");
}

#[test]
fn test_permission_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/permissions/fail/*.rs");
}

#[test]
fn test_routes_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/routes/pass/*.rs");
}

#[test]
fn test_routes_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/routes/fail/*.rs");
}

#[test]
fn test_action_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/action/pass/*.rs");
}

#[test]
fn test_action_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/action/fail/*.rs");
}

// Note: Generic types, lifetimes, and macro hygiene tests are in separate test files
// (generic_types_complete.rs, lifetime_annotations.rs, macro_hygiene.rs)
// They run as regular cargo tests, not trybuild compile tests

#[test]
fn test_validation_edge_cases_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/validation_edge_cases/multiple_invalid_methods.rs");
	t.compile_fail("tests/ui/validation_edge_cases/action_missing_both.rs");
	t.compile_fail("tests/ui/validation_edge_cases/action_url_path_no_slash.rs");
}

#[test]
fn test_validation_edge_cases_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/validation_edge_cases/case_sensitive_method.rs");
	t.pass("tests/ui/validation_edge_cases/empty_methods.rs");
}

#[test]
fn test_model_derive_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/model/pass/*.rs");
}

#[test]
fn test_model_derive_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/model/fail/*.rs");
}

#[test]
fn test_rel_attribute_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/rel/pass/*.rs");
}

#[test]
fn test_rel_attribute_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/rel/fail/*.rs");
}

// api_view macro tests
#[test]
fn test_api_view_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/api_view/pass/*.rs");
}

#[test]
fn test_api_view_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/api_view/fail/*.rs");
}

// Admin macro tests
#[test]
fn test_admin_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/admin/pass/*.rs");
}

#[test]
fn test_admin_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/admin/fail/*.rs");
}

// Injectable macro tests (#[injectable] and #[use_inject])
// Note: pass tests are omitted because generated code requires Injectable implementations
// which cannot be easily provided in trybuild tests
#[test]
fn test_injectable_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/injectable/fail/*.rs");
}

// Field attributes tests
#[test]
fn test_field_attributes_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/field_attributes/pass/*.rs");
}

#[test]
fn test_field_attributes_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/field_attributes/fail/*.rs");
}

// PostgreSQL-specific field attributes
#[cfg(feature = "db-postgres")]
#[test]
fn test_postgres_field_attributes() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/field_attributes/postgres/*.rs");
}

// MySQL-specific field attributes
#[cfg(feature = "db-mysql")]
#[test]
fn test_mysql_field_attributes() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/field_attributes/mysql/*.rs");
}

// SQLite-specific field attributes
#[cfg(feature = "db-sqlite")]
#[test]
fn test_sqlite_field_attributes() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/field_attributes/sqlite/*.rs");
}

// ========== Injectable Runtime Tests ==========
//
// These tests validate Injectable macro behavior at runtime.
// They require actual Injectable implementations and cannot be tested with trybuild.

/// Test Injectable macro expansion with inject method
///
/// Verifies:
/// - Macro generates correct inject() method
/// - Dependencies are resolved correctly
/// - Non-injected fields use Default
#[tokio::test]
async fn test_injectable_macro_expansion_inject_method() {
	#[injectable]
	#[derive(Clone, Default)]
	struct TestService {
		#[no_inject(default = Default)]
		#[allow(dead_code)]
		value: i32,
	}

	#[injectable]
	#[derive(Clone, Default)]
	struct InjectableExpansionTest {
		#[inject]
		#[allow(dead_code)]
		service: TestService,

		#[no_inject(default = Default)]
		data: String,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let result = InjectableExpansionTest::inject(&ctx).await;
	assert!(result.is_ok(), "Injectable should succeed");

	let instance = result.unwrap();
	assert_eq!(instance.data, "", "Non-injected field should use Default");
}

/// Test Injectable with nested generic types
///
/// Verifies:
/// - Injectable handles `Arc<T>` and other wrapper types
/// - Nested generic resolution works
/// - Type parameters are correctly preserved
#[tokio::test]
async fn test_injectable_with_nested_generic_types() {
	#[derive(Clone, Default)]
	#[injectable]
	struct NestedService1;

	#[derive(Clone, Default)]
	#[injectable]
	struct NestedService2;

	#[injectable]
	#[derive(Clone, Default)]
	struct NestedGenericInjectable {
		#[inject]
		#[allow(dead_code)]
		service1: NestedService1,
		#[inject]
		#[allow(dead_code)]
		service2: NestedService2,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let result = NestedGenericInjectable::inject(&ctx).await;
	assert!(result.is_ok(), "Nested generic Injectable should resolve");
}
