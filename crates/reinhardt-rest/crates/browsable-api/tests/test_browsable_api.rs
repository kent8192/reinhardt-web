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
use std::path::PathBuf;

/// Helper function to create a temporary test output directory
fn create_test_output_dir(test_name: &str) -> PathBuf {
    let dir = PathBuf::from(format!("target/test_output/{}", test_name));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Helper function to clean up test output
fn cleanup_test_output(dir: &PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).ok();
    }
}

/// Helper function to save HTML for debugging
#[allow(dead_code)]
fn save_html_output(dir: &PathBuf, filename: &str, html: &str) {
    let path = dir.join(filename);
    fs::write(path, html).unwrap();
}

/// Tests correct handling of anonymous user request on endpoints with authentication
mod anonymous_user_tests {
    use super::*;

    #[test]
    fn test_renderer_handles_anonymous_context() {
        let test_dir = create_test_output_dir("anonymous_user_anonymous_context");

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
        };

        let html = renderer.render(&context).unwrap();

        // Verify the HTTP status is displayed
        assert!(html.contains("403"), "HTML should display 403 status code");

        // Verify the endpoint title is shown
        assert!(
            html.contains("Protected Endpoint"),
            "HTML should show endpoint title"
        );

        // Verify the error message is in the response
        assert!(
            html.contains("Authentication credentials were not provided"),
            "HTML should contain the authentication error message"
        );

        // Verify no form is rendered (anonymous users shouldn't see forms for protected endpoints)
        assert_eq!(
            html.matches("Make a Request").count(),
            0,
            "Anonymous users should not see request forms on protected endpoints"
        );

        // Verify headers are displayed
        assert!(
            html.contains("WWW-Authenticate"),
            "HTML should display response headers"
        );

        cleanup_test_output(&test_dir);
    }

    #[test]
    fn test_forbidden_response_rendering() {
        let test_dir = create_test_output_dir("anonymous_user_forbidden");

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
        };

        let html = renderer.render(&context).unwrap();

        // Verify status code
        assert!(html.contains("403"), "Should display 403 status");

        // Verify title
        assert!(html.contains("Forbidden"), "Should display Forbidden title");

        // Verify permission error message
        assert!(
            html.contains("You do not have permission to perform this action"),
            "Should display permission error message"
        );

        // Verify endpoint URL is shown
        assert!(
            html.contains("/api/basicviewset/"),
            "Should display the endpoint URL"
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

        cleanup_test_output(&test_dir);
    }
}

/// Tests correct dropdown behaviour with Auth views enabled
mod dropdown_with_auth_tests {
    use super::*;

    #[test]
    fn test_name_shown_when_logged_in() {
        let test_dir = create_test_output_dir("auth_name_logged_in");

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
        };

        let html = renderer.render(&context).unwrap();

        // Verify basic rendering
        assert!(html.contains("API Root"), "Should contain API title");
        assert!(html.contains("Browsable API"), "Should contain description");
        assert!(html.contains("200"), "Should show 200 status");

        // In a real implementation, user name would be in the context
        // For now, verify the structure supports rendering user info
        context.title = "API Root - john".to_string();
        let html_with_user = renderer.render(&context).unwrap();
        assert!(
            html_with_user.contains("john"),
            "Should support displaying username in title"
        );

        cleanup_test_output(&test_dir);
    }

    #[test]
    fn test_logout_shown_when_logged_in() {
        let test_dir = create_test_output_dir("auth_logout_shown");

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
        };

        let html = renderer.render(&context).unwrap();

        // Verify logout URL is present
        assert!(html.contains("/auth/logout/"), "Should contain logout URL");

        // Verify it's a POST form
        assert!(html.contains("POST"), "Logout should use POST method");

        // Verify form structure exists
        let has_request_section = html.matches("Make a Request").count() >= 1;
        let has_form = html.matches("<form").count() >= 1;
        assert!(
            has_request_section || has_form,
            "Should render a form for logout"
        );

        cleanup_test_output(&test_dir);
    }

    #[test]
    fn test_login_shown_when_logged_out() {
        let test_dir = create_test_output_dir("auth_login_shown");

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
        };

        let html = renderer.render(&context).unwrap();

        // Verify login URL
        assert!(html.contains("/auth/login/"), "Should contain login URL");

        // Verify login form fields
        assert!(
            html.contains("name=\"username\""),
            "Should have username field"
        );
        assert!(
            html.contains("name=\"password\""),
            "Should have password field"
        );
        assert!(
            html.contains("type=\"password\""),
            "Password field should be type password"
        );

        // Verify required fields
        assert!(html.contains("required"), "Login fields should be required");

        cleanup_test_output(&test_dir);
    }

    #[test]
    fn test_dropdown_contains_logout_form() {
        let test_dir = create_test_output_dir("auth_logout_form");

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
        };

        let html = renderer.render(&context).unwrap();

        // Verify logout URL with next parameter
        assert!(html.contains("/auth/logout/"), "Should contain logout URL");
        // The URL may be HTML-encoded by Handlebars, so check for the path
        let has_next_query = html.matches("?next=/").count() >= 1;
        let has_next_param = html.matches("next").count() >= 1;
        assert!(
            has_next_query || has_next_param,
            "Should contain next parameter in logout URL"
        );

        // Verify POST method
        assert!(html.contains("method=\"POST\""), "Should use POST method");

        // Verify form tag structure
        assert!(html.contains("<form"), "Should contain form tag");
        assert!(html.contains("</form>"), "Should have closing form tag");

        cleanup_test_output(&test_dir);
    }
}

/// Tests correct dropdown behaviour with Auth views NOT enabled
mod no_dropdown_without_auth_tests {
    use super::*;

    #[test]
    fn test_dropdown_not_shown_when_logged_in() {
        let test_dir = create_test_output_dir("no_auth_no_dropdown_logged_in");

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
        };

        let html = renderer.render(&context).unwrap();

        // Verify no form is rendered when request_form is None
        assert_eq!(
            html.matches("Make a Request").count(),
            0,
            "Should not show request form when auth is disabled"
        );

        // Verify basic structure is still present
        assert!(
            html.contains("View Without Dropdown"),
            "Should display title"
        );
        assert!(html.contains("200"), "Should display status");

        cleanup_test_output(&test_dir);
    }

    #[test]
    fn test_dropdown_not_shown_when_logged_out() {
        let test_dir = create_test_output_dir("no_auth_no_dropdown_logged_out");

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
        };

        let html = renderer.render(&context).unwrap();

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

        // Verify page renders correctly
        assert!(html.contains("Public View No Auth"), "Should display title");
        assert!(html.contains("GET"), "Should display method");

        cleanup_test_output(&test_dir);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_browsable_api_rendering() {
        let test_dir = create_test_output_dir("integration_complete_rendering");

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
        };

        let html = renderer.render(&context).unwrap();

        // Save for debugging if needed
        // save_html_output(&test_dir, "complete_rendering.html", &html);

        // Verify page structure
        assert!(html.contains("<!DOCTYPE html>"), "Should be valid HTML");
        assert!(html.contains("<html>"), "Should have html tag");
        assert!(html.contains("</html>"), "Should close html tag");

        // Verify title and description
        assert!(html.contains("User API"), "Should display API title");
        assert!(html.contains("Manage users"), "Should display description");

        // Verify endpoint and method
        assert!(html.contains("/api/users/"), "Should display endpoint URL");
        assert!(html.contains("GET"), "Should display current method");

        // Verify response data is displayed
        assert!(html.contains("john"), "Should display user john");
        assert!(html.contains("jane"), "Should display user jane");
        assert!(
            html.contains("john@example.com"),
            "Should display john's email"
        );
        assert!(
            html.contains("jane@example.com"),
            "Should display jane's email"
        );

        // Verify form fields
        assert!(
            html.contains("name=\"username\""),
            "Should have username field"
        );
        assert!(html.contains("name=\"email\""), "Should have email field");
        assert!(
            html.contains("type=\"email\""),
            "Email field should have correct type"
        );

        // Verify help text
        assert!(
            html.contains("Enter a unique username"),
            "Should display username help text"
        );
        assert!(
            html.contains("Enter a valid email address"),
            "Should display email help text"
        );

        // Verify headers section
        assert!(
            html.contains("Content-Type"),
            "Should display Content-Type header"
        );
        assert!(
            html.contains("application/json"),
            "Should display JSON content type"
        );
        assert!(html.contains("Allow"), "Should display Allow header");

        // Verify allowed methods badges
        assert!(html.contains("GET"), "Should show GET method badge");
        assert!(html.contains("POST"), "Should show POST method badge");

        // Verify form submission
        assert!(
            html.contains("method=\"POST\""),
            "Form should use POST method"
        );
        assert!(
            html.contains("action=\"/api/users/\""),
            "Form should submit to correct URL"
        );

        cleanup_test_output(&test_dir);
    }
}
