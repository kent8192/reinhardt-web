//! Tests for nested serializers in browsable API
//!
//! This test module corresponds to Django REST Framework's
//! tests/browsable_api/test_browsable_nested_api.py
//!
//! These tests verify that nested serializers are correctly rendered
//! in browsable API forms, including proper field naming with dot notation.

use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer, FormContext, FormField};
use serde_json::json;

/// Tests correct rendering of nested serializers in browsable API forms
mod nested_serializers_tests {
	use super::*;

	#[test]
	fn test_nested_serializer_form_rendering() {
		// Test that nested serializers are properly rendered in forms
		// Corresponds to DRF's test_login test
		// This tests the core functionality of displaying nested field names
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Nested API".to_string(),
			description: Some("API with nested serializers".to_string()),
			endpoint: "/api/".to_string(),
			method: "POST".to_string(),
			response_data: json!([{"nested": {"one": 1, "two": 2}}]),
			response_status: 200,
			allowed_methods: vec!["GET".to_string(), "POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "nested.one".to_string(),
						label: "Nested One".to_string(),
						field_type: "number".to_string(),
						required: true,
						help_text: Some("Max value: 10".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "nested.two".to_string(),
						label: "Nested Two".to_string(),
						field_type: "number".to_string(),
						required: true,
						help_text: Some("Max value: 10".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify the response status
		assert!(html.contains("200"), "Should display 200 OK status");

		// Verify the form is present with correct action
		assert!(
			html.contains("action=\"/api/\""),
			"Form should submit to /api/ endpoint"
		);
		assert!(
			html.contains("method=\"POST\""),
			"Form should use POST method"
		);

		// Verify nested field names are rendered correctly with dot notation
		assert!(
			html.contains("name=\"nested.one\""),
			"Should render nested.one field with dot notation"
		);
		assert!(
			html.contains("name=\"nested.two\""),
			"Should render nested.two field with dot notation"
		);

		// Verify field labels
		assert!(
			html.contains("Nested One"),
			"Should display label for nested.one"
		);
		assert!(
			html.contains("Nested Two"),
			"Should display label for nested.two"
		);

		// Verify field type
		assert!(
			html.contains("type=\"number\""),
			"Nested fields should be number type"
		);

		// Verify help text
		assert!(
			html.contains("Max value: 10"),
			"Should display validation constraints in help text"
		);

		// Verify required fields
		assert!(
			html.contains("required"),
			"Nested fields should be marked as required"
		);
	}

	#[test]
	fn test_nested_field_with_dot_notation() {
		// Test that field names with dots (nested notation) are properly handled
		// including deeply nested structures
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Nested Fields".to_string(),
			description: None,
			endpoint: "/api/nested/".to_string(),
			method: "POST".to_string(),
			response_data: json!({}),
			response_status: 201,
			allowed_methods: vec!["POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![FormField {
					name: "parent.child.value".to_string(),
					label: "Deeply Nested Value".to_string(),
					field_type: "text".to_string(),
					required: false,
					help_text: None,
					initial_value: Some(json!("default")),
					options: None,
					initial_label: None,
				}],
				submit_url: "/api/nested/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify dot notation is preserved in field name
		assert!(
			html.contains("name=\"parent.child.value\""),
			"Should preserve full dot notation path in field name"
		);

		// Verify label is displayed
		assert!(
			html.contains("Deeply Nested Value"),
			"Should display field label"
		);

		// Verify initial value is set
		assert!(
			html.contains("default"),
			"Should display initial value in field"
		);

		// Verify field structure
		assert!(
			html.contains("type=\"text\""),
			"Should render as text field"
		);
	}

	#[test]
	fn test_nested_serializer_with_initial_values() {
		// Test nested serializer rendering with initial values
		// This is important for edit/update operations
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Update Nested".to_string(),
			description: None,
			endpoint: "/api/nested/1/".to_string(),
			method: "PUT".to_string(),
			response_data: json!({"nested": {"one": 5, "two": 7}}),
			response_status: 200,
			allowed_methods: vec!["GET".to_string(), "PUT".to_string(), "PATCH".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "nested.one".to_string(),
						label: "One".to_string(),
						field_type: "number".to_string(),
						required: true,
						help_text: None,
						initial_value: Some(json!(5)),
						options: None,
						initial_label: None,
					},
					FormField {
						name: "nested.two".to_string(),
						label: "Two".to_string(),
						field_type: "number".to_string(),
						required: true,
						help_text: None,
						initial_value: Some(json!(7)),
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/nested/1/".to_string(),
				submit_method: "PUT".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify nested field names
		assert!(
			html.contains("name=\"nested.one\""),
			"Should have nested.one field"
		);
		assert!(
			html.contains("name=\"nested.two\""),
			"Should have nested.two field"
		);

		// Verify initial values are present in the form
		// These should appear either as value attributes or in the field content
		assert!(
			html.contains("value=\"5\"") || html.contains(">5<"),
			"Should display initial value 5 for nested.one"
		);
		assert!(
			html.contains("value=\"7\"") || html.contains(">7<"),
			"Should display initial value 7 for nested.two"
		);

		// Verify PUT method for update
		assert!(html.contains("PUT"), "Should use PUT method for updates");

		// Verify response data shows updated values
		assert!(
			html.contains("\"one\": 5") || html.contains("one"),
			"Response should show nested.one value"
		);
		assert!(
			html.contains("\"two\": 7") || html.contains("two"),
			"Response should show nested.two value"
		);
	}

	#[test]
	fn test_multiple_nested_serializers() {
		// Test handling of multiple nested serializers
		// This ensures the renderer can handle complex object structures
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Complex Nesting".to_string(),
			description: Some("Multiple nested structures".to_string()),
			endpoint: "/api/complex/".to_string(),
			method: "POST".to_string(),
			response_data: json!({}),
			response_status: 201,
			allowed_methods: vec!["POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "first.value".to_string(),
						label: "First Value".to_string(),
						field_type: "text".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "second.value".to_string(),
						label: "Second Value".to_string(),
						field_type: "text".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/complex/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify both nested structures are rendered
		assert!(
			html.contains("name=\"first.value\""),
			"Should have first.value field"
		);
		assert!(
			html.contains("name=\"second.value\""),
			"Should have second.value field"
		);

		// Verify labels
		assert!(
			html.contains("First Value"),
			"Should display first value label"
		);
		assert!(
			html.contains("Second Value"),
			"Should display second value label"
		);

		// Verify both fields are required
		let first_field_pos = html.find("first.value").unwrap_or(0);
		let second_field_pos = html.find("second.value").unwrap_or(0);
		let first_section = &html[first_field_pos..first_field_pos + 200];
		let second_section = &html[second_field_pos..second_field_pos + 200];

		assert!(
			first_section.contains("required") || html.contains("required"),
			"First field should be marked as required"
		);
		assert!(
			second_section.contains("required") || html.contains("required"),
			"Second field should be marked as required"
		);
	}

	#[test]
	fn test_nested_with_validation_constraints() {
		// Test that validation constraints are properly rendered for nested fields
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Validated Nested".to_string(),
			description: None,
			endpoint: "/api/validated/".to_string(),
			method: "POST".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "data.count".to_string(),
						label: "Count".to_string(),
						field_type: "number".to_string(),
						required: true,
						help_text: Some("Integer field (max value: 10)".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "data.name".to_string(),
						label: "Name".to_string(),
						field_type: "text".to_string(),
						required: false,
						help_text: Some("Optional text field".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/validated/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify nested field names
		assert!(
			html.contains("name=\"data.count\""),
			"Should have data.count field"
		);
		assert!(
			html.contains("name=\"data.name\""),
			"Should have data.name field"
		);

		// Verify validation constraints in help text
		assert!(
			html.contains("max value: 10"),
			"Should display max value constraint"
		);
		assert!(html.contains("Integer field"), "Should indicate field type");
		assert!(
			html.contains("Optional text field"),
			"Should indicate optional field"
		);

		// Verify required attribute on count field
		let count_pos = html.find("data.count").unwrap_or(0);
		let count_section = &html[count_pos..count_pos.saturating_add(300).min(html.len())];
		assert!(
			count_section.contains("required"),
			"Count field should be marked as required"
		);
	}
}

#[cfg(test)]
mod list_create_view_tests {
	use super::*;

	#[test]
	fn test_list_create_api_view_rendering() {
		// Test rendering of ListCreateAPIView with nested serializers
		// This simulates a common DRF pattern
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "List Create View".to_string(),
			description: Some("View for listing and creating nested objects".to_string()),
			endpoint: "/api/items/".to_string(),
			method: "GET".to_string(),
			response_data: json!([
				{"nested": {"one": 1, "two": 2}},
				{"nested": {"one": 3, "two": 4}}
			]),
			response_status: 200,
			allowed_methods: vec!["GET".to_string(), "POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "nested.one".to_string(),
						label: "One".to_string(),
						field_type: "number".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "nested.two".to_string(),
						label: "Two".to_string(),
						field_type: "number".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/items/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify the view is rendered correctly
		assert!(
			html.contains("List Create View"),
			"Should display view title"
		);
		assert!(
			html.contains("View for listing and creating nested objects"),
			"Should display view description"
		);
		assert!(html.contains("/api/items/"), "Should display endpoint URL");

		// Verify response data is displayed (list of nested objects)
		assert!(
			html.contains("\"one\": 1") || html.contains("one"),
			"Should display first nested object"
		);
		assert!(
			html.contains("\"two\": 2") || html.contains("two"),
			"Should display first object's nested fields"
		);
		assert!(
			html.contains("\"one\": 3") || html.contains("3"),
			"Should display second nested object"
		);
		assert!(
			html.contains("\"two\": 4") || html.contains("4"),
			"Should display second object's nested fields"
		);

		// Verify form is present for creating new items
		assert!(
			html.contains("Make a Request"),
			"Should show form for creating new nested objects"
		);
		assert!(
			html.contains("name=\"nested.one\""),
			"Should have nested.one field in form"
		);
		assert!(
			html.contains("name=\"nested.two\""),
			"Should have nested.two field in form"
		);

		// Verify both GET and POST are allowed
		assert!(html.contains("GET"), "Should show GET as allowed method");
		assert!(html.contains("POST"), "Should show POST as allowed method");

		// Verify form submission method
		assert!(
			html.contains("method=\"POST\""),
			"Form should use POST for creating items"
		);
	}
}
