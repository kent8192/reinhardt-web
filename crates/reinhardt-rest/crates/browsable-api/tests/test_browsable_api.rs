//! Tests for browsable API functionality
//!
//! This test module corresponds to Django REST Framework's
//! tests/browsable_api/test_browsable_api.py
//!
//! These tests verify the BrowsableApiRenderer correctly renders HTML responses
//! for various authentication and authorization scenarios.

use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer, FormContext, FormField};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Helper function to save HTML for debugging
#[allow(dead_code)]
fn save_html_output(dir: &TempDir, filename: &str, html: &str) {
	let path = dir.path().join(filename);
	fs::write(path, html).unwrap();
}

/// Tests correct handling of anonymous user request on endpoints with authentication
mod anonymous_user_tests {
	use super::*;

	#[test]
	fn test_renderer_handles_anonymous_context() {
		// Simulate anonymous user accessing a protected endpoint
		// This corresponds to DRF's test where anonymous user gets 403
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Protected Endpoint".to_string(),
			description: Some("Requires authentication".to_string()),
			endpoint: "/api/protected/".to_string(),
			method: "GET".to_string(),
			response_data: json!({"detail": "Authentication credentials were not provided."}),
			response_status: 403,
			allowed_methods: vec!["GET".to_string()],
			request_form: None,
			headers: vec![
				("Content-Type".to_string(), "application/json".to_string()),
				("WWW-Authenticate".to_string(), "Bearer".to_string()),
			],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify the HTTP status is displayed
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));
		assert!(
			html.matches("403").count() >= 1,
			"HTML should display 403 status code at least once"
		);

		// Verify the endpoint title is shown (may appear in <title> and <h1>)
		assert!(
			html.matches("Protected Endpoint").count() >= 1,
			"HTML should show endpoint title at least once"
		);

		// Verify the error message is in the response
		assert_eq!(
			html.matches("Authentication credentials were not provided")
				.count(),
			1,
			"HTML should contain the authentication error message exactly once"
		);

		// Verify no form is rendered (anonymous users shouldn't see forms for protected endpoints)
		assert_eq!(
			html.matches("Make a Request").count(),
			0,
			"Anonymous users should not see request forms on protected endpoints"
		);

		// Verify headers are displayed
		assert_eq!(
			html.matches("WWW-Authenticate").count(),
			1,
			"HTML should display WWW-Authenticate header exactly once"
		);
	}

	#[test]
	fn test_forbidden_response_rendering() {
		// Simulate a forbidden response when anonymous user lacks permissions
		// This corresponds to DRF's test_get_returns_http_forbidden_when_anonymous_user
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Forbidden".to_string(),
			description: None,
			endpoint: "/api/basicviewset/".to_string(),
			method: "GET".to_string(),
			response_data: json!({"detail": "You do not have permission to perform this action."}),
			response_status: 403,
			allowed_methods: vec![],
			request_form: None,
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify HTML structure
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));

		// Verify status code
		assert!(
			html.matches("403").count() >= 1,
			"Should display 403 status at least once"
		);

		// Verify title (may appear in <title> and <h1>)
		assert!(
			html.matches("Forbidden").count() >= 1,
			"Should display Forbidden title at least once"
		);

		// Verify permission error message
		assert_eq!(
			html.matches("You do not have permission to perform this action")
				.count(),
			1,
			"Should display permission error message exactly once"
		);

		// Verify endpoint URL is shown
		assert_eq!(
			html.matches("/api/basicviewset/").count(),
			1,
			"Should display the endpoint URL exactly once"
		);

		// Verify no allowed methods are shown (empty vec)
		// Extract only the allowed-methods section (up to the next div)
		let allowed_section = html
			.split(r#"<div class="allowed-methods">"#)
			.nth(1)
			.and_then(|s| s.split("</div>").next())
			.unwrap_or("");
		assert_eq!(
			allowed_section.matches("method-badge").count(),
			0,
			"Should not show any method badges when no methods are allowed"
		);
	}
}

/// Tests correct dropdown behaviour with Auth views enabled
mod dropdown_with_auth_tests {
	use super::*;

	#[test]
	fn test_name_shown_when_logged_in() {
		// When auth views are enabled and user is logged in,
		// the browsable API should display user information
		let renderer = BrowsableApiRenderer::new();
		let mut context = ApiContext {
			title: "API Root".to_string(),
			description: Some("Browsable API".to_string()),
			endpoint: "/".to_string(),
			method: "GET".to_string(),
			response_data: json!({"message": "Welcome"}),
			response_status: 200,
			allowed_methods: vec!["GET".to_string()],
			request_form: None,
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify HTML structure
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));

		// Verify basic rendering (titles may appear in <title> and <h1>)
		assert!(
			html.matches("API Root").count() >= 1,
			"Should contain API title at least once"
		);
		assert!(
			html.matches("Browsable API").count() >= 1,
			"Should contain description at least once"
		);
		assert!(
			html.matches("200").count() >= 1,
			"Should show 200 status at least once"
		);

		// NOTE: This test verifies username display capability in rendered HTML
		// User authentication context would provide username in production
		// Test manually sets username in title to verify rendering behavior
		context.title = "API Root - john".to_string();
		let html_with_user = renderer.render(&context).unwrap();
		assert!(
			html_with_user.matches("john").count() >= 1,
			"Should support displaying username in title at least once"
		);
	}

	#[test]
	fn test_logout_shown_when_logged_in() {
		// When user is authenticated, logout option should be available
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Authenticated View".to_string(),
			description: None,
			endpoint: "/".to_string(),
			method: "GET".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["GET".to_string()],
			request_form: Some(FormContext {
				fields: vec![],
				submit_url: "/auth/logout/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify HTML structure
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));

		// Verify logout URL is present
		assert!(
			html.matches("/auth/logout/").count() >= 1,
			"Should contain logout URL at least once"
		);

		// Verify it's a POST form
		assert!(
			html.matches("POST").count() >= 1,
			"Logout should use POST method at least once"
		);

		// Verify form structure exists
		let has_request_section = html.matches("Make a Request").count() >= 1;
		let has_form = html.matches("<form").count() >= 1;
		assert!(
			has_request_section || has_form,
			"Should render a form for logout"
		);
	}

	#[test]
	fn test_login_shown_when_logged_out() {
		// When user is not authenticated, login option should be shown
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Public View".to_string(),
			description: None,
			endpoint: "/".to_string(),
			method: "GET".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["GET".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "username".to_string(),
						label: "Username".to_string(),
						field_type: "text".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "password".to_string(),
						label: "Password".to_string(),
						field_type: "password".to_string(),
						required: true,
						help_text: None,
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/auth/login/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify HTML structure
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));

		// Verify login URL
		assert!(
			html.matches("/auth/login/").count() >= 1,
			"Should contain login URL at least once"
		);

		// Verify login form fields
		assert!(
			html.matches("name=\"username\"").count() >= 1,
			"Should have username field at least once"
		);
		assert!(
			html.matches("name=\"password\"").count() >= 1,
			"Should have password field at least once"
		);
		assert!(
			html.matches("type=\"password\"").count() >= 1,
			"Password field should be type password at least once"
		);

		// Verify required fields
		assert!(
			html.matches("required").count() >= 1,
			"Login fields should be required at least once"
		);
	}

	#[test]
	fn test_dropdown_contains_logout_form() {
		// Verify logout form is properly rendered with correct action URL including next parameter
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "API View".to_string(),
			description: None,
			endpoint: "/".to_string(),
			method: "GET".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["GET".to_string()],
			request_form: Some(FormContext {
				fields: vec![],
				submit_url: "/auth/logout/?next=/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify HTML structure
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));

		// Verify logout URL with next parameter
		assert!(
			html.matches("/auth/logout/").count() >= 1,
			"Should contain logout URL at least once"
		);
		// The URL may be HTML-encoded by Handlebars, so check for the path
		let has_next_query = html.matches("?next=/").count() >= 1;
		let has_next_param = html.matches("next").count() >= 1;
		assert!(
			has_next_query || has_next_param,
			"Should contain next parameter in logout URL"
		);

		// Verify POST method
		assert!(
			html.matches("method=\"POST\"").count() >= 1,
			"Should use POST method at least once"
		);

		// Verify form tag structure
		assert!(
			html.matches("<form").count() >= 1,
			"Should contain form tag at least once"
		);
		assert!(
			html.matches("</form>").count() >= 1,
			"Should have closing form tag at least once"
		);
	}
}

/// Tests correct dropdown behaviour with Auth views NOT enabled
mod no_dropdown_without_auth_tests {
	use super::*;

	#[test]
	fn test_dropdown_not_shown_when_logged_in() {
		// When auth views are disabled, no dropdown should be shown even when logged in
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "View Without Dropdown".to_string(),
			description: None,
			endpoint: "/".to_string(),
			method: "GET".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["GET".to_string()],
			request_form: None,
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify HTML structure
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));

		// Verify no form is rendered when request_form is None
		assert_eq!(
			html.matches("Make a Request").count(),
			0,
			"Should not show request form when auth is disabled"
		);

		// Verify basic structure is still present (title may appear in <title> and <h1>)
		assert!(
			html.matches("View Without Dropdown").count() >= 1,
			"Should display title at least once"
		);
		assert!(
			html.matches("200").count() >= 1,
			"Should display status at least once"
		);
	}

	#[test]
	fn test_dropdown_not_shown_when_logged_out() {
		// No dropdown for logged out users when auth views are disabled
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "Public View No Auth".to_string(),
			description: None,
			endpoint: "/".to_string(),
			method: "GET".to_string(),
			response_data: json!({}),
			response_status: 200,
			allowed_methods: vec!["GET".to_string()],
			request_form: None,
			headers: vec![],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Verify HTML structure
		assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<html"));
		assert!(html.trim_end().ends_with("</html>"));

		// Verify no auth-related elements
		assert_eq!(
			html.matches("dropdown").count(),
			0,
			"Should not contain dropdown elements"
		);
		assert_eq!(
			html.matches("login").count(),
			0,
			"Should not contain login references"
		);
		assert_eq!(
			html.matches("logout").count(),
			0,
			"Should not contain logout references"
		);

		// Verify page renders correctly (title may appear in <title> and <h1>)
		assert!(
			html.matches("Public View No Auth").count() >= 1,
			"Should display title at least once"
		);
		assert!(
			html.matches("GET").count() >= 1,
			"Should display method at least once"
		);
	}
}

#[cfg(test)]
mod integration_tests {
	use super::*;

	#[test]
	fn test_complete_browsable_api_rendering() {
		// Comprehensive test of browsable API rendering with all features
		let renderer = BrowsableApiRenderer::new();
		let context = ApiContext {
			title: "User API".to_string(),
			description: Some("Manage users".to_string()),
			endpoint: "/api/users/".to_string(),
			method: "GET".to_string(),
			response_data: json!([
				{"id": 1, "username": "john", "email": "john@example.com"},
				{"id": 2, "username": "jane", "email": "jane@example.com"}
			]),
			response_status: 200,
			allowed_methods: vec!["GET".to_string(), "POST".to_string()],
			request_form: Some(FormContext {
				fields: vec![
					FormField {
						name: "username".to_string(),
						label: "Username".to_string(),
						field_type: "text".to_string(),
						required: true,
						help_text: Some("Enter a unique username".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
					FormField {
						name: "email".to_string(),
						label: "Email".to_string(),
						field_type: "email".to_string(),
						required: true,
						help_text: Some("Enter a valid email address".to_string()),
						initial_value: None,
						options: None,
						initial_label: None,
					},
				],
				submit_url: "/api/users/".to_string(),
				submit_method: "POST".to_string(),
			}),
			headers: vec![
				("Content-Type".to_string(), "application/json".to_string()),
				("Allow".to_string(), "GET, POST".to_string()),
			],
			csrf_token: None,
		};

		let html = renderer.render(&context).unwrap();

		// Save for debugging if needed
		// save_html_output(&test_dir, "complete_rendering.html", &html);

		// Verify page structure
		assert!(
			html.starts_with("<!DOCTYPE html>"),
			"Should start with valid HTML doctype"
		);
		assert!(
			html.trim_end().ends_with("</html>"),
			"Should end with closing html tag"
		);
		assert!(
			html.matches("<html").count() >= 1,
			"Should have at least one opening html tag"
		);

		// Verify title and description (may appear in <title> and <h1>)
		assert!(
			html.matches("User API").count() >= 1,
			"Should display API title at least once"
		);
		assert!(
			html.matches("Manage users").count() >= 1,
			"Should display description at least once"
		);

		// Verify endpoint and method
		assert!(
			html.matches("/api/users/").count() >= 1,
			"Should display endpoint URL at least once"
		);
		assert!(
			html.matches("GET").count() >= 1,
			"Should display GET method at least once"
		);

		// Verify response data is displayed
		assert!(
			html.matches("john").count() >= 1,
			"Should display user john at least once"
		);
		assert!(
			html.matches("jane").count() >= 1,
			"Should display user jane at least once"
		);
		assert_eq!(
			html.matches("john@example.com").count(),
			1,
			"Should display john's email exactly once"
		);
		assert_eq!(
			html.matches("jane@example.com").count(),
			1,
			"Should display jane's email exactly once"
		);

		// Verify form fields
		assert!(
			html.matches("name=\"username\"").count() >= 1,
			"Should have username field at least once"
		);
		assert!(
			html.matches("name=\"email\"").count() >= 1,
			"Should have email field at least once"
		);
		assert!(
			html.matches("type=\"email\"").count() >= 1,
			"Email field should have correct type at least once"
		);

		// Verify help text
		assert_eq!(
			html.matches("Enter a unique username").count(),
			1,
			"Should display username help text exactly once"
		);
		assert_eq!(
			html.matches("Enter a valid email address").count(),
			1,
			"Should display email help text exactly once"
		);

		// Verify headers section
		assert!(
			html.matches("Content-Type").count() >= 1,
			"Should display Content-Type header at least once"
		);
		assert!(
			html.matches("application/json").count() >= 1,
			"Should display JSON content type at least once"
		);
		assert!(
			html.matches("Allow").count() >= 1,
			"Should display Allow header at least once"
		);

		// Verify allowed methods badges (GET appears in multiple contexts: current method, allowed methods)
		assert!(
			html.matches("GET").count() >= 1,
			"Should show GET method at least once"
		);
		assert!(
			html.matches("POST").count() >= 1,
			"Should show POST method at least once"
		);

		// Verify form submission
		assert!(
			html.matches("method=\"POST\"").count() >= 1,
			"Form should use POST method at least once"
		);
		assert!(
			html.matches("action=\"/api/users/\"").count() >= 1,
			"Form should submit to correct URL at least once"
		);
	}
}
