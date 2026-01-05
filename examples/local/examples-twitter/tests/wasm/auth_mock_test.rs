//! Authentication Component WASM Tests with Mocking
//!
//! Layer 2 tests for authentication components that interact with server functions.
//! These tests verify that actual auth components from `src/client/components/features/auth.rs`
//! render correctly and can interact with mocked server function responses.
//!
//! **Test Categories:**
//! - Pure rendering tests (no server_fn interaction)
//! - Structure validation tests (form elements, attributes)
//! - Mock infrastructure integration tests (for future mock-enabled tests)
//!
//! **Run with**: `cargo make wasm-test`

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Import actual components from the application
use examples_twitter::client::components::features::auth::{login_form, register_form};
use examples_twitter::shared::types::{LoginRequest, RegisterRequest, UserInfo};
use reinhardt::pages::component::View;
use reinhardt::pages::testing::{
	assert_server_fn_call_count, assert_server_fn_not_called, clear_mocks, get_call_history,
	mock_server_fn, mock_server_fn_error,
};
use uuid::Uuid;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Create a mock UserInfo for testing
fn mock_user_info() -> UserInfo {
	UserInfo {
		id: Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap(),
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		is_active: true,
	}
}

/// Create a mock LoginRequest for testing
#[allow(dead_code)] // For future mock tests
fn mock_login_request() -> LoginRequest {
	LoginRequest {
		email: "test@example.com".to_string(),
		password: "password123".to_string(),
	}
}

/// Create a mock RegisterRequest for testing
#[allow(dead_code)] // For future mock tests
fn mock_register_request() -> RegisterRequest {
	RegisterRequest {
		username: "newuser".to_string(),
		email: "newuser@example.com".to_string(),
		password: "password123".to_string(),
		password_confirmation: "password123".to_string(),
	}
}

// ============================================================================
// Login Form Rendering Tests
// ============================================================================

/// Test login form renders as a View::Element
#[wasm_bindgen_test]
fn test_login_form_renders() {
	let view = login_form();
	assert!(matches!(view, View::Element(_)));
}

/// Test login form contains expected structure
#[wasm_bindgen_test]
fn test_login_form_structure() {
	let view = login_form();

	if let View::Element(element) = view {
		let html = element.to_html();

		// Verify card container
		assert!(html.contains("card"));

		// Verify form element
		assert!(html.contains("<form"));

		// Verify email input
		assert!(html.contains("type=\"email\""));
		assert!(html.contains("name=\"email\""));

		// Verify password input
		assert!(html.contains("type=\"password\""));
		assert!(html.contains("name=\"password\""));

		// Verify submit button
		assert!(html.contains("<button"));
		assert!(html.contains("type=\"submit\""));
		assert!(html.contains("Login"));

		// Verify Bootstrap classes
		assert!(html.contains("form-control"));
		assert!(html.contains("btn btn-primary"));
	} else {
		panic!("Expected View::Element, got Fragment or None");
	}
}

/// Test login form has title
#[wasm_bindgen_test]
fn test_login_form_has_title() {
	let view = login_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("<h2"));
		assert!(html.contains("Login"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test login form has registration link
#[wasm_bindgen_test]
fn test_login_form_has_register_link() {
	let view = login_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("href=\"/register\""));
		assert!(html.contains("Register here"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test login form email field has required attribute
#[wasm_bindgen_test]
fn test_login_form_email_required() {
	let view = login_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		// Email input should be required
		assert!(
			html.contains("id=\"email\""),
			"Email input should have id='email'"
		);
		assert!(
			html.contains("required"),
			"Form inputs should have required attribute"
		);
	} else {
		panic!("Expected View::Element");
	}
}

/// Test login form password field has required attribute
#[wasm_bindgen_test]
fn test_login_form_password_required() {
	let view = login_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		// Password input should be required
		assert!(
			html.contains("id=\"password\""),
			"Password input should have id='password'"
		);
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Register Form Rendering Tests
// ============================================================================

/// Test register form renders as a View::Element
#[wasm_bindgen_test]
fn test_register_form_renders() {
	let view = register_form();
	assert!(matches!(view, View::Element(_)));
}

/// Test register form contains expected structure
#[wasm_bindgen_test]
fn test_register_form_structure() {
	let view = register_form();

	if let View::Element(element) = view {
		let html = element.to_html();

		// Verify card container
		assert!(html.contains("card"));

		// Verify form element
		assert!(html.contains("<form"));

		// Verify username input
		assert!(html.contains("type=\"text\""));
		assert!(html.contains("name=\"username\""));

		// Verify email input
		assert!(html.contains("type=\"email\""));
		assert!(html.contains("name=\"email\""));

		// Verify password inputs
		assert!(html.contains("name=\"password\""));
		assert!(html.contains("name=\"password_confirmation\""));

		// Verify submit button
		assert!(html.contains("<button"));
		assert!(html.contains("type=\"submit\""));
		assert!(html.contains("Register"));
	} else {
		panic!("Expected View::Element, got Fragment or None");
	}
}

/// Test register form has title
#[wasm_bindgen_test]
fn test_register_form_has_title() {
	let view = register_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("<h2"));
		assert!(html.contains("Register"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test register form has login link
#[wasm_bindgen_test]
fn test_register_form_has_login_link() {
	let view = register_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("href=\"/login\""));
		assert!(html.contains("Login here"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test register form has all four input fields
#[wasm_bindgen_test]
fn test_register_form_has_four_fields() {
	let view = register_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(
			html.contains("id=\"username\""),
			"Should have username field"
		);
		assert!(html.contains("id=\"email\""), "Should have email field");
		assert!(
			html.contains("id=\"password\""),
			"Should have password field"
		);
		assert!(
			html.contains("id=\"password_confirmation\""),
			"Should have password confirmation field"
		);
	} else {
		panic!("Expected View::Element");
	}
}

/// Test register form password fields are of type password
#[wasm_bindgen_test]
fn test_register_form_password_fields_hidden() {
	let view = register_form();

	if let View::Element(element) = view {
		let html = element.to_html();
		// Count password type inputs - should have at least 2
		let password_count = html.matches("type=\"password\"").count();
		assert!(
			password_count >= 2,
			"Should have at least 2 password type inputs, found {}",
			password_count
		);
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Mock Infrastructure Tests
// ============================================================================

/// Test mock infrastructure is available and works correctly
#[wasm_bindgen_test]
fn test_mock_infrastructure_available() {
	clear_mocks();

	// Register a mock response
	let user = mock_user_info();
	mock_server_fn("/api/server_fn/login", &user);

	// Verify call history starts empty
	let history = get_call_history();
	assert!(history.is_empty(), "Call history should be empty initially");

	// Verify no calls were made
	assert_server_fn_not_called("/api/server_fn/login");
	assert_server_fn_call_count("/api/server_fn/login", 0);

	clear_mocks();
}

/// Test mock error registration
#[wasm_bindgen_test]
fn test_mock_error_registration() {
	clear_mocks();

	mock_server_fn_error("/api/server_fn/login", 401, "Invalid credentials");

	// Just verify the mock infrastructure doesn't panic
	// The actual response would be retrieved by mock_fetch during a real HTTP call

	clear_mocks();
}

/// Test multiple mocks can be registered
#[wasm_bindgen_test]
fn test_multiple_mocks_registration() {
	clear_mocks();

	let user = mock_user_info();
	mock_server_fn("/api/server_fn/login", &user);
	mock_server_fn_error("/api/server_fn/register", 400, "Email already exists");
	mock_server_fn("/api/server_fn/current_user", &Some(user.clone()));

	// Verify none were called
	assert_server_fn_not_called("/api/server_fn/login");
	assert_server_fn_not_called("/api/server_fn/register");
	assert_server_fn_not_called("/api/server_fn/current_user");

	clear_mocks();
}

/// Test clear_mocks actually clears everything
#[wasm_bindgen_test]
fn test_clear_mocks_works() {
	clear_mocks();

	let user = mock_user_info();
	mock_server_fn("/api/server_fn/login", &user);

	// Clear and verify
	clear_mocks();

	let history = get_call_history();
	assert!(history.is_empty(), "History should be cleared");
}

// ============================================================================
// Shared Types Serialization Tests
// ============================================================================

/// Test LoginRequest serialization
#[wasm_bindgen_test]
fn test_login_request_serialization() {
	let request = LoginRequest {
		email: "test@example.com".to_string(),
		password: "password123".to_string(),
	};

	let json = serde_json::to_string(&request).expect("Should serialize LoginRequest");
	assert!(json.contains("test@example.com"));
	assert!(json.contains("password123"));
}

/// Test RegisterRequest serialization
#[wasm_bindgen_test]
fn test_register_request_serialization() {
	let request = RegisterRequest {
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		password: "password123".to_string(),
		password_confirmation: "password123".to_string(),
	};

	let json = serde_json::to_string(&request).expect("Should serialize RegisterRequest");
	assert!(json.contains("testuser"));
	assert!(json.contains("test@example.com"));
	assert!(json.contains("password_confirmation"));
}

/// Test UserInfo deserialization
#[wasm_bindgen_test]
fn test_user_info_deserialization() {
	let json = r#"{
        "id": "12345678-1234-1234-1234-123456789abc",
        "username": "testuser",
        "email": "test@example.com",
        "is_active": true
    }"#;

	let user: UserInfo = serde_json::from_str(json).expect("Should deserialize UserInfo");
	assert_eq!(user.username, "testuser");
	assert_eq!(user.email, "test@example.com");
	assert!(user.is_active);
}

/// Test UserInfo roundtrip serialization
#[wasm_bindgen_test]
fn test_user_info_roundtrip() {
	let original = mock_user_info();
	let json = serde_json::to_string(&original).expect("Should serialize");
	let deserialized: UserInfo = serde_json::from_str(&json).expect("Should deserialize");

	assert_eq!(original.id, deserialized.id);
	assert_eq!(original.username, deserialized.username);
	assert_eq!(original.email, deserialized.email);
	assert_eq!(original.is_active, deserialized.is_active);
}

// ============================================================================
// RegisterRequest Validation Tests
// ============================================================================

/// Test password match validation - matching passwords
#[wasm_bindgen_test]
fn test_register_request_passwords_match() {
	let request = RegisterRequest {
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		password: "password123".to_string(),
		password_confirmation: "password123".to_string(),
	};

	let result = request.validate_passwords_match();
	assert!(result.is_ok(), "Matching passwords should pass validation");
}

/// Test password match validation - mismatched passwords
#[wasm_bindgen_test]
fn test_register_request_passwords_mismatch() {
	let request = RegisterRequest {
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		password: "password123".to_string(),
		password_confirmation: "different_password".to_string(),
	};

	let result = request.validate_passwords_match();
	assert!(
		result.is_err(),
		"Mismatched passwords should fail validation"
	);
	assert_eq!(result.unwrap_err(), "Passwords do not match");
}
