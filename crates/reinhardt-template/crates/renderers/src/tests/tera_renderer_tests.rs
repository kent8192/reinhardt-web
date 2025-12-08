//! Unit tests for Tera renderer and strategy selection
//!
//! These tests verify the integration of Tera template engine within the
//! reinhardt-renderers crate (single-crate unit tests).
//!
//! **Note:** "Integration" here refers to Tera template engine integration,
//! NOT multi-crate integration tests. For multi-crate integration tests,
//! see `tests/integration/tests/rendering/` directory (as per TESTING_STANDARDS.md TO-1).
//!
//! ## Test Coverage
//!
//! - Tera template rendering (runtime with compile-time embedding)
//! - Template strategy selection (CompileTime vs Runtime)
//! - TemplateHTMLRenderer comparison
//! - Conditional rendering (if/else in templates)
//! - List rendering (for loops in templates)

use crate::strategy::{TemplateSource, TemplateStrategy, TemplateStrategySelector};
use crate::template_html_renderer::TemplateHTMLRenderer;
use crate::tera_renderer::{TeraRenderer, UserData, UserListTemplate, UserTemplate};

#[test]
fn test_tera_user_template_basic() {
	let template = UserTemplate::new(
		"Integration Test".to_string(),
		"integration@test.com".to_string(),
		30,
	);

	let html = template.render_user().expect("Failed to render");

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
fn test_tera_user_list_template() {
	let users = vec![
		UserData::new("User A", "a@test.com"),
		UserData::new("User B", "b@test.com"),
		UserData::new("User C", "c@test.com"),
	];

	let template = UserListTemplate::new(users, "Integration Test Users".to_string());
	let html = template.render_list().expect("Failed to render");

	assert!(
		html.contains("Integration Test Users"),
		"HTML must contain title 'Integration Test Users'. Actual HTML: {}",
		html
	);

	let expected_users = [
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
	let strategy = TemplateStrategySelector::recommend_for_use_case("view template");
	assert_eq!(strategy, TemplateStrategy::CompileTime);

	let strategy = TemplateStrategySelector::recommend_for_use_case("user provided template");
	assert_eq!(strategy, TemplateStrategy::Runtime);
}

#[tokio::test]
async fn test_runtime_vs_compile_time_correctness() {
	// Test Tera compile-time embedded template
	let tera_template =
		UserTemplate::new("Test User".to_string(), "test@example.com".to_string(), 25);
	let tera_html = tera_template.render_user().expect("Failed to render");

	assert!(
		tera_html.contains("Test User"),
		"Tera HTML must contain 'Test User'. Actual HTML: {}",
		tera_html
	);
	assert!(
		tera_html.contains("test@example.com"),
		"Tera HTML must contain 'test@example.com'. Actual HTML: {}",
		tera_html
	);
	assert!(
		tera_html.contains("25"),
		"Tera HTML must contain age '25'. Actual HTML: {}",
		tera_html
	);

	// Test TemplateHTMLRenderer (now using Tera internally)
	let renderer = TemplateHTMLRenderer::new();
	let context = serde_json::json!({
		"template_string": "<h1>{{ name }}</h1><p>Email: {{ email }}</p><p>Age: {{ age }}</p>",
		"name": "Test User",
		"email": "test@example.com",
		"age": 25
	});

	let result = renderer.render_template(&context).await;
	assert!(result.is_ok(), "Failed to render template: {:?}", result.err());

	let runtime_html = result.unwrap();
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
fn test_tera_renderer_direct() {
	let renderer = TeraRenderer::new();
	let template = UserTemplate::new("Direct Test".to_string(), "direct@test.com".to_string(), 35);

	let html = renderer
		.render_template("user.tpl", &template)
		.expect("Failed to render");

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
fn test_tera_renderer_with_context() {
	let renderer = TeraRenderer::new();
	let template = UserTemplate::new(
		"Context Test".to_string(),
		"context@test.com".to_string(),
		40,
	);

	let html = renderer
		.render_with_context("user.tpl", &template, "user profile page")
		.expect("Failed to render with context");

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
	let static_source = TemplateSource::Static("template.html");
	assert!(static_source.is_static());
	assert!(!static_source.is_dynamic());
	assert!(!static_source.is_file());
	assert_eq!(static_source.as_str(), "template.html");

	let dynamic_source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
	assert!(!dynamic_source.is_static());
	assert!(dynamic_source.is_dynamic());
	assert!(!dynamic_source.is_file());
	assert_eq!(dynamic_source.as_str(), "<h1>{{ title }}</h1>");

	let file_source = TemplateSource::File("/path/to/template.html".to_string());
	assert!(!file_source.is_static());
	assert!(!file_source.is_dynamic());
	assert!(file_source.is_file());
	assert_eq!(file_source.as_str(), "/path/to/template.html");
}

#[test]
fn test_file_extension_based_strategy() {
	let tera_file = TemplateSource::File("template.tera".to_string());
	assert_eq!(
		TemplateStrategySelector::select(&tera_file),
		TemplateStrategy::CompileTime
	);

	let jinja = TemplateSource::File("template.jinja".to_string());
	assert_eq!(
		TemplateStrategySelector::select(&jinja),
		TemplateStrategy::CompileTime
	);

	let tpl = TemplateSource::File("template.tpl".to_string());
	assert_eq!(
		TemplateStrategySelector::select(&tpl),
		TemplateStrategy::CompileTime
	);

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

	assert!(
		html.contains("Large List Test"),
		"HTML must contain title 'Large List Test'. Actual HTML: {}",
		html
	);

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
fn test_tera_conditional_rendering_adult() {
	let adult_template =
		UserTemplate::new("Adult User".to_string(), "adult@test.com".to_string(), 18);
	let html = adult_template.render_user().expect("Failed to render");

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
fn test_tera_conditional_rendering_minor() {
	let minor_template =
		UserTemplate::new("Minor User".to_string(), "minor@test.com".to_string(), 17);
	let html = minor_template.render_user().expect("Failed to render");

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
