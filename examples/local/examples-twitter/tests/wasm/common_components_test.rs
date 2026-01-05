//! Common Components WASM Tests
//!
//! Layer 2 tests for pure UI components that don't require server functions.
//! These tests verify that actual components from `src/client/components/common.rs`
//! render correctly.
//!
//! **Run with**: `cargo make wasm-test`

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Import actual components from the application
use examples_twitter::client::components::common::{
	ButtonVariant, avatar, button, empty, error_alert, loading_spinner, success_alert, text_input,
	textarea,
};
use reinhardt::pages::component::View;
use reinhardt::pages::reactive::Signal;

// ============================================================================
// Loading Spinner Tests
// ============================================================================

/// Test loading spinner renders as a View::Element
#[wasm_bindgen_test]
fn test_loading_spinner_renders() {
	let view = loading_spinner();
	assert!(matches!(view, View::Element(_)));
}

/// Test loading spinner contains expected structure
#[wasm_bindgen_test]
fn test_loading_spinner_structure() {
	let view = loading_spinner();

	// Verify it renders without panic
	if let View::Element(element) = view {
		// Element should have class "text-center py-5"
		assert!(element.to_html().contains("spinner-border"));
	} else {
		panic!("Expected View::Element, got Fragment or None");
	}
}

// ============================================================================
// Error Alert Tests
// ============================================================================

/// Test error alert renders with message
#[wasm_bindgen_test]
fn test_error_alert_renders() {
	let view = error_alert("Test error message", false);
	assert!(matches!(view, View::Element(_)));
}

/// Test dismissible error alert
#[wasm_bindgen_test]
fn test_error_alert_dismissible() {
	let view = error_alert("Dismissible error", true);
	assert!(matches!(view, View::Element(_)));

	if let View::Element(element) = view {
		let html = element.to_html();
		// Dismissible alert should have close button
		assert!(html.contains("alert-dismissible"));
		assert!(html.contains("btn-close"));
	}
}

/// Test non-dismissible error alert
#[wasm_bindgen_test]
fn test_error_alert_non_dismissible() {
	let view = error_alert("Non-dismissible error", false);
	assert!(matches!(view, View::Element(_)));

	if let View::Element(element) = view {
		let html = element.to_html();
		// Non-dismissible alert should not have dismissible class
		assert!(!html.contains("alert-dismissible"));
		assert!(html.contains("alert-danger"));
	}
}

/// Test error alert contains message
#[wasm_bindgen_test]
fn test_error_alert_contains_message() {
	let message = "Something went wrong!";
	let view = error_alert(message, false);

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains(message));
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Success Alert Tests
// ============================================================================

/// Test success alert renders
#[wasm_bindgen_test]
fn test_success_alert_renders() {
	let view = success_alert("Operation completed successfully");
	assert!(matches!(view, View::Element(_)));
}

/// Test success alert contains message and correct class
#[wasm_bindgen_test]
fn test_success_alert_structure() {
	let message = "Tweet posted!";
	let view = success_alert(message);

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("alert-success"));
		assert!(html.contains(message));
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Avatar Tests
// ============================================================================

/// Test avatar with URL renders
#[wasm_bindgen_test]
fn test_avatar_with_url() {
	let url = Some("https://example.com/avatar.jpg");
	let view = avatar(url, "User avatar", 48);
	assert!(matches!(view, View::Element(_)));
}

/// Test avatar without URL uses placeholder
#[wasm_bindgen_test]
fn test_avatar_without_url() {
	let view = avatar(None, "Default avatar", 48);
	assert!(matches!(view, View::Element(_)));

	if let View::Element(element) = view {
		let html = element.to_html();
		// Should use placeholder URL
		assert!(html.contains("placeholder"));
		assert!(html.contains("rounded-circle"));
	}
}

/// Test avatar size attribute
#[wasm_bindgen_test]
fn test_avatar_size() {
	let view = avatar(Some("https://example.com/avatar.jpg"), "Avatar", 64);

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("64px"));
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Empty Component Tests
// ============================================================================

/// Test empty component renders
#[wasm_bindgen_test]
fn test_empty_renders() {
	let view = empty();
	assert!(matches!(view, View::Element(_)));
}

/// Test empty component produces empty div
#[wasm_bindgen_test]
fn test_empty_is_div() {
	let view = empty();

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("<div"));
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Button Variant Tests
// ============================================================================

/// Test ButtonVariant enum values
#[wasm_bindgen_test]
fn test_button_variant_classes() {
	assert_eq!(ButtonVariant::Primary.class(), "btn btn-primary");
	assert_eq!(ButtonVariant::Secondary.class(), "btn btn-secondary");
	assert_eq!(ButtonVariant::Success.class(), "btn btn-success");
	assert_eq!(ButtonVariant::Danger.class(), "btn btn-danger");
	assert_eq!(ButtonVariant::Warning.class(), "btn btn-warning");
	assert_eq!(ButtonVariant::Link.class(), "btn btn-link");
	assert_eq!(
		ButtonVariant::OutlinePrimary.class(),
		"btn btn-outline-primary"
	);
}

/// Test button component renders with different variants
#[wasm_bindgen_test]
fn test_button_variants_render() {
	let on_click = Signal::new(false);

	let variants = [
		ButtonVariant::Primary,
		ButtonVariant::Secondary,
		ButtonVariant::Success,
		ButtonVariant::Danger,
		ButtonVariant::Warning,
	];

	for variant in variants {
		let view = button("Click me", variant, false, on_click.clone());
		assert!(matches!(view, View::Element(_)));
	}
}

/// Test disabled button
#[wasm_bindgen_test]
fn test_button_disabled() {
	let on_click = Signal::new(false);
	let view = button("Disabled", ButtonVariant::Primary, true, on_click);

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("disabled"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test button text content
#[wasm_bindgen_test]
fn test_button_text_content() {
	let on_click = Signal::new(false);
	let view = button("Submit Form", ButtonVariant::Primary, false, on_click);

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("Submit Form"));
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Text Input Tests
// ============================================================================

/// Test text input renders
#[wasm_bindgen_test]
fn test_text_input_renders() {
	let value = Signal::new(String::new());
	let view = text_input("email", "Email", "Enter email", "email", value, true);
	assert!(matches!(view, View::Element(_)));
}

/// Test text input with required attribute
#[wasm_bindgen_test]
fn test_text_input_required() {
	let value = Signal::new(String::new());
	let view = text_input(
		"username",
		"Username",
		"Enter username",
		"text",
		value,
		true,
	);

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("required"));
		assert!(html.contains("form-control"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test text input with initial value
#[wasm_bindgen_test]
fn test_text_input_with_value() {
	let value = Signal::new("initial@example.com".to_string());
	let view = text_input("email", "Email", "Enter email", "email", value, false);

	if let View::Element(element) = view {
		let html = element.to_html();
		assert!(html.contains("initial@example.com"));
	} else {
		panic!("Expected View::Element");
	}
}

// ============================================================================
// Textarea Tests
// ============================================================================

/// Test textarea renders
#[wasm_bindgen_test]
fn test_textarea_renders() {
	let value = Signal::new(String::new());
	let view = textarea("content", "Tweet", "What's happening?", 3, 280, value);
	assert!(matches!(view, View::Element(_)));
}

/// Test textarea with character count
#[wasm_bindgen_test]
fn test_textarea_char_count() {
	let value = Signal::new("Hello world".to_string());
	let view = textarea("content", "Tweet", "What's happening?", 3, 280, value);

	if let View::Element(element) = view {
		let html = element.to_html();
		// Should show character count
		assert!(html.contains("11/280")); // "Hello world" is 11 chars
	} else {
		panic!("Expected View::Element");
	}
}

/// Test textarea without max length
#[wasm_bindgen_test]
fn test_textarea_no_max_length() {
	let value = Signal::new(String::new());
	let view = textarea("bio", "Bio", "Tell us about yourself", 5, 0, value);

	if let View::Element(element) = view {
		let html = element.to_html();
		// Should not show character count when max_length is 0
		assert!(!html.contains("/0"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test textarea near limit warning
#[wasm_bindgen_test]
fn test_textarea_near_limit_warning() {
	// Create a string that's 90% of 280 chars = 252 chars
	let long_text = "x".repeat(260);
	let value = Signal::new(long_text);
	let view = textarea("content", "Tweet", "What's happening?", 3, 280, value);

	if let View::Element(element) = view {
		let html = element.to_html();
		// Should have warning class when near limit
		assert!(html.contains("text-warning"));
	} else {
		panic!("Expected View::Element");
	}
}

/// Test textarea over limit danger
#[wasm_bindgen_test]
fn test_textarea_over_limit_danger() {
	// Create a string over 280 chars
	let over_limit_text = "x".repeat(300);
	let value = Signal::new(over_limit_text);
	let view = textarea("content", "Tweet", "What's happening?", 3, 280, value);

	if let View::Element(element) = view {
		let html = element.to_html();
		// Should have danger class when over limit
		assert!(html.contains("text-danger"));
	} else {
		panic!("Expected View::Element");
	}
}
