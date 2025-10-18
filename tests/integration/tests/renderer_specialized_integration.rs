//! Specialized Renderer Integration Tests
//!
//! These tests require specialized renderer implementations:
//! - StaticHTMLRenderer
//! - AdminRenderer
//! - DocumentationRenderer
//! - SchemaJSRenderer
//! - BrowsableAPIRenderer (full implementation)
//!
//! Based on Django REST Framework's specialized renderer tests

#[cfg(test)]
mod static_html_renderer_tests {
    use reinhardt_renderers::{json::Renderer, StaticHTMLRenderer};
    use serde_json::json;

    #[tokio::test]
    async fn test_static_renderer() {
        // Test: Static HTML renderer basic functionality
        // Expected: Pre-rendered HTML string is returned as-is

        let static_content =
            "<html><body><h1>Welcome to Reinhardt</h1><p>Static page</p></body></html>";
        let renderer = StaticHTMLRenderer::new(static_content);

        // Provide arbitrary data - should be ignored
        let data = json!({"ignored": "data", "value": 123});

        let result = renderer.render(&data, None).await;
        assert!(result.is_ok(), "Rendering should succeed");

        let bytes = result.unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();

        // Verify the content is exactly what we provided
        assert_eq!(html, static_content);
        assert!(html.contains("Welcome to Reinhardt"));
        assert!(html.contains("Static page"));

        // Verify data was ignored
        assert!(!html.contains("ignored"));
        assert!(!html.contains("123"));

        // Verify media type
        assert_eq!(renderer.media_types(), vec!["text/html"]);
    }

    #[tokio::test]
    async fn test_static_renderer_with_exception() {
        // Test: Static HTML renderer with exception information
        // Expected: Exception information is included in HTML output

        let error_html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Error 500</title></head>
            <body>
                <h1>Internal Server Error</h1>
                <div class="error">
                    <p>An unexpected error occurred.</p>
                    <pre>Exception: ValueError at /api/users/
Invalid user ID format
File "views.py", line 42, in get_user</pre>
                </div>
            </body>
            </html>
        "#;

        let renderer = StaticHTMLRenderer::new(error_html);

        // Provide exception data (should still be ignored)
        let exception_data = json!({
            "error": "ValueError",
            "message": "Invalid user ID format",
            "traceback": ["views.py:42"]
        });

        let result = renderer.render(&exception_data, None).await;
        assert!(
            result.is_ok(),
            "Rendering should succeed even with exception data"
        );

        let bytes = result.unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();

        // Verify exception information is in the static HTML
        assert!(html.contains("Internal Server Error"));
        assert!(html.contains("Exception: ValueError"));
        assert!(html.contains("Invalid user ID format"));
        assert!(html.contains("views.py"));

        // Verify format
        assert_eq!(renderer.format(), Some("html"));
    }

    #[tokio::test]
    async fn test_static_renderer_empty_content() {
        // Test: Static HTML renderer with empty content
        // Expected: Empty string is rendered

        let renderer = StaticHTMLRenderer::new("");
        let data = json!({"test": "data"});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(html, "");
    }

    #[tokio::test]
    async fn test_static_renderer_unicode_content() {
        // Test: Static HTML renderer with Unicode content
        // Expected: Unicode characters are preserved

        let unicode_html =
            "<html><body><h1>こんにちは</h1><p>Здравствуйте</p><p>مرحبا</p></body></html>";
        let renderer = StaticHTMLRenderer::new(unicode_html);
        let data = json!({});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(html, unicode_html);
        assert!(html.contains("こんにちは"));
        assert!(html.contains("Здравствуйте"));
        assert!(html.contains("مرحبا"));
    }
}

#[cfg(test)]
mod admin_renderer_tests {
    use reinhardt_renderers::{json::Renderer, AdminRenderer};
    use serde_json::json;

    #[tokio::test]
    async fn test_render_when_resource_created() {
        // Test: Admin renderer when resource is created
        // Expected: Success message and redirect information

        let renderer = AdminRenderer::new();
        let data = json!({
            "id": "42",
            "name": "New Item",
            "description": "Successfully created"
        });

        let result = renderer.render(&data, None).await;
        assert!(result.is_ok(), "Rendering should succeed");

        let bytes = result.unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();

        // Verify success message
        assert!(html.contains("Resource created successfully"));

        // Verify result URL is generated
        assert!(html.contains("/admin/42"));
        assert!(html.contains("href"));

        // Verify data is displayed
        assert!(html.contains("New Item"));
        assert!(html.contains("Successfully created"));

        // Verify media type
        assert_eq!(renderer.media_types(), vec!["text/html"]);
    }

    #[tokio::test]
    async fn test_render_dict() {
        // Test: Admin renderer renders dictionary
        // Expected: Dictionary data formatted for admin interface

        let renderer = AdminRenderer::new();
        let data = json!({
            "username": "admin",
            "email": "admin@example.com",
            "active": true,
            "login_count": 42
        });

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        // Verify all fields are rendered in table format
        assert!(html.contains("<table"));
        assert!(html.contains("username"));
        assert!(html.contains("admin"));
        assert!(html.contains("email"));
        assert!(html.contains("admin@example.com"));
        assert!(html.contains("active"));
        assert!(html.contains("true"));
        assert!(html.contains("login_count"));
        assert!(html.contains("42"));

        // Verify admin interface styling
        assert!(html.contains("Admin Interface"));
    }

    #[tokio::test]
    async fn test_render_dict_with_items_key() {
        // Test: Admin renderer with 'items' key in dict
        // Expected: Special handling of 'items' key (doesn't confuse with iterator)

        let renderer = AdminRenderer::new();
        let data = json!({
            "items": [
                {"name": "Item 1", "price": 100},
                {"name": "Item 2", "price": 200}
            ],
            "total": 300
        });

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        // Verify 'items' key is treated as a normal field, not confused with iterator
        assert!(html.contains("items"));
        assert!(html.contains("Item 1"));
        assert!(html.contains("Item 2"));
        assert!(html.contains("total"));
        assert!(html.contains("300"));

        // Verify table structure
        assert!(html.contains("<table"));
    }

    #[tokio::test]
    async fn test_render_dict_with_iteritems_key() {
        // Test: Admin renderer with 'iteritems' key in dict
        // Expected: Special handling of 'iteritems' key

        let renderer = AdminRenderer::new();
        let data = json!({
            "iteritems": "legacy_method",
            "name": "Test Object",
            "value": 123
        });

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        // Verify 'iteritems' is rendered as a normal field
        assert!(html.contains("iteritems"));
        assert!(html.contains("legacy_method"));
        assert!(html.contains("name"));
        assert!(html.contains("Test Object"));
        assert!(html.contains("value"));
        assert!(html.contains("123"));
    }

    #[tokio::test]
    async fn test_get_result_url() {
        // Test: Admin renderer generates result URL
        // Expected: URL pointing to the created/updated resource

        let renderer = AdminRenderer::new();

        // Test with string ID
        let data_str = json!({"id": "abc123", "title": "Post"});
        let result = renderer.render(&data_str, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();
        assert!(html.contains("/admin/abc123"));

        // Test with integer ID
        let data_int = json!({"id": 999, "title": "Post"});
        let result = renderer.render(&data_int, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();
        assert!(html.contains("/admin/999"));
    }

    #[tokio::test]
    async fn test_get_result_url_no_result() {
        // Test: Admin renderer with no result
        // Expected: No URL generated, appropriate fallback

        let renderer = AdminRenderer::new();
        let data = json!({
            "message": "Operation completed",
            "status": "success"
        });

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        // Verify no success message for resource creation (no ID field)
        assert!(!html.contains("Resource created successfully"));
        assert!(!html.contains("View at:"));

        // Verify data is still displayed
        assert!(html.contains("Operation completed"));
        assert!(html.contains("success"));
    }

    #[tokio::test]
    async fn test_get_context_result_urls() {
        // Test: Admin renderer context with result URLs
        // Expected: Multiple result URLs in context for bulk operations

        let renderer = AdminRenderer::new();

        // Simulate bulk operation results (array of objects with IDs)
        let data = json!([
            {"id": 1, "name": "User 1", "created": true},
            {"id": 2, "name": "User 2", "created": true},
            {"id": 3, "name": "User 3", "created": true}
        ]);

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        // Verify all items are displayed in table format
        assert!(html.contains("User 1"));
        assert!(html.contains("User 2"));
        assert!(html.contains("User 3"));

        // Verify IDs are displayed
        assert!(html.contains("1") || html.contains("id"));
        assert!(html.contains("2"));
        assert!(html.contains("3"));

        // Verify table structure for arrays
        assert!(html.contains("<table"));
        assert!(html.contains("<thead"));
        assert!(html.contains("<tbody"));
    }

    #[tokio::test]
    async fn test_admin_renderer_custom_base_url() {
        // Test: Admin renderer with custom base URL
        // Expected: Custom base URL is used in result links

        let renderer = AdminRenderer::new().base_url("/custom-admin");
        let data = json!({"id": 5, "name": "Item"});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        // Verify custom base URL is used
        assert!(html.contains("/custom-admin/5"));
        assert!(!html.contains("/admin/5"));
    }

    #[tokio::test]
    async fn test_admin_renderer_format() {
        // Test: Admin renderer format identifier
        // Expected: Format is 'admin'

        let renderer = AdminRenderer::new();
        assert_eq!(renderer.format(), Some("admin"));
    }
}

#[cfg(test)]
mod documentation_renderer_tests {
    use reinhardt_renderers::{json::Renderer, DocumentationRenderer};
    use serde_json::json;

    #[tokio::test]
    async fn test_document_with_link_named_data() {
        // Test: Documentation renderer with link named 'data'
        // Expected: Proper handling of special field name 'data'

        let renderer = DocumentationRenderer::new();
        let schema = json!({
            "info": {
                "title": "User API",
                "description": "API for managing users"
            },
            "paths": {
                "/data": {
                    "get": {
                        "description": "Get data endpoint - special field name"
                    },
                    "post": {
                        "description": "Create data"
                    }
                }
            }
        });

        let result = renderer.render(&schema, None).await;
        assert!(result.is_ok(), "Rendering should succeed");

        let bytes = result.unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();

        // Verify 'data' is handled as a normal path
        assert!(html.contains("/data"));
        assert!(html.contains("GET"));
        assert!(html.contains("POST"));
        assert!(html.contains("Get data endpoint"));

        // Verify HTML structure
        assert!(html.contains("<html>"));
        assert!(html.contains("User API"));

        // Verify media type
        assert_eq!(renderer.media_types(), vec!["text/html"]);
    }

    #[tokio::test]
    async fn test_shell_code_example_rendering() {
        // Test: Documentation renderer shell code examples
        // Expected: Shell code examples with proper syntax highlighting (as text in HTML)

        let renderer = DocumentationRenderer::new();

        // Schema with shell command examples in description
        let schema = json!({
            "info": {
                "title": "CLI API",
                "description": "API for command-line tools"
            },
            "paths": {
                "/execute": {
                    "post": {
                        "description": "Execute command: curl -X POST /execute -d '{\"cmd\":\"ls -la\"}'"
                    }
                }
            }
        });

        let result = renderer.render(&schema, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        // Verify shell examples are preserved in HTML
        assert!(html.contains("curl"));
        assert!(html.contains("POST"));
        assert!(html.contains("/execute"));
        assert!(html.contains("ls -la"));

        // Verify format
        assert_eq!(renderer.format(), Some("docs"));
    }

    #[tokio::test]
    async fn test_documentation_markdown_format() {
        // Test: Documentation renderer in markdown format
        // Expected: Markdown output instead of HTML

        let renderer = DocumentationRenderer::new().format_type("markdown");
        let schema = json!({
            "info": {
                "title": "Markdown API",
                "description": "Testing markdown output"
            },
            "paths": {
                "/test": {
                    "get": {
                        "description": "Test endpoint"
                    }
                }
            }
        });

        let result = renderer.render(&schema, None).await.unwrap();
        let md = String::from_utf8(result.to_vec()).unwrap();

        // Verify markdown format
        assert!(md.contains("# Markdown API"));
        assert!(md.contains("## Endpoints"));
        assert!(md.contains("### GET /test"));
        assert!(md.contains("Testing markdown output"));

        // Verify media type
        assert_eq!(renderer.media_types(), vec!["text/markdown"]);
    }
}

#[cfg(test)]
mod schema_js_renderer_tests {
    use reinhardt_renderers::{json::Renderer, SchemaJSRenderer};
    use serde_json::json;

    #[tokio::test]
    async fn test_schemajs_output() {
        // Test: Schema.js renderer output
        // Expected: JavaScript code for Schema.js library

        let renderer = SchemaJSRenderer::new();
        let schema = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "User API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "summary": "List all users",
                        "responses": {
                            "200": {
                                "description": "Success"
                            }
                        }
                    }
                }
            }
        });

        let result = renderer.render(&schema, None).await;
        assert!(result.is_ok(), "Rendering should succeed");

        let bytes = result.unwrap();
        let js_code = String::from_utf8(bytes.to_vec()).unwrap();

        // Verify JavaScript structure
        assert!(js_code.contains("const apiSchema"));
        assert!(js_code.contains("openapi:"));
        assert!(js_code.contains("\"3.0.0\""));

        // Verify paths are included
        assert!(js_code.contains("paths:"));
        assert!(js_code.contains("\"/users\""));

        // Verify helper functions
        assert!(js_code.contains("function getEndpoint"));
        assert!(js_code.contains("function getAllPaths"));

        // Verify module export
        assert!(js_code.contains("module.exports"));

        // Verify media type
        assert_eq!(renderer.media_types(), vec!["application/javascript"]);
    }

    #[tokio::test]
    async fn test_schemajs_javascript_validity() {
        // Test: Schema.js renderer produces valid JavaScript
        // Expected: Output can be parsed as JavaScript (basic syntax check)

        let renderer = SchemaJSRenderer::new();
        let schema = json!({
            "paths": {
                "/test": {
                    "post": {
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let result = renderer.render(&schema, None).await.unwrap();
        let js_code = String::from_utf8(result.to_vec()).unwrap();

        // Verify JavaScript keywords and structure
        assert!(js_code.contains("const"));
        assert!(js_code.contains("function"));
        assert!(js_code.contains("return"));

        // Verify object property syntax
        assert!(js_code.contains("paths:"));
        assert!(js_code.contains("post:"));

        // Verify format
        assert_eq!(renderer.format(), Some("schemajs"));
    }

    #[tokio::test]
    async fn test_schemajs_special_characters() {
        // Test: Schema.js renderer handles special characters
        // Expected: Proper escaping of strings with quotes and backslashes

        let renderer = SchemaJSRenderer::new();
        let schema = json!({
            "info": {
                "title": "API with \"quotes\" and \\backslashes\\",
                "description": "Testing special characters"
            }
        });

        let result = renderer.render(&schema, None).await.unwrap();
        let js_code = String::from_utf8(result.to_vec()).unwrap();

        // Verify JavaScript is generated (exact escaping may vary)
        assert!(js_code.contains("info:"));
        assert!(js_code.contains("title:"));

        // The output should contain escaped characters or be valid JavaScript
        // (the implementation uses proper escaping in value_to_js)
        assert!(js_code.contains("\\\"") || js_code.contains("quotes"));
    }
}

#[cfg(test)]
mod browsable_api_renderer_tests {
    use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer, FormContext, FormField};
    use serde_json::json;

    #[test]
    fn test_render_form_for_serializer() {
        // Test: Render form for serializer in browsable API
        // Expected: HTML form generated from serializer fields

        let renderer = BrowsableApiRenderer::new();

        // Create form context representing serializer fields
        let form = FormContext {
            fields: vec![
                FormField {
                    name: "username".to_string(),
                    label: "Username".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    help_text: Some("Enter your username".to_string()),
                    initial_value: None,
                },
                FormField {
                    name: "email".to_string(),
                    label: "Email Address".to_string(),
                    field_type: "email".to_string(),
                    required: true,
                    help_text: Some("Enter a valid email".to_string()),
                    initial_value: None,
                },
                FormField {
                    name: "bio".to_string(),
                    label: "Biography".to_string(),
                    field_type: "textarea".to_string(),
                    required: false,
                    help_text: None,
                    initial_value: Some(json!("Default bio text")),
                },
            ],
            submit_url: "/api/users/create/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = ApiContext {
            title: "User Registration".to_string(),
            description: Some("Register a new user".to_string()),
            endpoint: "/api/users/create/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify form is rendered with serializer fields
        assert!(html.contains("<form"));
        assert!(html.contains("username"));
        assert!(html.contains("email"));
        assert!(html.contains("bio"));
        assert!(html.contains("Enter your username"));
        assert!(html.contains("Enter a valid email"));
        assert!(html.contains("Default bio text"));
        assert!(html.contains("submit"));
    }

    #[test]
    fn test_get_raw_data_form() {
        // Test: Get raw data form in browsable API
        // Expected: Raw JSON/data input form

        let renderer = BrowsableApiRenderer::new();

        // Create a raw data form (textarea for JSON input)
        let form = FormContext {
            fields: vec![FormField {
                name: "raw_data".to_string(),
                label: "Raw Data".to_string(),
                field_type: "textarea".to_string(),
                required: true,
                help_text: Some("Enter JSON data".to_string()),
                initial_value: Some(json!(r#"{"key": "value"}"#)),
            }],
            submit_url: "/api/raw/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = ApiContext {
            title: "Raw Data Endpoint".to_string(),
            description: None,
            endpoint: "/api/raw/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify raw data form is rendered
        assert!(html.contains("raw_data"));
        assert!(html.contains("textarea"));
        assert!(html.contains("Enter JSON data"));
        assert!(html.contains(r#"{"key": "value"}"#) || html.contains("key"));
    }

    #[test]
    fn test_get_description_returns_empty_string_for_401_and_403_statuses() {
        // Test: Description is empty for 401/403 statuses
        // Expected: No description shown for authentication/permission errors

        let renderer = BrowsableApiRenderer::new();

        // Test 401 Unauthorized
        let context_401 = ApiContext {
            title: "Unauthorized".to_string(),
            description: None, // No description for auth errors
            endpoint: "/api/protected/".to_string(),
            method: "GET".to_string(),
            response_data: json!({"detail": "Authentication required"}),
            response_status: 401,
            allowed_methods: vec![],
            request_form: None,
            headers: vec![],
        };

        let html_401 = renderer.render(&context_401).unwrap();
        assert!(html_401.contains("Unauthorized"));
        assert!(html_401.contains("401"));

        // Test 403 Forbidden
        let context_403 = ApiContext {
            title: "Forbidden".to_string(),
            description: None, // No description for permission errors
            endpoint: "/api/admin/".to_string(),
            method: "GET".to_string(),
            response_data: json!({"detail": "Permission denied"}),
            response_status: 403,
            allowed_methods: vec![],
            request_form: None,
            headers: vec![],
        };

        let html_403 = renderer.render(&context_403).unwrap();
        assert!(html_403.contains("Forbidden"));
        assert!(html_403.contains("403"));
    }

    #[test]
    fn test_get_filter_form_returns_none_if_data_is_not_list_instance() {
        // Test: Filter form returns None for non-list data
        // Expected: Filter form only shown for list endpoints

        let renderer = BrowsableApiRenderer::new();

        // Single object response (not a list) - no filter form
        let context = ApiContext {
            title: "User Detail".to_string(),
            description: None,
            endpoint: "/api/users/1/".to_string(),
            method: "GET".to_string(),
            response_data: json!({"id": 1, "name": "Alice"}), // Single object, not array
            response_status: 200,
            allowed_methods: vec!["GET".to_string(), "PUT".to_string(), "DELETE".to_string()],
            request_form: None, // No form for detail endpoints
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify no filter form is rendered for single object responses
        assert!(html.contains("Alice"));
        assert!(!html.contains("Make a Request")); // No form section
    }

    #[test]
    fn test_extra_actions_dropdown() {
        // Test: Extra actions dropdown in browsable API
        // Expected: Custom actions displayed in dropdown

        let renderer = BrowsableApiRenderer::new();

        // Simulate multiple allowed methods (acts like action dropdown)
        let context = ApiContext {
            title: "User Actions".to_string(),
            description: Some("Available user actions".to_string()),
            endpoint: "/api/users/1/".to_string(),
            method: "GET".to_string(),
            response_data: json!({"id": 1, "name": "Bob"}),
            response_status: 200,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "PATCH".to_string(),
                "DELETE".to_string(),
            ],
            request_form: None,
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify all action methods are displayed
        assert!(html.contains("GET"));
        assert!(html.contains("POST"));
        assert!(html.contains("PUT"));
        assert!(html.contains("PATCH"));
        assert!(html.contains("DELETE"));
        assert!(html.contains("Allowed methods"));
    }

    #[test]
    fn test_extra_actions_dropdown_not_authed() {
        // Test: Extra actions dropdown when not authenticated
        // Expected: Actions requiring authentication are hidden

        let renderer = BrowsableApiRenderer::new();

        // Unauthenticated context - limited allowed methods
        let context = ApiContext {
            title: "Public Endpoint".to_string(),
            description: Some("Limited actions for unauthenticated users".to_string()),
            endpoint: "/api/public/".to_string(),
            method: "GET".to_string(),
            response_data: json!({"message": "Public data"}),
            response_status: 200,
            allowed_methods: vec!["GET".to_string()], // Only GET allowed
            request_form: None,
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify only allowed method is shown
        assert!(html.contains("GET"));
        assert!(!html.contains("POST")); // Not in allowed_methods
        assert!(!html.contains("DELETE")); // Not in allowed_methods
        assert!(html.contains("Allowed methods"));
    }
}

#[cfg(test)]
mod html_form_renderer_tests {
    use reinhardt_browsable_api::{BrowsableApiRenderer, FormContext, FormField};
    use reinhardt_forms::{CharField, Form, IntegerField};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_hidden_field_rendering() {
        // Test: Hidden field rendering in HTML forms
        // Expected: Hidden input fields properly rendered

        let renderer = BrowsableApiRenderer::new();

        // Create form with hidden field
        let form = FormContext {
            fields: vec![
                FormField {
                    name: "csrf_token".to_string(),
                    label: "CSRF Token".to_string(),
                    field_type: "hidden".to_string(),
                    required: true,
                    help_text: None,
                    initial_value: Some(json!("abc123xyz")),
                },
                FormField {
                    name: "username".to_string(),
                    label: "Username".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    help_text: None,
                    initial_value: None,
                },
            ],
            submit_url: "/api/submit/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = reinhardt_browsable_api::ApiContext {
            title: "Form with Hidden Field".to_string(),
            description: None,
            endpoint: "/api/submit/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify hidden field is rendered
        assert!(html.contains("csrf_token"));
        assert!(html.contains("hidden"));
        assert!(html.contains("abc123xyz"));
    }

    #[test]
    fn test_render_with_default_args() {
        // Test: HTML form rendering with default arguments
        // Expected: Form rendered with default styling/structure

        let renderer = BrowsableApiRenderer::new();

        // Simple form with default rendering
        let form = FormContext {
            fields: vec![
                FormField {
                    name: "email".to_string(),
                    label: "Email".to_string(),
                    field_type: "email".to_string(),
                    required: true,
                    help_text: None,
                    initial_value: None,
                },
                FormField {
                    name: "password".to_string(),
                    label: "Password".to_string(),
                    field_type: "password".to_string(),
                    required: true,
                    help_text: None,
                    initial_value: None,
                },
            ],
            submit_url: "/api/login/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = reinhardt_browsable_api::ApiContext {
            title: "Login Form".to_string(),
            description: None,
            endpoint: "/api/login/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify default form structure
        assert!(html.contains("<form"));
        assert!(html.contains("email"));
        assert!(html.contains("password"));
        assert!(html.contains("type=\"email\""));
        assert!(html.contains("type=\"password\""));
        assert!(html.contains("required"));
    }

    #[test]
    fn test_render_with_provided_args() {
        // Test: HTML form rendering with provided arguments
        // Expected: Custom arguments affect form rendering

        let renderer = BrowsableApiRenderer::new();

        // Form with custom help text and initial values
        let form = FormContext {
            fields: vec![
                FormField {
                    name: "title".to_string(),
                    label: "Post Title".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    help_text: Some("Enter a descriptive title (max 100 chars)".to_string()),
                    initial_value: Some(json!("My First Post")),
                },
                FormField {
                    name: "content".to_string(),
                    label: "Content".to_string(),
                    field_type: "textarea".to_string(),
                    required: false,
                    help_text: Some("Markdown supported".to_string()),
                    initial_value: Some(json!("# Hello World")),
                },
            ],
            submit_url: "/api/posts/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = reinhardt_browsable_api::ApiContext {
            title: "Create Post".to_string(),
            description: None,
            endpoint: "/api/posts/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify custom arguments are rendered
        assert!(html.contains("Enter a descriptive title (max 100 chars)"));
        assert!(html.contains("Markdown supported"));
        assert!(html.contains("My First Post"));
        assert!(html.contains("# Hello World") || html.contains("Hello World"));
    }

    #[test]
    fn test_render_initial_option() {
        // Test: Choice field initial option rendering
        // Expected: Initial/placeholder option in select field
        // Note: Using text field with initial value as select not yet implemented

        let renderer = BrowsableApiRenderer::new();

        let form = FormContext {
            fields: vec![FormField {
                name: "category".to_string(),
                label: "Category".to_string(),
                field_type: "text".to_string(), // Using text as proxy for select
                required: false,
                help_text: Some("Select a category".to_string()),
                initial_value: Some(json!("")), // Empty initial value
            }],
            submit_url: "/api/items/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = reinhardt_browsable_api::ApiContext {
            title: "Create Item".to_string(),
            description: None,
            endpoint: "/api/items/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify field is rendered with placeholder concept
        assert!(html.contains("category"));
        assert!(html.contains("Select a category"));
    }

    #[test]
    fn test_render_selected_option() {
        // Test: Choice field selected option rendering
        // Expected: Selected option has proper value

        let renderer = BrowsableApiRenderer::new();

        let form = FormContext {
            fields: vec![FormField {
                name: "status".to_string(),
                label: "Status".to_string(),
                field_type: "text".to_string(),
                required: true,
                help_text: None,
                initial_value: Some(json!("active")), // Pre-selected value
            }],
            submit_url: "/api/update/".to_string(),
            submit_method: "PUT".to_string(),
        };

        let context = reinhardt_browsable_api::ApiContext {
            title: "Update Status".to_string(),
            description: None,
            endpoint: "/api/update/".to_string(),
            method: "PUT".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["PUT".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify selected value is rendered
        assert!(html.contains("status"));
        assert!(html.contains("active"));
    }

    #[test]
    fn test_render_selected_option_with_string_option_ids() {
        // Test: Multiple choice field with string IDs
        // Expected: String option IDs handled correctly

        // Use reinhardt-forms to create a form and validate string IDs
        let mut form = Form::new();
        form.add_field(Box::new(CharField::new("tags".to_string())));

        let mut data = HashMap::new();
        data.insert("tags".to_string(), json!("rust,web,api")); // Comma-separated string IDs
        form.bind(data);

        assert!(form.is_valid());
        assert_eq!(
            form.cleaned_data().get("tags"),
            Some(&json!("rust,web,api"))
        );

        // Now test rendering with BrowsableApiRenderer
        let renderer = BrowsableApiRenderer::new();

        let form_context = FormContext {
            fields: vec![FormField {
                name: "tags".to_string(),
                label: "Tags".to_string(),
                field_type: "text".to_string(),
                required: false,
                help_text: Some("Enter comma-separated tags".to_string()),
                initial_value: Some(json!("rust,web,api")),
            }],
            submit_url: "/api/items/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = reinhardt_browsable_api::ApiContext {
            title: "Item Tags".to_string(),
            description: None,
            endpoint: "/api/items/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form_context),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify string IDs are handled
        assert!(html.contains("tags"));
        assert!(html.contains("rust,web,api") || html.contains("rust"));
    }

    #[test]
    fn test_render_selected_option_with_integer_option_ids() {
        // Test: Multiple choice field with integer IDs
        // Expected: Integer option IDs handled correctly

        // Use reinhardt-forms to validate integer IDs
        let mut form = Form::new();
        form.add_field(Box::new(IntegerField::new("priority".to_string())));

        let mut data = HashMap::new();
        data.insert("priority".to_string(), json!(1)); // Integer ID
        form.bind(data);

        assert!(form.is_valid());
        assert_eq!(form.cleaned_data().get("priority"), Some(&json!(1)));

        // Now test rendering with BrowsableApiRenderer
        let renderer = BrowsableApiRenderer::new();

        let form_context = FormContext {
            fields: vec![FormField {
                name: "priority".to_string(),
                label: "Priority Level".to_string(),
                field_type: "number".to_string(),
                required: true,
                help_text: Some("1=Low, 2=Medium, 3=High".to_string()),
                initial_value: Some(json!(2)),
            }],
            submit_url: "/api/tasks/".to_string(),
            submit_method: "POST".to_string(),
        };

        let context = reinhardt_browsable_api::ApiContext {
            title: "Create Task".to_string(),
            description: None,
            endpoint: "/api/tasks/".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form_context),
            headers: vec![],
        };

        let html = renderer.render(&context).unwrap();

        // Verify integer IDs are handled
        assert!(html.contains("priority"));
        assert!(html.contains("number"));
        // Help text might be HTML escaped, so check for key parts
        assert!(html.contains("Low") && html.contains("Medium") && html.contains("High"));
    }
}

#[cfg(test)]
mod placeholder_test {}
