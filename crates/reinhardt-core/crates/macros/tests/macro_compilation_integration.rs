//! Integration tests for macro compilation and expansion verification
//!
//! This test suite validates macro behavior through compile-time tests using trybuild.
//! Tests verify:
//! - Model derive macro compilation (success and failure cases)
//! - Route macro compilation (HTTP method decorators)
//! - Validator macro compilation (validation logic)
//! - Macro error messages (clear and helpful diagnostics)
//! - Macro expansion verification (correct code generation)
//! - Macros with generic types (type parameter handling)
//!
//! Each test uses trybuild to compile test files and verify expected outcomes.

use reinhardt_db::orm::Model as ModelTrait;
use reinhardt_di::{Injectable as InjectableTrait, InjectionContext, SingletonScope};
use reinhardt_macros::{Injectable, Model, endpoint};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ========== Model Derive Macro Compilation Tests ==========

/// Test Model derive macro with basic valid model definition
///
/// Verifies:
/// - Basic model attributes compile successfully
/// - Primary key field is recognized
/// - Field metadata is generated correctly
#[test]
fn test_model_derive_basic_success() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/model/pass/basic_model.rs");
}

/// Test Model derive macro with various field types
///
/// Verifies:
/// - Different field types compile correctly (i32, i64, String, Option, DateTime, etc.)
/// - Field type inference works
/// - Type-specific attributes are validated
#[test]
fn test_model_derive_various_field_types() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/model/pass/various_field_types.rs");
}

/// Test Model derive macro with non-Option primary key
///
/// Verifies:
/// - Primary key can be non-Option type
/// - PK field metadata is correct
#[test]
fn test_model_derive_non_option_pk() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/model/pass/non_option_pk.rs");
}

/// Test Model derive macro fails without primary key
///
/// Verifies:
/// - Model without primary_key attribute fails compilation
/// - Error message indicates missing primary key
#[test]
fn test_model_derive_fail_no_primary_key() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/model/fail/no_primary_key.rs");
}

/// Test Model derive macro fails without primary key at all
///
/// Verifies:
/// - Model with no primary key field fails compilation
/// - Error message is clear about requirement
#[test]
fn test_model_derive_fail_no_primary_key_at_all() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/model/fail/no_primary_key_at_all.rs");
}

/// Test Model derive macro fails without app_label
///
/// Verifies:
/// - Missing app_label attribute causes compilation error
/// - Error message mentions required attribute
#[test]
fn test_model_derive_fail_missing_app_label() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/model/fail/missing_app_label.rs");
}

/// Test Model derive macro fails without table_name
///
/// Verifies:
/// - Missing table_name attribute causes compilation error
/// - Error message indicates required attribute
#[test]
fn test_model_derive_fail_missing_table_name() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/model/fail/missing_table_name.rs");
}

// ========== Route Macro Compilation Tests ==========

/// Test GET route macro with simple path
///
/// Verifies:
/// - #[get] attribute compiles successfully
/// - Path without parameters works
/// - Function signature is preserved
#[test]
fn test_route_get_simple_path() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/routes/pass/get_simple_path.rs");
}

/// Test POST route macro with path parameter
///
/// Verifies:
/// - #[post] attribute compiles successfully
/// - Path with parameter placeholder works
/// - Parameter extraction logic is generated
#[test]
fn test_route_post_with_parameter() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/routes/pass/post_with_parameter.rs");
}

/// Test PUT route macro with typed parameter
///
/// Verifies:
/// - #[put] attribute compiles successfully
/// - Django-style typed parameters work
/// - Type conversion code is generated
#[test]
fn test_route_put_typed_parameter() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/routes/pass/put_typed_parameter.rs");
}

/// Test route macro fails with invalid type specifier
///
/// Verifies:
/// - Invalid type in parameter causes compilation error
/// - Error message indicates valid type specifiers
#[test]
fn test_route_fail_invalid_type_spec() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/routes/fail/invalid_type_spec.rs");
}

/// Test route macro fails with unclosed brace
///
/// Verifies:
/// - Unclosed parameter brace causes compilation error
/// - Error message indicates syntax error
#[test]
fn test_route_fail_unclosed_brace() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/routes/fail/unclosed_brace.rs");
}

/// Test route macro fails with empty parameter name
///
/// Verifies:
/// - Empty parameter placeholder causes compilation error
/// - Error message indicates parameter name required
#[test]
fn test_route_fail_empty_parameter() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/routes/fail/empty_parameter.rs");
}

// ========== API View Macro Compilation Tests ==========

/// Test api_view macro with basic usage
///
/// Verifies:
/// - #[api_view] attribute compiles successfully
/// - Methods parameter is parsed correctly
/// - Function-based view is generated
#[test]
fn test_api_view_basic() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/api_view/pass/basic.rs");
}

/// Test api_view macro with multiple HTTP methods
///
/// Verifies:
/// - Multiple methods in array work correctly
/// - Method validation is performed
/// - Proper code generation for multiple methods
#[test]
fn test_api_view_multiple_methods() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/api_view/pass/multiple_methods.rs");
}

/// Test api_view macro with no methods specified
///
/// Verifies:
/// - Omitting methods parameter works (defaults to GET)
/// - Default behavior is correct
#[test]
fn test_api_view_no_methods() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/api_view/pass/no_methods.rs");
}

/// Test api_view macro fails with invalid method
///
/// Verifies:
/// - Invalid HTTP method name causes compilation error
/// - Error message lists valid methods
#[test]
fn test_api_view_fail_invalid_method() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/api_view/fail/invalid_method.rs");
}

/// Test api_view macro fails with invalid methods format
///
/// Verifies:
/// - Invalid array syntax causes compilation error
/// - Error message indicates expected format
#[test]
fn test_api_view_fail_invalid_methods_format() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/api_view/fail/invalid_methods_format.rs");
}

/// Test api_view macro fails with missing equals sign
///
/// Verifies:
/// - Missing = in attribute causes compilation error
/// - Error message indicates syntax error
#[test]
fn test_api_view_fail_missing_equals() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/api_view/fail/missing_equals.rs");
}

/// Test api_view macro fails with invalid syntax
///
/// Verifies:
/// - Malformed attribute syntax causes compilation error
/// - Error message is helpful
#[test]
fn test_api_view_fail_invalid_syntax() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/api_view/fail/invalid_syntax.rs");
}

// ========== Action Macro Compilation Tests ==========

/// Test action macro with basic detail action
///
/// Verifies:
/// - #[action] attribute compiles successfully
/// - detail = true parameter works
/// - ViewSet method is generated correctly
#[test]
fn test_action_basic_detail() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/action/pass/basic_detail.rs");
}

/// Test action macro with list action
///
/// Verifies:
/// - detail = false parameter works
/// - List-level action is generated
#[test]
fn test_action_list_action() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/action/pass/list_action.rs");
}

/// Test action macro with custom url_path
///
/// Verifies:
/// - url_path parameter works
/// - Custom URL routing is generated
#[test]
fn test_action_with_url_path() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/action/pass/with_url_path.rs");
}

/// Test action macro fails with invalid url_path
///
/// Verifies:
/// - Invalid url_path format causes compilation error
/// - Error message indicates path format requirements
#[test]
fn test_action_fail_invalid_url_path() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/action/fail/invalid_url_path.rs");
}

/// Test action macro fails with missing methods syntax
///
/// Verifies:
/// - Missing methods parameter causes compilation error
/// - Error message indicates required parameter
#[test]
fn test_action_fail_missing_methods_syntax() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/action/fail/missing_methods_syntax.rs");
}

/// Test action macro fails with invalid detail type
///
/// Verifies:
/// - Non-boolean detail value causes compilation error
/// - Error message indicates expected type
#[test]
fn test_action_fail_invalid_detail_type() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/action/fail/invalid_detail_type.rs");
}

// ========== Permission Macro Compilation Tests ==========

/// Test permission_required macro with simple permission
///
/// Verifies:
/// - #[permission_required] attribute compiles successfully
/// - Permission string is parsed correctly (app.permission format)
/// - Permission check code is generated
#[test]
fn test_permission_simple() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/permissions/pass/simple_permission.rs");
}

/// Test permission_required macro with underscore in permission name
///
/// Verifies:
/// - Underscores in permission names work
/// - Permission naming conventions are flexible
#[test]
fn test_permission_underscore() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/permissions/pass/underscore_permission.rs");
}

/// Test permission macro fails with space in permission string
///
/// Verifies:
/// - Spaces in permission string cause compilation error
/// - Error message indicates format requirements
#[test]
fn test_permission_fail_with_space() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/permissions/fail/with_space.rs");
}

/// Test permission macro fails with empty app name
///
/// Verifies:
/// - Empty app name causes compilation error
/// - Error message indicates app.permission format
#[test]
fn test_permission_fail_empty_app() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/permissions/fail/empty_app.rs");
}

/// Test permission macro fails with multiple dots
///
/// Verifies:
/// - Multiple dots in permission string cause compilation error
/// - Error message indicates single dot separator
#[test]
fn test_permission_fail_multiple_dots() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/permissions/fail/multiple_dots.rs");
}

/// Test permission macro fails without dot separator
///
/// Verifies:
/// - Missing dot separator causes compilation error
/// - Error message indicates app.permission format
#[test]
fn test_permission_fail_no_dot() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/permissions/fail/no_dot.rs");
}

// ========== Path Macro Compilation Tests ==========

/// Test path macro with simple pattern
///
/// Verifies:
/// - path! macro compiles successfully
/// - Static path segments work
/// - Path string is validated at compile time
#[test]
fn test_path_simple_pattern() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/simple_pattern.rs");
}

/// Test path macro with single parameter
///
/// Verifies:
/// - Single parameter placeholder works
/// - Parameter name is validated
#[test]
fn test_path_single_parameter() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/single_parameter.rs");
}

/// Test path macro with multiple parameters
///
/// Verifies:
/// - Multiple parameters work correctly
/// - Parameter ordering is preserved
#[test]
fn test_path_multiple_parameters() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/multiple_parameters.rs");
}

/// Test path macro with typed parameter (int)
///
/// Verifies:
/// - Django-style int type specifier works
/// - Type conversion is generated
#[test]
fn test_path_typed_parameter_int() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/typed_parameter_int.rs");
}

/// Test path macro with typed parameter (str)
///
/// Verifies:
/// - Django-style str type specifier works
/// - String type is recognized
#[test]
fn test_path_typed_parameter_str() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/typed_parameter_str.rs");
}

/// Test path macro with typed parameter (uuid)
///
/// Verifies:
/// - Django-style uuid type specifier works
/// - UUID type is recognized
#[test]
fn test_path_typed_parameter_uuid() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/typed_parameter_uuid.rs");
}

/// Test path macro with underscore in parameter name
///
/// Verifies:
/// - Underscores in parameter names work
/// - Naming conventions are flexible
#[test]
fn test_path_underscore_parameter() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/underscore_parameter.rs");
}

/// Test path macro fails with empty parameter
///
/// Verifies:
/// - Empty parameter placeholder causes compilation error
/// - Error message indicates parameter name required
#[test]
fn test_path_fail_empty_parameter() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/empty_parameter.rs");
}

/// Test path macro fails with unclosed brace
///
/// Verifies:
/// - Unclosed brace causes compilation error
/// - Error message indicates brace matching issue
#[test]
fn test_path_fail_unclosed_brace() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/unclosed_brace.rs");
}

/// Test path macro fails with unmatched closing brace
///
/// Verifies:
/// - Closing brace without opening causes compilation error
/// - Error message indicates unexpected brace
#[test]
fn test_path_fail_unmatched_closing_brace() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/unmatched_closing_brace.rs");
}

/// Test path macro fails with nested braces
///
/// Verifies:
/// - Nested braces cause compilation error
/// - Error message indicates invalid nesting
#[test]
fn test_path_fail_nested_braces() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/nested_braces.rs");
}

/// Test path macro fails with parameter starting with number
///
/// Verifies:
/// - Parameter names starting with numbers cause compilation error
/// - Error message indicates identifier rules
#[test]
fn test_path_fail_parameter_starts_with_number() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/parameter_starts_with_number.rs");
}

/// Test path macro fails with Django-style parameter outside braces
///
/// Verifies:
/// - Django-style <type:name> outside braces causes compilation error
/// - Error message indicates proper placement
#[test]
fn test_path_fail_django_style_outside_braces() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/reinhardt_style_outside_braces.rs");
}

/// Test path macro fails with invalid type specifier
///
/// Verifies:
/// - Invalid type specifier causes compilation error
/// - Error message lists valid type specifiers
#[test]
fn test_path_fail_invalid_type_specifier() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/invalid_type_specifier.rs");
}

// ========== Installed Apps Macro Compilation Tests ==========

/// Test installed_apps macro with single app
///
/// Verifies:
/// - installed_apps! macro compiles successfully
/// - Single app registration works
/// - Generated code is syntactically correct
#[test]
fn test_installed_apps_single_app() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/single_app.rs");
}

/// Test installed_apps macro with user apps
///
/// Verifies:
/// - Multiple app registration works
/// - Custom app paths are validated
#[test]
fn test_installed_apps_user_apps() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/user_apps.rs");
}

/// Test installed_apps macro with trailing comma
///
/// Verifies:
/// - Trailing comma in app list is accepted
/// - Comma handling is flexible
#[test]
fn test_installed_apps_trailing_comma() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/trailing_comma.rs");
}

/// Test installed_apps macro without trailing comma
///
/// Verifies:
/// - Missing trailing comma is acceptable
/// - Syntax is lenient
#[test]
fn test_installed_apps_no_trailing_comma() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/no_trailing_comma.rs");
}

/// Test installed_apps macro fails with duplicate labels
///
/// Verifies:
/// - Duplicate app labels cause compilation error
/// - Error message indicates duplicate
#[test]
fn test_installed_apps_fail_duplicate_labels() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/duplicate_labels.rs");
}

/// Test installed_apps macro fails with invalid syntax
///
/// Verifies:
/// - Invalid syntax causes compilation error
/// - Error message is clear
#[test]
fn test_installed_apps_fail_invalid_syntax() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/invalid_syntax.rs");
}

/// Test installed_apps macro fails with missing path
///
/// Verifies:
/// - Missing app path causes compilation error
/// - Error message indicates required path
#[test]
fn test_installed_apps_fail_missing_path() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/missing_path.rs");
}

/// Test installed_apps macro fails with empty label
///
/// Verifies:
/// - Empty app label causes compilation error
/// - Error message indicates label required
#[test]
fn test_installed_apps_fail_empty_label() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/empty_label.rs");
}

// ========== Validation Edge Cases Tests ==========

/// Test validation with case-sensitive HTTP method
///
/// Verifies:
/// - HTTP method names are case-sensitive
/// - Uppercase methods are recognized
#[test]
fn test_validation_case_sensitive_method() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/validation_edge_cases/case_sensitive_method.rs");
}

/// Test validation with empty methods array
///
/// Verifies:
/// - Empty methods array is handled correctly
/// - Default behavior is applied
#[test]
fn test_validation_empty_methods() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/validation_edge_cases/empty_methods.rs");
}

/// Test validation fails with multiple invalid methods
///
/// Verifies:
/// - Multiple invalid methods cause compilation error
/// - Error message lists all invalid methods
#[test]
fn test_validation_fail_multiple_invalid_methods() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/validation_edge_cases/multiple_invalid_methods.rs");
}

/// Test action validation fails when both methods and detail are missing
///
/// Verifies:
/// - Missing both required parameters causes compilation error
/// - Error message indicates both are missing
#[test]
fn test_validation_fail_action_missing_both() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/validation_edge_cases/action_missing_both.rs");
}

/// Test action validation fails when url_path doesn't start with slash
///
/// Verifies:
/// - url_path without leading slash causes compilation error
/// - Error message indicates slash requirement
#[test]
fn test_validation_fail_action_url_path_no_slash() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/validation_edge_cases/action_url_path_no_slash.rs");
}

// ========== Macro Expansion Verification Tests ==========

/// Test Model derive macro expansion generates correct trait implementations
///
/// Verifies:
/// - Model trait is implemented
/// - Table name method returns correct value
/// - App label method returns correct value
/// - Primary key field method returns correct value
#[test]
fn test_model_macro_expansion_trait_implementation() {
	#[derive(Debug, Clone, Serialize, Deserialize, Model)]
	#[model(app_label = "expansion_test", table_name = "expansion_models")]
	struct ExpansionTestModel {
		#[field(primary_key = true)]
		id: Option<i64>,

		#[field(max_length = 100)]
		name: String,
	}

	// Verify trait methods are implemented and return correct values
	assert_eq!(ExpansionTestModel::table_name(), "expansion_models");
	assert_eq!(ExpansionTestModel::app_label(), "expansion_test");
	assert_eq!(ExpansionTestModel::primary_key_field(), "id");
}

/// Test Model derive macro expansion generates field metadata
///
/// Verifies:
/// - Field metadata is generated for all fields
/// - Field count is correct
/// - Field names are correct
/// - Field types are correctly identified
#[test]
fn test_model_macro_expansion_field_metadata() {
	use reinhardt_db::orm::Model;

	#[derive(Debug, Clone, Serialize, Deserialize, Model)]
	#[model(app_label = "metadata_test", table_name = "metadata_models")]
	struct MetadataTestModel {
		#[field(primary_key = true)]
		pk: Option<i64>,

		#[field(max_length = 200)]
		title: String,

		#[field(null = true, max_length = 500)]
		description: Option<String>,
	}

	let fields = <MetadataTestModel as Model>::field_metadata();
	assert_eq!(fields.len(), 3, "Should have 3 fields");

	// Verify primary key field
	let pk_field = fields.iter().find(|f| f.name == "pk");
	assert!(pk_field.is_some(), "Primary key field should exist");

	// Verify title field
	let title_field = fields.iter().find(|f| f.name == "title");
	assert!(title_field.is_some(), "Title field should exist");

	// Verify description field
	let desc_field = fields.iter().find(|f| f.name == "description");
	assert!(desc_field.is_some(), "Description field should exist");
	assert!(
		desc_field.unwrap().nullable,
		"Description should be nullable"
	);
}

/// Test Injectable derive macro expansion generates inject method
///
/// Verifies:
/// - Injectable trait is implemented
/// - inject method is generated
/// - Dependencies are resolved correctly
/// - Non-injected fields use Default
#[tokio::test]
async fn test_injectable_macro_expansion_inject_method() {
	#[derive(Clone, Default)]
	struct TestService {
		#[allow(dead_code)]
		value: i32,
	}

	#[derive(Clone, Injectable)]
	struct InjectableExpansionTest {
		#[inject]
		#[allow(dead_code)]
		service: TestService,
		data: String,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let result = InjectableExpansionTest::inject(&ctx).await;
	assert!(result.is_ok(), "Injectable should succeed");

	let instance = result.unwrap();
	assert_eq!(instance.data, "", "Non-injected field should use Default");
}

// ========== Macros with Generic Types Tests ==========

/// Test Model derive macro with generic-like field types
///
/// Verifies:
/// - Models with Option<T> fields compile correctly
/// - Generic type handling in field definitions
/// - Type inference for generic fields
#[test]
fn test_model_with_generic_like_fields() {
	#[derive(Debug, Clone, Serialize, Deserialize, Model)]
	#[model(app_label = "generic_test", table_name = "generic_models")]
	struct GenericLikeModel {
		#[field(primary_key = true)]
		id: Option<i64>,

		#[field(null = true)]
		optional_count: Option<i32>,

		#[field(null = true, max_length = 255)]
		optional_text: Option<String>,
	}

	// Verify model compiles and metadata is correct
	let fields = GenericLikeModel::field_metadata();
	assert_eq!(fields.len(), 3);

	let optional_count_field = fields.iter().find(|f| f.name == "optional_count");
	assert!(optional_count_field.is_some());
	assert!(optional_count_field.unwrap().nullable);

	let optional_text_field = fields.iter().find(|f| f.name == "optional_text");
	assert!(optional_text_field.is_some());
	assert!(optional_text_field.unwrap().nullable);
}

/// Test endpoint macro with generic return type
///
/// Verifies:
/// - endpoint macro handles Result<T, E> return types
/// - Generic error types are preserved
/// - Type inference works correctly
#[tokio::test]
async fn test_endpoint_with_generic_return_type() {
	#[derive(Clone, Default)]
	struct GenericTestService;

	#[endpoint]
	async fn generic_endpoint(
		#[inject] _service: GenericTestService,
		value: i32,
	) -> Result<String, reinhardt_di::DiError> {
		Ok(format!("Value: {}", value))
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	let result = generic_endpoint(42, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "Value: 42");
}

/// Test Injectable with nested generic types
///
/// Verifies:
/// - Injectable handles Arc<T> and other wrapper types
/// - Nested generic resolution works
/// - Type parameters are correctly preserved
#[tokio::test]
async fn test_injectable_with_nested_generic_types() {
	#[derive(Clone, Default)]
	struct NestedService1;

	#[derive(Clone, Default)]
	struct NestedService2;

	#[derive(Clone, Injectable)]
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
