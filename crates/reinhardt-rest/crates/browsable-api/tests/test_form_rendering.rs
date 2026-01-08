//! Tests for form rendering in browsable API
//!
//! This test module corresponds to Django REST Framework's
//! tests/browsable_api/test_form_rendering.py
//!
//! These tests verify that the BrowsableApiRenderer correctly handles
//! various form rendering scenarios, including edge cases like posting
//! list data and rendering forms for views that return lists.

use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer, FormContext, FormField};
use serde_json::json;

/// POSTing a list of data to a regular view should not cause the browsable
/// API to fail during rendering.
///
/// Regression test for <https://github.com/encode/django-rest-framework/issues/5637>
mod posting_list_data_tests {
	use super::*;

	#[test]
	fn test_browsable_api_form_json_response() {
		// Sanity check for non-browsable API responses with list data
		// When POSTing list data to an endpoint expecting a dict, should get 400
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Create Item".to_string(),
			description: None,
			endpoint: "/api/items/".to_string(),
			method: "POST".to_string(),
			response_data: json!({
				"non_field_errors": ["Invalid data. Expected a dictionary, but got list."]
			}),
			response_status: 400,
			allowed_methods: vec!["GET".to_string(), "POST".to_string()],
			request_form: None,
			headers: vec![("Content-Type".to_string(), "application/json".to_string())],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify error status appears in response
		let status_400_count = html.matches("400").count();
		assert!(
			status_400_count >= 1,
			"Should display 400 Bad Request status at least once, found {} times",
			status_400_count
		);

		// Verify error message structure is present
		let non_field_errors_count = html.matches("non_field_errors").count();
		assert_eq!(
			non_field_errors_count, 1,
			"non_field_errors key should appear exactly once, found {} times",
			non_field_errors_count
		);

		let invalid_data_count = html.matches("Invalid data").count();
		assert_eq!(
			invalid_data_count, 1,
			"Validation error message should appear exactly once, found {} times",
			invalid_data_count
		);

		let list_error_count = html.matches("Expected a dictionary, but got list").count();
		assert_eq!(
			list_error_count, 1,
			"Specific error about list vs dict should appear exactly once, found {} times",
			list_error_count
		);

		// Verify basic structure
		let title_count = html.matches("Create Item").count();
		assert!(
			title_count >= 1,
			"Endpoint title should appear at least once, found {} times",
			title_count
		);

		let endpoint_count = html.matches("/api/items/").count();
		assert!(
			endpoint_count >= 1,
			"Endpoint URL should appear at least once, found {} times",
			endpoint_count
		);
	}

	#[test]
	fn test_browsable_api_with_list_data() {
		// Test that browsable API can render even when list data causes validation errors
		// The key is that rendering shouldn't crash, even with validation errors
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "API Create".to_string(),
			description: None,
			endpoint: "/api/create/?format=api".to_string(),
			method: "POST".to_string(),
			response_data: json!({
				"non_field_errors": ["Invalid data. Expected a dictionary, but got list."]
			}),
			response_status: 400,
			allowed_methods: vec!["POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![FormField {
					name: "data".to_string(),
					label: "Data".to_string(),
					field_type: "textarea".to_string(),
					required: true,
					help_text: Some("Enter valid JSON data".to_string()),
					initial_value: None,
					options: None,
					initial_label: None,
				}],
				submit_url: "/api/create/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify error status is rendered
		let status_400_count = html.matches("400").count();
		assert!(
			status_400_count >= 1,
			"Should show 400 status at least once, found {} times",
			status_400_count
		);

		let non_field_errors_count = html.matches("non_field_errors").count();
		assert_eq!(
			non_field_errors_count, 1,
			"Should display field errors exactly once, found {} times",
			non_field_errors_count
		);

		// Verify form is still rendered for retry - critical for user experience
		let make_request_count = html.matches("Make a Request").count();
		assert_eq!(
			make_request_count, 1,
			"Should still show form section exactly once after validation error, found {} times",
			make_request_count
		);

		let textarea_count = html.matches("textarea").count();
		assert!(
			textarea_count >= 1,
			"Should render textarea field for JSON input, found {} times",
			textarea_count
		);

		let data_field_count = html.matches("name=\"data\"").count();
		assert_eq!(
			data_field_count, 1,
			"Should have data field exactly once in form, found {} times",
			data_field_count
		);

		let help_text_count = html.matches("Enter valid JSON data").count();
		assert_eq!(
			help_text_count, 1,
			"Should display help text exactly once to guide user, found {} times",
			help_text_count
		);

		// Verify form can be submitted again
		let action_count = html.matches("action=\"/api/create/\"").count();
		assert_eq!(
			action_count, 1,
			"Form should submit to correct URL exactly once for retry, found {} times",
			action_count
		);

		let method_count = html.matches("method=\"POST\"").count();
		assert_eq!(
			method_count, 1,
			"Form should use POST method exactly once, found {} times",
			method_count
		);
	}

	#[test]
	fn test_list_error_response_rendering() {
		// Test that list errors are properly displayed in browsable API
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Validation Error".to_string(),
			description: Some("List data validation failed".to_string()),
			endpoint: "/api/validate/".to_string(),
			method: "POST".to_string(),
			response_data: json!({
				"non_field_errors": [
					"This field is required.",
					"Expected a dictionary but got list."
				]
			}),
			response_status: 400,
			allowed_methods: vec!["POST".to_string()],
			request_form: None,
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify all error messages are displayed
		let error_title_count = html.matches("Validation Error").count();
		assert!(
			error_title_count >= 1,
			"Should display error title at least once, found {} times",
			error_title_count
		);

		let non_field_errors_count = html.matches("non_field_errors").count();
		assert_eq!(
			non_field_errors_count, 1,
			"Should label error type exactly once, found {} times",
			non_field_errors_count
		);

		let required_error_count = html.matches("This field is required").count();
		assert_eq!(
			required_error_count, 1,
			"Should display first error message exactly once, found {} times",
			required_error_count
		);

		let dict_error_count = html.matches("Expected a dictionary but got list").count();
		assert_eq!(
			dict_error_count, 1,
			"Should display second error message exactly once, found {} times",
			dict_error_count
		);

		let status_400_count = html.matches("400").count();
		assert!(
			status_400_count >= 1,
			"Should show 400 status at least once, found {} times",
			status_400_count
		);

		// Verify proper JSON formatting of errors
		let has_array_format = html.contains("[") && html.contains("]");
		assert!(
			has_array_format,
			"Errors should be displayed as array in JSON"
		);
	}
}

/// Tests for views that return lists with many=True serializers
///
/// Regression test for <https://github.com/encode/django-rest-framework/pull/3164>
mod many_post_view_tests {
	use super::*;

	#[test]
	fn test_post_many_post_view() {
		// POST request to a view that returns a list of objects should
		// still successfully return the browsable API with a rendered form
		let renderer = BrowsableApiRenderer::new();
		let test_items = vec![
			json!({"id": 1, "text": "foo"}),
			json!({"id": 2, "text": "bar"}),
			json!({"id": 3, "text": "baz"}),
		];

		let context = ApiContext {
			title: "Items List".to_string(),
			description: Some("View returning multiple items".to_string()),
			endpoint: "/api/items/".to_string(),
			method: "POST".to_string(),
			response_data: json!(test_items),
			response_status: 200,
			allowed_methods: vec!["GET".to_string(), "POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![FormField {
					name: "text".to_string(),
					label: "Text".to_string(),
					field_type: "text".to_string(),
					required: true,
					help_text: None,
					initial_value: None,
					options: None,
					initial_label: None,
				}],
				submit_url: "/api/items/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify response status is 200
		let status_200_count = html.matches("200").count();
		assert!(
			status_200_count >= 1,
			"Should show 200 OK status at least once, found {} times",
			status_200_count
		);

		// Verify all items are rendered in the response
		let foo_count = html.matches("foo").count();
		assert_eq!(
			foo_count, 1,
			"Should display first item exactly once, found {} times",
			foo_count
		);

		let bar_count = html.matches("bar").count();
		assert_eq!(
			bar_count, 1,
			"Should display second item exactly once, found {} times",
			bar_count
		);

		let baz_count = html.matches("baz").count();
		assert_eq!(
			baz_count, 1,
			"Should display third item exactly once, found {} times",
			baz_count
		);

		// Verify list structure is maintained
		let has_id_structure = html.contains("\"id\": 1") || html.contains("id");
		assert!(
			has_id_structure,
			"Should display item IDs in JSON structure"
		);

		// Verify form is rendered despite returning a list - this is the key test
		let make_request_count = html.matches("Make a Request").count();
		assert_eq!(
			make_request_count, 1,
			"Should render form section exactly once even when response is a list, found {} times",
			make_request_count
		);

		let text_field_count = html.matches("name=\"text\"").count();
		assert_eq!(
			text_field_count, 1,
			"Should have text field exactly once in form, found {} times",
			text_field_count
		);

		let required_count = html.matches("required").count();
		assert!(
			required_count >= 1,
			"Should mark text field as required at least once, found {} times",
			required_count
		);

		// Verify form submission details
		let action_count = html.matches("action=\"/api/items/\"").count();
		assert_eq!(
			action_count, 1,
			"Form should submit to items endpoint exactly once, found {} times",
			action_count
		);

		let method_count = html.matches("method=\"POST\"").count();
		assert_eq!(
			method_count, 1,
			"Form should use POST method exactly once, found {} times",
			method_count
		);
	}

	#[test]
	fn test_many_serializer_with_form_rendering() {
		// Test that forms are correctly rendered even when response is a list
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Many Items View".to_string(),
			description: None,
			endpoint: "/api/many/".to_string(),
			method: "POST".to_string(),
			response_data: json!([
				{"id": 1, "name": "Item 1"},
				{"id": 2, "name": "Item 2"}
			]),
			response_status: 200,
			allowed_methods: vec!["GET".to_string(), "POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "id".to_string(),
						label: "ID".to_string(),
						field_type: "number".to_string(),
						required: false,
						help_text: Some("Read-only field".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "name".to_string(),
						label: "Name".to_string(),
						field_type: "text".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/many/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify list data is rendered
		let item1_count = html.matches("Item 1").count();
		assert_eq!(
			item1_count, 1,
			"Should display first item exactly once, found {} times",
			item1_count
		);

		let item2_count = html.matches("Item 2").count();
		assert_eq!(
			item2_count, 1,
			"Should display second item exactly once, found {} times",
			item2_count
		);

		// Verify both form fields are present
		let id_field_count = html.matches("name=\"id\"").count();
		assert_eq!(
			id_field_count, 1,
			"Should have ID field exactly once, found {} times",
			id_field_count
		);

		let name_field_count = html.matches("name=\"name\"").count();
		assert_eq!(
			name_field_count, 1,
			"Should have name field exactly once, found {} times",
			name_field_count
		);

		let number_type_count = html.matches("type=\"number\"").count();
		assert_eq!(
			number_type_count, 1,
			"ID should be number type exactly once, found {} times",
			number_type_count
		);

		let text_type_count = html.matches("type=\"text\"").count();
		assert_eq!(
			text_type_count, 1,
			"Name should be text type exactly once, found {} times",
			text_type_count
		);

		// Verify field attributes
		let help_text_count = html.matches("Read-only field").count();
		assert_eq!(
			help_text_count, 1,
			"Should display help text for read-only fields exactly once, found {} times",
			help_text_count
		);

		let required_count = html.matches("required").count();
		assert!(
			required_count >= 1,
			"Name field should be marked as required at least once, found {} times",
			required_count
		);
	}

	#[test]
	fn test_empty_list_response_with_form() {
		// Test rendering when response is an empty list
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Empty List".to_string(),
			description: None,
			endpoint: "/api/empty/".to_string(),
			method: "GET".to_string(),
			response_data: json!([]),
			response_status: 200,
			allowed_methods: vec!["GET".to_string(), "POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![FormField {
					name: "item".to_string(),
					label: "Item".to_string(),
					field_type: "text".to_string(),
					required: true,
					help_text: None,
					initial_value: None,
					options: None,
					initial_label: None,
				}],
				submit_url: "/api/empty/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify empty list is displayed
		let title_count = html.matches("Empty List").count();
		assert!(
			title_count >= 1,
			"Should display title at least once, found {} times",
			title_count
		);

		let has_empty_array = html.contains("[]") || html.contains("[ ]");
		assert!(has_empty_array, "Should display empty array in response");

		let status_200_count = html.matches("200").count();
		assert!(
			status_200_count >= 1,
			"Should show 200 status at least once, found {} times",
			status_200_count
		);

		// Verify form is still rendered (users can add first item)
		let make_request_count = html.matches("Make a Request").count();
		assert_eq!(
			make_request_count, 1,
			"Should show form section exactly once to add items to empty list, found {} times",
			make_request_count
		);

		let item_field_count = html.matches("name=\"item\"").count();
		assert_eq!(
			item_field_count, 1,
			"Should have item field exactly once, found {} times",
			item_field_count
		);
	}

	#[test]
	fn test_large_list_response_rendering() {
		// Test that large lists are properly rendered without issues
		let items: Vec<_> = (1..=100)
			.map(|i| json!({"id": i, "value": format!("item_{}", i)}))
			.collect();

		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Large List".to_string(),
			description: Some("100 items".to_string()),
			endpoint: "/api/large/".to_string(),
			method: "GET".to_string(),
			response_data: json!(items),
			response_status: 200,
			allowed_methods: vec!["GET".to_string()],
			request_form: None,
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify rendering succeeds
		let title_count = html.matches("Large List").count();
		assert!(
			title_count >= 1,
			"Should display title at least once, found {} times",
			title_count
		);

		let description_count = html.matches("100 items").count();
		assert!(
			description_count >= 1,
			"Should display description at least once, found {} times",
			description_count
		);

		// Verify first and last items are present
		// NOTE: Using contains() here because "item_1" substring appears in "item_10", "item_11", etc.
		// This is a justified exception to TI-5 (Assertion Strictness) because we're testing
		// that a large list (100 items) renders correctly, not the exact count of substrings.
		// For large lists, exact count matching is impractical due to overlapping substrings.
		assert!(
			html.contains("item_1"),
			"Should display first item value 'item_1' in response"
		);

		assert!(
			html.contains("item_100"),
			"Should display last item value 'item_100' in response"
		);

		assert!(
			html.contains("item_50"),
			"Should display middle item value 'item_50' in response"
		);

		// Verify that multiple items are present (checking a few specific ones)
		let has_multiple_items = html.contains("item_25") && html.contains("item_75");
		assert!(
			has_multiple_items,
			"Should display multiple items throughout the list"
		);

		// Verify structure
		let status_200_count = html.matches("200").count();
		assert!(
			status_200_count >= 1,
			"Should show 200 status at least once, found {} times",
			status_200_count
		);
	}
}

#[cfg(test)]
mod form_field_rendering_tests {
	use super::*;

	#[test]
	fn test_textarea_field_rendering() {
		// Test that textarea fields are properly rendered with correct attributes
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Text Area Test".to_string(),
			description: None,
			endpoint: "/api/textarea/".to_string(),
			method: "POST".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![FormField {
					name: "content".to_string(),
					label: "Content".to_string(),
					field_type: "textarea".to_string(),
					required: true,
					help_text: Some("Enter your content here".to_string()),
					initial_value: Some(json!("Initial content")),
					options: None,
					initial_label: None,
				}],
				submit_url: "/api/textarea/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify textarea element
		let textarea_count = html.matches("textarea").count();
		assert!(
			textarea_count >= 2,
			"Should have textarea element (opening and closing tags), found {} times",
			textarea_count
		);

		let name_attr_count = html.matches("name=\"content\"").count();
		assert_eq!(
			name_attr_count, 1,
			"Should have correct name attribute exactly once, found {} times",
			name_attr_count
		);

		let id_attr_count = html.matches("id=\"content\"").count();
		assert_eq!(
			id_attr_count, 1,
			"Should have id attribute for label exactly once, found {} times",
			id_attr_count
		);

		// Verify label
		let label_count = html.matches("Content").count();
		assert!(
			label_count >= 1,
			"Should display label text at least once, found {} times",
			label_count
		);

		// Verify initial value
		let initial_value_count = html.matches("Initial content").count();
		assert_eq!(
			initial_value_count, 1,
			"Should display initial value in textarea exactly once, found {} times",
			initial_value_count
		);

		// Verify help text
		let help_text_count = html.matches("Enter your content here").count();
		assert_eq!(
			help_text_count, 1,
			"Should display help text exactly once, found {} times",
			help_text_count
		);

		// Verify required attribute
		let required_count = html.matches("required").count();
		assert!(
			required_count >= 1,
			"Should mark field as required at least once, found {} times",
			required_count
		);
	}

	#[test]
	fn test_required_field_marking() {
		// Test that required fields are properly marked with asterisk
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Required Fields".to_string(),
			description: None,
			endpoint: "/api/required/".to_string(),
			method: "POST".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "required_field".to_string(),
						label: "Required".to_string(),
						field_type: "text".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "optional_field".to_string(),
						label: "Optional".to_string(),
						field_type: "text".to_string(),
						required: false,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/required/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Find required field section
		let required_section = if let Some(pos) = html.find("required_field") {
			&html[pos..pos.saturating_add(200).min(html.len())]
		} else {
			&html
		};

		// Verify required field has asterisk or required attribute
		let has_required_marker =
			required_section.contains("required") || required_section.contains("*");
		assert!(
			has_required_marker,
			"Required field should be marked with required attribute or asterisk"
		);

		// Verify field labels are present
		let required_label_count = html.matches("Required").count();
		assert!(
			required_label_count >= 1,
			"Should display required field label at least once, found {} times",
			required_label_count
		);

		let optional_label_count = html.matches("Optional").count();
		assert!(
			optional_label_count >= 1,
			"Should display optional field label at least once, found {} times",
			optional_label_count
		);

		// Verify both fields are rendered
		let required_field_count = html.matches("name=\"required_field\"").count();
		assert_eq!(
			required_field_count, 1,
			"Should have required field exactly once, found {} times",
			required_field_count
		);

		let optional_field_count = html.matches("name=\"optional_field\"").count();
		assert_eq!(
			optional_field_count, 1,
			"Should have optional field exactly once, found {} times",
			optional_field_count
		);
	}

	#[test]
	fn test_form_with_multiple_field_types() {
		// Test rendering of various field types in a single form
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Mixed Fields".to_string(),
			description: None,
			endpoint: "/api/mixed/".to_string(),
			method: "POST".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "text_field".to_string(),
						label: "Text".to_string(),
						field_type: "text".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "email_field".to_string(),
						label: "Email".to_string(),
						field_type: "email".to_string(),
						required: true,
						help_text: Some("Enter valid email".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "number_field".to_string(),
						label: "Number".to_string(),
						field_type: "number".to_string(),
						required: false,
						help_text: Some("Enter a number".to_string()),
						initial_value: Some(json!(42)),
						options: None,
						initial_label: None,
					},
					FormField {
						name: "textarea_field".to_string(),
						label: "Description".to_string(),
						field_type: "textarea".to_string(),
						required: false,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/mixed/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify all field types are rendered with correct type attributes
		let text_type_count = html.matches("type=\"text\"").count();
		assert_eq!(
			text_type_count, 1,
			"Should have text input exactly once, found {} times",
			text_type_count
		);

		let email_type_count = html.matches("type=\"email\"").count();
		assert_eq!(
			email_type_count, 1,
			"Should have email input exactly once, found {} times",
			email_type_count
		);

		let number_type_count = html.matches("type=\"number\"").count();
		assert_eq!(
			number_type_count, 1,
			"Should have number input exactly once, found {} times",
			number_type_count
		);

		let textarea_count = html.matches("textarea").count();
		assert!(
			textarea_count >= 2,
			"Should have textarea element (opening and closing tags), found {} times",
			textarea_count
		);

		// Verify all field names
		let text_field_count = html.matches("name=\"text_field\"").count();
		assert_eq!(
			text_field_count, 1,
			"Should have text field exactly once, found {} times",
			text_field_count
		);

		let email_field_count = html.matches("name=\"email_field\"").count();
		assert_eq!(
			email_field_count, 1,
			"Should have email field exactly once, found {} times",
			email_field_count
		);

		let number_field_count = html.matches("name=\"number_field\"").count();
		assert_eq!(
			number_field_count, 1,
			"Should have number field exactly once, found {} times",
			number_field_count
		);

		let textarea_field_count = html.matches("name=\"textarea_field\"").count();
		assert_eq!(
			textarea_field_count, 1,
			"Should have textarea field exactly once, found {} times",
			textarea_field_count
		);

		// Verify help texts
		let email_help_count = html.matches("Enter valid email").count();
		assert_eq!(
			email_help_count, 1,
			"Should show email help text exactly once, found {} times",
			email_help_count
		);

		let number_help_count = html.matches("Enter a number").count();
		assert_eq!(
			number_help_count, 1,
			"Should show number help text exactly once, found {} times",
			number_help_count
		);

		// Verify initial value for number field
		let initial_value_count = html.matches("42").count();
		assert!(
			initial_value_count >= 1,
			"Should display initial value for number field at least once, found {} times",
			initial_value_count
		);

		// Verify form structure
		let make_request_count = html.matches("Make a Request").count();
		assert_eq!(
			make_request_count, 1,
			"Should have form section exactly once, found {} times",
			make_request_count
		);

		let submit_count = html.matches("Submit").count();
		assert!(
			submit_count >= 1,
			"Should have submit button at least once, found {} times",
			submit_count
		);
	}
}
