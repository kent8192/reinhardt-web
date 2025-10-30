//! Unit tests for Askama renderer and strategy selection
//!
//! These tests verify the integration of Askama template engine within the
//! reinhardt-renderers crate (single-crate unit tests).
//!
//! **Note:** "Integration" here refers to Askama template engine integration,
//! NOT multi-crate integration tests. For multi-crate integration tests,
//! see `tests/integration/tests/rendering/` directory (as per TESTING_STANDARDS.md TO-1).
//!
//! ## Test Coverage
//!
//! - Askama template rendering (compile-time)
//! - Template strategy selection (CompileTime vs Runtime)
//! - TemplateHTMLRenderer comparison
//! - Conditional rendering (if/else in templates)
//! - List rendering (for loops in templates)

use crate::askama_renderer::{AskamaRenderer, UserData, UserListTemplate, UserTemplate};
use crate::strategy::{TemplateSource, TemplateStrategy, TemplateStrategySelector};
use crate::template_html_renderer::TemplateHTMLRenderer;
use std::collections::HashMap;

#[test]
fn test_askama_user_template_basic() {
    let template = UserTemplate::new(
        "Integration Test".to_string(),
        "integration@test.com".to_string(),
        30,
    );

    let html = template.render_user().expect("Failed to render");

    // Verify exact values of template variables
    assert!(
        html.contains("Integration Test"),
        "HTML must contain 'Integration Test'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("integration@test.com"),
        "HTML must contain 'integration@test.com'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("30"),
        "HTML must contain age '30'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("Adult"),
        "HTML must contain 'Adult' status. Actual HTML: {}",
        html
    );
}

#[test]
fn test_askama_user_list_template() {
    let users = vec![
        UserData::new("User A", "a@test.com"),
        UserData::new("User B", "b@test.com"),
        UserData::new("User C", "c@test.com"),
    ];

    let template = UserListTemplate::new(users, "Integration Test Users".to_string());
    let html = template.render_list().expect("Failed to render");

    // Verify exact values of template variables
    assert!(
        html.contains("Integration Test Users"),
        "HTML must contain title 'Integration Test Users'. Actual HTML: {}",
        html
    );

    // Verify each user exists
    let expected_users = vec![
        ("User A", "a@test.com"),
        ("User B", "b@test.com"),
        ("User C", "c@test.com"),
    ];

    for (name, email) in expected_users.iter() {
        assert!(
            html.contains(name),
            "HTML must contain username '{}'. Actual HTML: {}",
            name,
            html
        );
        assert!(
            html.contains(email),
            "HTML must contain email '{}'. Actual HTML: {}",
            email,
            html
        );
    }
}

#[test]
fn test_strategy_selection_compile_time() {
    let source = TemplateSource::Static("user.html");
    let strategy = TemplateStrategySelector::select(&source);
    assert_eq!(strategy, TemplateStrategy::CompileTime);
}

#[test]
fn test_strategy_selection_runtime() {
    let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
    let strategy = TemplateStrategySelector::select(&source);
    assert_eq!(strategy, TemplateStrategy::Runtime);
}

#[test]
fn test_strategy_recommendation() {
    // View templates should use compile-time
    let strategy = TemplateStrategySelector::recommend_for_use_case("view template");
    assert_eq!(strategy, TemplateStrategy::CompileTime);

    // User templates should use runtime
    let strategy = TemplateStrategySelector::recommend_for_use_case("user provided template");
    assert_eq!(strategy, TemplateStrategy::Runtime);
}

#[test]
fn test_runtime_vs_compile_time_correctness() {
    // Test that both renderers produce similar output for basic substitution

    // Compile-time (Askama)
    let askama_template = UserTemplate::new(
        "Test User".to_string(),
        "test@example.com".to_string(),
        25,
    );
    let askama_html = askama_template.render_user().expect("Failed to render");

    // Verify Askama output contains expected data with detailed error messages
    assert!(
        askama_html.contains("Test User"),
        "Askama HTML must contain 'Test User'. Actual HTML: {}",
        askama_html
    );
    assert!(
        askama_html.contains("test@example.com"),
        "Askama HTML must contain 'test@example.com'. Actual HTML: {}",
        askama_html
    );
    assert!(
        askama_html.contains("25"),
        "Askama HTML must contain age '25'. Actual HTML: {}",
        askama_html
    );

    // Runtime (TemplateHTMLRenderer)
    let mut runtime_context = HashMap::new();
    runtime_context.insert("name".to_string(), "Test User".to_string());
    runtime_context.insert("email".to_string(), "test@example.com".to_string());
    runtime_context.insert("age".to_string(), "25".to_string());

    let template_str = "<h1>{{ name }}</h1><p>Email: {{ email }}</p><p>Age: {{ age }}</p>";
    let runtime_html =
        TemplateHTMLRenderer::substitute_variables_single_pass(template_str, &runtime_context);

    // Verify runtime output contains expected data with detailed error messages
    assert!(
        runtime_html.contains("Test User"),
        "Runtime HTML must contain 'Test User'. Actual HTML: {}",
        runtime_html
    );
    assert!(
        runtime_html.contains("test@example.com"),
        "Runtime HTML must contain 'test@example.com'. Actual HTML: {}",
        runtime_html
    );
    assert!(
        runtime_html.contains("25"),
        "Runtime HTML must contain age '25'. Actual HTML: {}",
        runtime_html
    );
}

#[test]
fn test_askama_renderer_direct() {
    let renderer = AskamaRenderer::new();
    let template = UserTemplate::new(
        "Direct Test".to_string(),
        "direct@test.com".to_string(),
        35,
    );

    let html = renderer.render(&template).expect("Failed to render");

    // Verify exact values of template variables
    assert!(
        html.contains("Direct Test"),
        "HTML must contain 'Direct Test'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("direct@test.com"),
        "HTML must contain 'direct@test.com'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("35"),
        "HTML must contain age '35'. Actual HTML: {}",
        html
    );
}

#[test]
fn test_askama_renderer_with_context() {
    let renderer = AskamaRenderer::new();
    let template = UserTemplate::new(
        "Context Test".to_string(),
        "context@test.com".to_string(),
        40,
    );

    let html = renderer
        .render_with_context(&template, "user profile page")
        .expect("Failed to render with context");

    // Verify exact values of template variables
    assert!(
        html.contains("Context Test"),
        "HTML must contain 'Context Test'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("context@test.com"),
        "HTML must contain 'context@test.com'. Actual HTML: {}",
        html
    );
}

#[test]
fn test_template_source_types() {
    // Static
    let static_source = TemplateSource::Static("template.html");
    assert!(static_source.is_static());
    assert!(!static_source.is_dynamic());
    assert!(!static_source.is_file());
    assert_eq!(static_source.as_str(), "template.html");

    // Dynamic
    let dynamic_source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
    assert!(!dynamic_source.is_static());
    assert!(dynamic_source.is_dynamic());
    assert!(!dynamic_source.is_file());
    assert_eq!(dynamic_source.as_str(), "<h1>{{ title }}</h1>");

    // File
    let file_source = TemplateSource::File("/path/to/template.html".to_string());
    assert!(!file_source.is_static());
    assert!(!file_source.is_dynamic());
    assert!(file_source.is_file());
    assert_eq!(file_source.as_str(), "/path/to/template.html");
}

#[test]
fn test_file_extension_based_strategy() {
    // Askama-specific extensions -> Compile-time
    let askama_html = TemplateSource::File("template.askama.html".to_string());
    assert_eq!(
        TemplateStrategySelector::select(&askama_html),
        TemplateStrategy::CompileTime
    );

    let jinja = TemplateSource::File("template.jinja".to_string());
    assert_eq!(
        TemplateStrategySelector::select(&jinja),
        TemplateStrategy::CompileTime
    );

    // Regular extensions -> Runtime
    let html = TemplateSource::File("template.html".to_string());
    assert_eq!(
        TemplateStrategySelector::select(&html),
        TemplateStrategy::Runtime
    );

    let txt = TemplateSource::File("template.txt".to_string());
    assert_eq!(
        TemplateStrategySelector::select(&txt),
        TemplateStrategy::Runtime
    );
}

#[test]
fn test_empty_user_list() {
    let users = vec![];
    let template = UserListTemplate::new(users, "Empty List Test".to_string());
    let html = template.render_list().expect("Failed to render empty list");

    // Verify rendering results for empty list
    assert!(
        html.contains("Empty List Test"),
        "HTML must contain title 'Empty List Test'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("No users found"),
        "HTML must contain 'No users found' message. Actual HTML: {}",
        html
    );
}

#[test]
fn test_single_user_list() {
    let users = vec![UserData::new("Single User", "single@test.com")];
    let template = UserListTemplate::new(users, "Single User Test".to_string());
    let html = template.render_list().expect("Failed to render");

    // Verify rendering results for single user
    assert!(
        html.contains("Single User"),
        "HTML must contain username 'Single User'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("single@test.com"),
        "HTML must contain email 'single@test.com'. Actual HTML: {}",
        html
    );
}

#[test]
fn test_large_user_list() {
    let users: Vec<UserData> = (0..100)
        .map(|i| UserData::new(format!("User {}", i), format!("user{}@test.com", i)))
        .collect();

    let template = UserListTemplate::new(users, "Large List Test".to_string());
    let html = template.render_list().expect("Failed to render large list");

    // Verify rendering results for large list
    assert!(
        html.contains("Large List Test"),
        "HTML must contain title 'Large List Test'. Actual HTML: {}",
        html
    );

    // Verify first and last users
    assert!(
        html.contains("User 0"),
        "HTML must contain first user 'User 0'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("User 99"),
        "HTML must contain last user 'User 99'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("user0@test.com"),
        "HTML must contain first user's email 'user0@test.com'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("user99@test.com"),
        "HTML must contain last user's email 'user99@test.com'. Actual HTML: {}",
        html
    );
}

#[test]
fn test_user_data_display_trait() {
    let user = UserData::new("Display User", "display@test.com");
    let display_str = format!("{}", user);
    assert_eq!(display_str, "Display User (display@test.com)");
}

#[test]
fn test_askama_conditional_rendering_adult() {
    let adult_template = UserTemplate::new(
        "Adult User".to_string(),
        "adult@test.com".to_string(),
        18,
    );
    let html = adult_template.render_user().expect("Failed to render");

    // Verify conditional rendering for adult user
    assert!(
        html.contains("Adult User"),
        "HTML must contain username 'Adult User'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("18"),
        "HTML must contain age '18'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("Adult"),
        "HTML must contain 'Adult' status. Actual HTML: {}",
        html
    );
    assert!(
        !html.contains("Minor"),
        "HTML must not contain 'Minor' status. Actual HTML: {}",
        html
    );
}

#[test]
fn test_askama_conditional_rendering_minor() {
    let minor_template = UserTemplate::new(
        "Minor User".to_string(),
        "minor@test.com".to_string(),
        17,
    );
    let html = minor_template.render_user().expect("Failed to render");

    // Verify conditional rendering for minor user
    assert!(
        html.contains("Minor User"),
        "HTML must contain username 'Minor User'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("17"),
        "HTML must contain age '17'. Actual HTML: {}",
        html
    );
    assert!(
        html.contains("Minor"),
        "HTML must contain 'Minor' status. Actual HTML: {}",
        html
    );
    assert!(
        !html.contains("Adult User"),
        "HTML must not contain 'Adult User' (different username). Actual HTML: {}",
        html
    );
}
