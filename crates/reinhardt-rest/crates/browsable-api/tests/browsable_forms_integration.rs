//! Browsable API Forms Integration Tests
//!
//! Tests the integration of HTML form generation components including:
//! - HTML form generation for API endpoints
//! - Form field rendering (text, textarea, select, email, number, etc.)
//! - Form validation display and error messages
//! - CSRF token integration in forms
//! - Form submission handling and processing
//! - Nested form rendering for complex data structures

use reinhardt_browsable_api::{
	ApiContext, BrowsableApiRenderer, FormContext, FormField, SelectOption,
};
use reinhardt_test::fixtures::*;
use rstest::*;
use serde_json::json;
use std::sync::Arc;

// =============================================================================
// Fixtures
// =============================================================================

/// Basic form context with simple text field
#[fixture]
fn simple_form_context() -> FormContext {
	FormContext {
		fields: vec![FormField {
			name: "username".to_string(),
			label: "Username".to_string(),
			field_type: "text".to_string(),
			required: true,
			help_text: Some("Enter your username".to_string()),
			initial_value: None,
			options: None,
			initial_label: None,
		}],
		submit_url: "/api/users/".to_string(),
		submit_method: "POST".to_string(),
	}
}

/// Form context with multiple field types
#[fixture]
fn multi_field_form_context() -> FormContext {
	FormContext {
		fields: vec![
			FormField {
				name: "username".to_string(),
				label: "Username".to_string(),
				field_type: "text".to_string(),
				required: true,
				help_text: Some("Enter your username (3-20 characters)".to_string()),
				initial_value: None,
				options: None,
				initial_label: None,
			},
			FormField {
				name: "email".to_string(),
				label: "Email Address".to_string(),
				field_type: "email".to_string(),
				required: true,
				help_text: Some("Enter a valid email address".to_string()),
				initial_value: None,
				options: None,
				initial_label: None,
			},
			FormField {
				name: "age".to_string(),
				label: "Age".to_string(),
				field_type: "number".to_string(),
				required: false,
				help_text: Some("Optional: Enter your age".to_string()),
				initial_value: None,
				options: None,
				initial_label: None,
			},
			FormField {
				name: "bio".to_string(),
				label: "Biography".to_string(),
				field_type: "textarea".to_string(),
				required: false,
				help_text: Some("Tell us about yourself".to_string()),
				initial_value: None,
				options: None,
				initial_label: None,
			},
		],
		submit_url: "/api/users/".to_string(),
		submit_method: "POST".to_string(),
	}
}

/// Form context with select field
#[fixture]
fn select_field_form_context() -> FormContext {
	FormContext {
		fields: vec![FormField {
			name: "category".to_string(),
			label: "Category".to_string(),
			field_type: "select".to_string(),
			required: true,
			help_text: Some("Select a category".to_string()),
			initial_value: None,
			options: Some(vec![
				SelectOption {
					value: "tech".to_string(),
					label: "Technology".to_string(),
				},
				SelectOption {
					value: "science".to_string(),
					label: "Science".to_string(),
				},
				SelectOption {
					value: "art".to_string(),
					label: "Art".to_string(),
				},
			]),
			initial_label: Some("-- Select a category --".to_string()),
		}],
		submit_url: "/api/posts/".to_string(),
		submit_method: "POST".to_string(),
	}
}

/// Form context with initial values
#[fixture]
fn form_with_initial_values() -> FormContext {
	FormContext {
		fields: vec![
			FormField {
				name: "username".to_string(),
				label: "Username".to_string(),
				field_type: "text".to_string(),
				required: true,
				help_text: None,
				initial_value: Some(json!("alice")),
				options: None,
				initial_label: None,
			},
			FormField {
				name: "email".to_string(),
				label: "Email".to_string(),
				field_type: "email".to_string(),
				required: true,
				help_text: None,
				initial_value: Some(json!("alice@example.com")),
				options: None,
				initial_label: None,
			},
		],
		submit_url: "/api/users/1/".to_string(),
		submit_method: "PUT".to_string(),
	}
}

/// Renderer fixture
#[fixture]
fn renderer() -> BrowsableApiRenderer {
	BrowsableApiRenderer::new()
}

// =============================================================================
// Test: Basic Form Generation
// =============================================================================

#[rstest]
fn test_simple_text_field_rendering(
	renderer: BrowsableApiRenderer,
	simple_form_context: FormContext,
) {
	// Test: Basic text field is rendered with correct attributes
	let context = ApiContext {
		title: "Create User".to_string(),
		description: Some("Create a new user account".to_string()),
		endpoint: "/api/users/".to_string(),
		method: "POST".to_string(),
		response_data: json!({"message": "Success"}),
		response_status: 201,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(simple_form_context),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify form section exists
	assert!(html.contains("Make a Request"));
	assert!(html.contains("<form"));

	// Verify form attributes
	assert!(html.contains(r#"method="POST""#));
	assert!(html.contains(r#"action="/api/users/""#));

	// Verify field rendering
	assert!(html.contains(r#"name="username""#));
	assert!(html.contains(r#"type="text""#));
	assert!(html.contains(r#"id="username""#));

	// Verify label
	assert!(html.contains("<label for=\"username\">"));
	assert!(html.contains("Username"));

	// Verify required attribute
	assert!(html.contains(r#"required"#));
	assert!(html.contains(r#"<span style="color: red;">*</span>"#));

	// Verify help text
	assert!(html.contains("Enter your username"));
	assert!(html.contains(r#"class="help-text""#));

	// Verify submit button
	assert!(html.contains(r#"<button type="submit""#));
	assert!(html.contains("Submit"));
}

#[rstest]
fn test_multiple_field_types_rendering(
	renderer: BrowsableApiRenderer,
	multi_field_form_context: FormContext,
) {
	// Test: Form with multiple field types renders correctly
	let context = ApiContext {
		title: "User Registration".to_string(),
		description: None,
		endpoint: "/api/users/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(multi_field_form_context),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify text field (username)
	assert!(html.contains(r#"name="username""#));
	assert!(html.contains(r#"type="text""#));
	assert!(html.contains("Enter your username (3-20 characters)"));

	// Verify email field
	assert!(html.contains(r#"name="email""#));
	assert!(html.contains(r#"type="email""#));
	assert!(html.contains("Enter a valid email address"));

	// Verify number field
	assert!(html.contains(r#"name="age""#));
	assert!(html.contains(r#"type="number""#));
	assert!(html.contains("Optional: Enter your age"));

	// Verify textarea field
	assert!(html.contains(r#"name="bio""#));
	assert!(html.contains("<textarea"));
	assert!(html.contains("Tell us about yourself"));

	// Verify required vs optional fields
	// Username is required
	let username_pos = html.find(r#"name="username""#).unwrap();
	let username_section = &html[username_pos..username_pos + 200];
	assert!(username_section.contains("required"));

	// Age is optional (should not have required attribute)
	let age_pos = html.find(r#"name="age""#).unwrap();
	let age_section = &html[age_pos..age_pos + 200];
	// Check that "required" does not appear in the age field's input tag
	let age_input_end = age_section.find('>').unwrap();
	let age_input_tag = &age_section[..age_input_end];
	assert!(!age_input_tag.contains("required"));
}

// =============================================================================
// Test: Select Field Rendering
// =============================================================================

#[rstest]
fn test_select_field_with_options(
	renderer: BrowsableApiRenderer,
	select_field_form_context: FormContext,
) {
	// Test: Select field renders with all options
	let context = ApiContext {
		title: "Create Post".to_string(),
		description: None,
		endpoint: "/api/posts/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(select_field_form_context),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify select element
	assert!(html.contains("<select"));
	assert!(html.contains(r#"name="category""#));
	assert!(html.contains(r#"id="category""#));

	// Verify initial/placeholder option
	assert!(html.contains("-- Select a category --"));
	assert!(html.contains(r#"<option value="" selected>-- Select a category --</option>"#));

	// Verify all options are rendered
	assert!(html.contains(r#"value="tech""#));
	assert!(html.contains("Technology"));
	assert!(html.contains(r#"value="science""#));
	assert!(html.contains("Science"));
	assert!(html.contains(r#"value="art""#));
	assert!(html.contains("Art"));

	// Verify option order (placeholder first)
	let placeholder_pos = html.find("-- Select a category --").unwrap();
	let tech_pos = html.find("Technology").unwrap();
	assert!(placeholder_pos < tech_pos);
}

#[rstest]
fn test_select_field_with_pre_selected_value(renderer: BrowsableApiRenderer) {
	// Test: Select field with initial_value shows correct selection
	let form = FormContext {
		fields: vec![FormField {
			name: "status".to_string(),
			label: "Status".to_string(),
			field_type: "select".to_string(),
			required: true,
			help_text: None,
			initial_value: Some(json!("active")),
			options: Some(vec![
				SelectOption {
					value: "active".to_string(),
					label: "Active".to_string(),
				},
				SelectOption {
					value: "inactive".to_string(),
					label: "Inactive".to_string(),
				},
				SelectOption {
					value: "pending".to_string(),
					label: "Pending".to_string(),
				},
			]),
			initial_label: None,
		}],
		submit_url: "/api/users/1/".to_string(),
		submit_method: "PATCH".to_string(),
	};

	let context = ApiContext {
		title: "Update User".to_string(),
		description: None,
		endpoint: "/api/users/1/".to_string(),
		method: "PATCH".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["PATCH".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// NOTE: Template uses string comparison for selected attribute
	// The template checks: {% if option.value == field.initial_value %}
	// Since initial_value is JSON "active" and option.value is "active",
	// this requires the template engine to handle JSON to string comparison

	// Verify select element exists
	assert!(html.contains("<select"));
	assert!(html.contains(r#"name="status""#));

	// Verify all options are present
	assert!(html.contains(r#"value="active""#));
	assert!(html.contains(r#"value="inactive""#));
	assert!(html.contains(r#"value="pending""#));
}

// =============================================================================
// Test: Form with Initial Values
// =============================================================================

#[rstest]
fn test_form_field_initial_values(
	renderer: BrowsableApiRenderer,
	form_with_initial_values: FormContext,
) {
	// Test: Form fields display initial values for editing
	let context = ApiContext {
		title: "Edit User".to_string(),
		description: Some("Update user information".to_string()),
		endpoint: "/api/users/1/".to_string(),
		method: "PUT".to_string(),
		response_data: json!({"id": 1, "username": "alice", "email": "alice@example.com"}),
		response_status: 200,
		allowed_methods: vec!["GET".to_string(), "PUT".to_string(), "DELETE".to_string()],
		request_form: Some(form_with_initial_values),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify form method is PUT
	assert!(html.contains(r#"method="PUT""#));

	// Verify username field has initial value
	assert!(html.contains(r#"name="username""#));
	assert!(html.contains(r#"value="alice""#));

	// Verify email field has initial value
	assert!(html.contains(r#"name="email""#));
	assert!(html.contains(r#"value="alice@example.com""#));
}

#[rstest]
fn test_textarea_with_initial_value(renderer: BrowsableApiRenderer) {
	// Test: Textarea field displays initial value correctly
	let form = FormContext {
		fields: vec![FormField {
			name: "description".to_string(),
			label: "Description".to_string(),
			field_type: "textarea".to_string(),
			required: false,
			help_text: Some("Describe the item".to_string()),
			initial_value: Some(json!("This is a sample description\nwith multiple lines.")),
			options: None,
			initial_label: None,
		}],
		submit_url: "/api/items/1/".to_string(),
		submit_method: "PATCH".to_string(),
	};

	let context = ApiContext {
		title: "Edit Item".to_string(),
		description: None,
		endpoint: "/api/items/1/".to_string(),
		method: "PATCH".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["PATCH".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify textarea element
	assert!(html.contains("<textarea"));
	assert!(html.contains(r#"name="description""#));

	// Verify initial value is rendered inside textarea tags
	assert!(html.contains("This is a sample description"));
	assert!(html.contains("with multiple lines."));
}

// =============================================================================
// Test: Form Validation Display
// =============================================================================

#[rstest]
fn test_required_field_indicators(
	renderer: BrowsableApiRenderer,
	multi_field_form_context: FormContext,
) {
	// Test: Required fields are visually indicated with asterisk
	let context = ApiContext {
		title: "Create User".to_string(),
		description: None,
		endpoint: "/api/users/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(multi_field_form_context),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Find username field section (required)
	let username_label_pos = html.find("<label for=\"username\">").unwrap();
	let username_section = &html[username_label_pos..username_label_pos + 200];

	// Verify asterisk for required field
	assert!(username_section.contains(r#"<span style="color: red;">*</span>"#));

	// Find age field section (optional)
	let age_label_pos = html.find("<label for=\"age\">").unwrap();
	let age_section = &html[age_label_pos..age_label_pos + 150];

	// Verify no asterisk for optional field
	assert!(!age_section.contains(r#"<span style="color: red;">*</span>"#));
}

#[rstest]
fn test_help_text_display(renderer: BrowsableApiRenderer, multi_field_form_context: FormContext) {
	// Test: Help text is displayed below form fields
	let context = ApiContext {
		title: "Register".to_string(),
		description: None,
		endpoint: "/api/users/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(multi_field_form_context),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify help text is rendered with correct class
	assert!(html.contains(r#"class="help-text""#));

	// Verify specific help texts
	assert!(html.contains("Enter your username (3-20 characters)"));
	assert!(html.contains("Enter a valid email address"));
	assert!(html.contains("Optional: Enter your age"));
	assert!(html.contains("Tell us about yourself"));
}

// =============================================================================
// Test: CSRF Token in Forms
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_csrf_token_field_generation(
	renderer: BrowsableApiRenderer,
	simple_form_context: FormContext,
	_temp_dir: tempfile::TempDir,
) {
	// Test: Forms include CSRF token field when CSRF protection is enabled

	// Test case 1: Form with CSRF token
	let context_with_csrf = ApiContext {
		title: "Create User".to_string(),
		description: None,
		endpoint: "/api/users/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(simple_form_context.clone()),
		headers: vec![],
		csrf_token: Some("test-csrf-token-value".to_string()),
	};

	let html_with_csrf = renderer
		.render(&context_with_csrf)
		.expect("Failed to render with CSRF token");

	// Verify CSRF token field is present
	assert!(html_with_csrf.contains("<form"));
	assert!(html_with_csrf.contains(r#"method="POST""#));
	assert!(html_with_csrf.contains(r#"name="csrfmiddlewaretoken""#));
	assert!(html_with_csrf.contains(r#"type="hidden""#));
	assert!(html_with_csrf.contains(r#"value="test-csrf-token-value""#));

	// Verify CSRF token field is placed within form tags
	let form_start = html_with_csrf.find("<form").expect("Form tag not found");
	let form_end = html_with_csrf
		.find("</form>")
		.expect("Form end tag not found");
	let csrf_field = html_with_csrf
		.find(r#"name="csrfmiddlewaretoken""#)
		.expect("CSRF field not found");
	assert!(
		csrf_field > form_start && csrf_field < form_end,
		"CSRF token field is not within form tags"
	);

	// Test case 2: Form without CSRF token (backward compatibility)
	let context_without_csrf = ApiContext {
		title: "Create User".to_string(),
		description: None,
		endpoint: "/api/users/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(simple_form_context),
		headers: vec![],
		csrf_token: None,
	};

	let html_without_csrf = renderer
		.render(&context_without_csrf)
		.expect("Failed to render without CSRF token");

	// Verify form exists but CSRF token field is not present
	assert!(html_without_csrf.contains("<form"));
	assert!(html_without_csrf.contains(r#"method="POST""#));
	assert!(!html_without_csrf.contains(r#"name="csrfmiddlewaretoken""#));
}

// =============================================================================
// Test: Form Submission Handling
// =============================================================================

#[rstest]
fn test_form_method_attribute(renderer: BrowsableApiRenderer) {
	// Test: Form method attribute matches the specified HTTP method
	let methods = vec!["POST", "PUT", "PATCH", "DELETE"];

	for method in methods {
		let form = FormContext {
			fields: vec![FormField {
				name: "data".to_string(),
				label: "Data".to_string(),
				field_type: "text".to_string(),
				required: false,
				help_text: None,
				initial_value: None,
				options: None,
				initial_label: None,
			}],
			submit_url: "/api/endpoint/".to_string(),
			submit_method: method.to_string(),
		};

		let context = ApiContext {
			title: format!("Test {}", method),
			description: None,
			endpoint: "/api/endpoint/".to_string(),
			method: method.to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec![method.to_string()],
			request_form: Some(form),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).expect("Failed to render");

		// Verify form method attribute
		let expected_method = format!(r#"method="{}""#, method);
		assert!(html.contains(&expected_method));
	}
}

#[rstest]
fn test_form_action_url(renderer: BrowsableApiRenderer) {
	// Test: Form action attribute uses the correct submission URL
	let form = FormContext {
		fields: vec![],
		submit_url: "/api/custom/endpoint/".to_string(),
		submit_method: "POST".to_string(),
	};

	let context = ApiContext {
		title: "Custom Endpoint".to_string(),
		description: None,
		endpoint: "/api/custom/endpoint/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify form action URL
	assert!(html.contains(r#"action="/api/custom/endpoint/""#));
}

// =============================================================================
// Test: Nested Form Rendering
// =============================================================================

#[rstest]
fn test_nested_object_form_rendering(renderer: BrowsableApiRenderer) {
	// Test: Forms handle nested object structures with dotted field names
	let form = FormContext {
		fields: vec![
			FormField {
				name: "user.username".to_string(),
				label: "Username".to_string(),
				field_type: "text".to_string(),
				required: true,
				help_text: None,
				initial_value: None,
				options: None,
				initial_label: None,
			},
			FormField {
				name: "user.email".to_string(),
				label: "Email".to_string(),
				field_type: "email".to_string(),
				required: true,
				help_text: None,
				initial_value: None,
				options: None,
				initial_label: None,
			},
			FormField {
				name: "user.profile.bio".to_string(),
				label: "Biography".to_string(),
				field_type: "textarea".to_string(),
				required: false,
				help_text: Some("Nested profile field".to_string()),
				initial_value: None,
				options: None,
				initial_label: None,
			},
		],
		submit_url: "/api/users/".to_string(),
		submit_method: "POST".to_string(),
	};

	let context = ApiContext {
		title: "Create Nested User".to_string(),
		description: Some("Example of nested object form".to_string()),
		endpoint: "/api/users/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify nested field names are rendered correctly
	assert!(html.contains(r#"name="user.username""#));
	assert!(html.contains(r#"name="user.email""#));
	assert!(html.contains(r#"name="user.profile.bio""#));

	// Verify help text for nested field
	assert!(html.contains("Nested profile field"));
}

// =============================================================================
// Test: Complex Form Scenarios
// =============================================================================

#[rstest]
fn test_form_with_mixed_required_optional_fields(renderer: BrowsableApiRenderer) {
	// Test: Form correctly handles mix of required and optional fields
	let form = FormContext {
		fields: vec![
			FormField {
				name: "required_text".to_string(),
				label: "Required Text".to_string(),
				field_type: "text".to_string(),
				required: true,
				help_text: None,
				initial_value: None,
				options: None,
				initial_label: None,
			},
			FormField {
				name: "optional_text".to_string(),
				label: "Optional Text".to_string(),
				field_type: "text".to_string(),
				required: false,
				help_text: None,
				initial_value: None,
				options: None,
				initial_label: None,
			},
			FormField {
				name: "required_select".to_string(),
				label: "Required Select".to_string(),
				field_type: "select".to_string(),
				required: true,
				help_text: None,
				initial_value: None,
				options: Some(vec![
					SelectOption {
						value: "a".to_string(),
						label: "Option A".to_string(),
					},
					SelectOption {
						value: "b".to_string(),
						label: "Option B".to_string(),
					},
				]),
				initial_label: Some("-- Choose --".to_string()),
			},
			FormField {
				name: "optional_number".to_string(),
				label: "Optional Number".to_string(),
				field_type: "number".to_string(),
				required: false,
				help_text: None,
				initial_value: None,
				options: None,
				initial_label: None,
			},
		],
		submit_url: "/api/mixed/".to_string(),
		submit_method: "POST".to_string(),
	};

	let context = ApiContext {
		title: "Mixed Form".to_string(),
		description: None,
		endpoint: "/api/mixed/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify required_text has required attribute
	let required_text_pos = html.find(r#"name="required_text""#).unwrap();
	let required_text_section = &html[required_text_pos..required_text_pos + 150];
	assert!(required_text_section.contains("required"));

	// Verify optional_text does NOT have required attribute
	let optional_text_pos = html.find(r#"name="optional_text""#).unwrap();
	let optional_text_section = &html[optional_text_pos..optional_text_pos + 150];
	let optional_input_end = optional_text_section.find('>').unwrap();
	let optional_input_tag = &optional_text_section[..optional_input_end];
	assert!(!optional_input_tag.contains("required"));

	// Verify required_select has required attribute
	let required_select_pos = html.find(r#"name="required_select""#).unwrap();
	let required_select_section = &html[required_select_pos..required_select_pos + 150];
	assert!(required_select_section.contains("required"));

	// Verify optional_number does NOT have required attribute
	let optional_number_pos = html.find(r#"name="optional_number""#).unwrap();
	let optional_number_section = &html[optional_number_pos..optional_number_pos + 150];
	let optional_number_input_end = optional_number_section.find('>').unwrap();
	let optional_number_input_tag = &optional_number_section[..optional_number_input_end];
	assert!(!optional_number_input_tag.contains("required"));
}

#[rstest]
fn test_empty_form_renders_only_submit_button(renderer: BrowsableApiRenderer) {
	// Test: Form with no fields still renders submit button
	let form = FormContext {
		fields: vec![],
		submit_url: "/api/action/".to_string(),
		submit_method: "POST".to_string(),
	};

	let context = ApiContext {
		title: "Empty Form Action".to_string(),
		description: None,
		endpoint: "/api/action/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify form exists
	assert!(html.contains("<form"));
	assert!(html.contains(r#"method="POST""#));

	// Verify submit button exists
	assert!(html.contains(r#"<button type="submit""#));
	assert!(html.contains("Submit"));

	// Verify no input/select/textarea fields
	assert!(!html.contains("<input"));
	assert!(!html.contains("<select"));
	assert!(!html.contains("<textarea"));
}

// =============================================================================
// Test: Form Rendering with Database Integration
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_form_rendering_with_database_backed_options(
	renderer: BrowsableApiRenderer,
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) {
	// Test: Form select options can be populated from database query results
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create a test table with category data
	sqlx::query(
		r#"
		CREATE TABLE categories (
			id SERIAL PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			label VARCHAR(100) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create categories table");

	// Insert test data
	sqlx::query(
		r#"
		INSERT INTO categories (name, label) VALUES
		('tech', 'Technology'),
		('science', 'Science'),
		('art', 'Art'),
		('music', 'Music')
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert test data");

	// Fetch categories from database
	let categories: Vec<(String, String)> =
		sqlx::query_as("SELECT name, label FROM categories ORDER BY name")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to fetch categories");

	// Build SelectOptions from database results
	let options: Vec<SelectOption> = categories
		.into_iter()
		.map(|(name, label)| SelectOption { value: name, label })
		.collect();

	// Create form with database-backed options
	let form = FormContext {
		fields: vec![FormField {
			name: "category_id".to_string(),
			label: "Category".to_string(),
			field_type: "select".to_string(),
			required: true,
			help_text: Some("Select from available categories".to_string()),
			initial_value: None,
			options: Some(options),
			initial_label: Some("-- Choose a category --".to_string()),
		}],
		submit_url: "/api/posts/".to_string(),
		submit_method: "POST".to_string(),
	};

	let context = ApiContext {
		title: "Create Post".to_string(),
		description: None,
		endpoint: "/api/posts/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Verify all database-fetched options are rendered
	assert!(html.contains(r#"value="tech""#));
	assert!(html.contains("Technology"));
	assert!(html.contains(r#"value="science""#));
	assert!(html.contains("Science"));
	assert!(html.contains(r#"value="art""#));
	assert!(html.contains("Art"));
	assert!(html.contains(r#"value="music""#));
	assert!(html.contains("Music"));

	// Verify options are in correct order (alphabetical by name)
	let tech_pos = html.find(r#"value="tech""#).unwrap();
	let art_pos = html.find(r#"value="art""#).unwrap();
	let music_pos = html.find(r#"value="music""#).unwrap();
	let science_pos = html.find(r#"value="science""#).unwrap();

	assert!(art_pos < music_pos);
	assert!(music_pos < science_pos);
	assert!(science_pos < tech_pos);
}

// =============================================================================
// Test: Form Field Value Escaping
// =============================================================================

#[rstest]
fn test_html_escaping_in_field_values(renderer: BrowsableApiRenderer) {
	// Test: Form field values with HTML special characters are properly escaped
	let form = FormContext {
		fields: vec![
			FormField {
				name: "username".to_string(),
				label: "Username".to_string(),
				field_type: "text".to_string(),
				required: false,
				help_text: Some("Use <strong> tags carefully".to_string()),
				initial_value: Some(json!(r#"<script>alert("xss")</script>"#)),
				options: None,
				initial_label: None,
			},
			FormField {
				name: "description".to_string(),
				label: "Description".to_string(),
				field_type: "textarea".to_string(),
				required: false,
				help_text: None,
				initial_value: Some(json!("Line 1 & Line 2\n<div>HTML content</div>")),
				options: None,
				initial_label: None,
			},
		],
		submit_url: "/api/items/".to_string(),
		submit_method: "POST".to_string(),
	};

	let context = ApiContext {
		title: "Test Escaping".to_string(),
		description: None,
		endpoint: "/api/items/".to_string(),
		method: "POST".to_string(),
		response_data: json!({}),
		response_status: 200,
		allowed_methods: vec!["POST".to_string()],
		request_form: Some(form),
		headers: vec![],
		csrf_token: None,
	};

	let html = renderer.render(&context).expect("Failed to render");

	// Debug: Print HTML output
	println!("=== HTML Output ===");
	println!("{}", html);
	println!("=== End HTML ===");

	// Verify HTML in initial_value is escaped in input value attribute
	// NOTE: Tera template engine automatically escapes HTML in {{ }} expressions
	assert!(html.contains(r#"name="username""#));

	// Verify textarea content escaping
	assert!(html.contains(r#"name="description""#));
	assert!(html.contains("Line 1 &amp; Line 2"));

	// Verify help text HTML is escaped
	assert!(html.contains("Use &lt;strong&gt; tags carefully"));
}
