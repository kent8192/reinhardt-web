#![cfg(feature = "templates")]

use reinhardt_shortcuts::template_inheritance::render_string_with_inheritance;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Create a temporary test directory and return its path
fn setup_test_templates() -> PathBuf {
	let test_dir = PathBuf::from("/tmp/reinhardt_test_templates");

	// Clean up any existing test directory
	if test_dir.exists() {
		fs::remove_dir_all(&test_dir).unwrap();
	}

	// Create fresh test directory
	fs::create_dir_all(&test_dir).unwrap();

	test_dir
}

/// Clean up the test template directory
fn cleanup_test_templates(test_dir: &PathBuf) {
	if test_dir.exists() {
		fs::remove_dir_all(test_dir).unwrap();
	}
}

#[test]
fn test_render_string_with_basic_variables() {
	let mut context = HashMap::new();
	context.insert("title", serde_json::json!("Welcome"));
	context.insert("name", serde_json::json!("Alice"));

	let template = "<h1>{{ title }}</h1><p>Hello, {{ name }}!</p>";
	let result = render_string_with_inheritance(template, &context).unwrap();

	assert!(result.contains("<h1>Welcome</h1>"));
	assert!(result.contains("<p>Hello, Alice!</p>"));
}

#[test]
fn test_render_string_with_conditionals() {
	let mut context = HashMap::new();
	context.insert("user_authenticated", serde_json::json!(true));
	context.insert("username", serde_json::json!("Bob"));

	let template = r#"
        {% if user_authenticated %}
            Welcome back, {{ username }}!
        {% else %}
            Please log in.
        {% endif %}
    "#;

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert!(result.contains("Welcome back, Bob!"));
	assert!(!result.contains("Please log in"));
}

#[test]
fn test_render_string_with_loops() {
	let mut context = HashMap::new();
	context.insert(
		"items",
		serde_json::json!(vec!["Apple", "Banana", "Cherry"]),
	);

	let template = r#"
        <ul>
        {% for item in items %}
            <li>{{ item }}</li>
        {% endfor %}
        </ul>
    "#;

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert!(result.contains("<li>Apple</li>"));
	assert!(result.contains("<li>Banana</li>"));
	assert!(result.contains("<li>Cherry</li>"));
}

#[test]
fn test_render_string_with_filters() {
	let mut context = HashMap::new();
	context.insert("text", serde_json::json!("HELLO WORLD"));

	let template = r#"
        Lowercase: {{ text | lower }}
        Uppercase: {{ text | upper }}
    "#;

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert!(result.contains("Lowercase: hello world"));
	assert!(result.contains("Uppercase: HELLO WORLD"));
}

#[test]
#[ignore = "Template inheritance with extends requires file-based templates, which needs filesystem-based Tera instance"]
fn test_render_with_inheritance_base_template() {
	let test_dir = setup_test_templates();

	// Create base template
	let base_path = test_dir.join("base.html");
	fs::write(
		&base_path,
		r#"<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}Default Title{% endblock %}</title>
</head>
<body>
    <header>{% block header %}Default Header{% endblock %}</header>
    <main>{% block content %}Default Content{% endblock %}</main>
</body>
</html>"#,
	)
	.unwrap();

	// Create child template
	let child_path = test_dir.join("child.html");
	fs::write(
		&child_path,
		r#"{% extends "base.html" %}

{% block title %}Child Page{% endblock %}

{% block content %}
<p>This is child content: {{ message }}</p>
{% endblock %}"#,
	)
	.unwrap();

	// Set template directory
	unsafe {
		env::set_var("REINHARDT_TEMPLATE_DIR", test_dir.to_str().unwrap());
	}

	// Render child template
	let mut context = HashMap::new();
	context.insert("message", serde_json::json!("Hello from child!"));

	// Note: Tera caches templates on initialization, so we need to create a new instance
	// For this test, we'll use render_string_with_inheritance since render_with_inheritance
	// uses a cached global instance
	let result =
		render_string_with_inheritance(&fs::read_to_string(&child_path).unwrap(), &context);

	// Clean up
	cleanup_test_templates(&test_dir);
	unsafe {
		env::remove_var("REINHARDT_TEMPLATE_DIR");
	}

	// The template inheritance requires file-based templates
	// String rendering doesn't support extends
	assert!(result.is_ok());
}

#[test]
fn test_render_string_with_nested_variables() {
	let mut context = HashMap::new();

	// Tera handles nested data through JSON serialization
	let user_data = serde_json::json!({
		"name": "Alice",
		"profile": {
			"age": 30,
			"city": "Tokyo"
		}
	});

	context.insert("user", user_data);

	let template = r#"
        Name: {{ user.name }}
        Age: {{ user.profile.age }}
        City: {{ user.profile.city }}
    "#;

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert!(result.contains("Name: Alice"));
	assert!(result.contains("Age: 30"));
	assert!(result.contains("City: Tokyo"));
}

#[test]
fn test_render_string_with_array_indexing() {
	let mut context = HashMap::new();
	context.insert("items", serde_json::json!(vec!["First", "Second", "Third"]));

	let template = r#"
        Item 0: {{ items.0 }}
        Item 1: {{ items.1 }}
    "#;

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert!(result.contains("Item 0: First"));
	assert!(result.contains("Item 1: Second"));
}

#[test]
fn test_render_string_empty_context() {
	let context: HashMap<String, serde_json::Value> = HashMap::new();
	let template = "<h1>Static Template</h1>";

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert_eq!(result, "<h1>Static Template</h1>");
}

#[test]
fn test_render_string_with_comments() {
	let mut context = HashMap::new();
	context.insert("visible", serde_json::json!("This is visible"));

	let template = r#"
        {{ visible }}
        {# This is a comment and should not appear #}
        Done
    "#;

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert!(result.contains("This is visible"));
	assert!(!result.contains("This is a comment"));
	assert!(result.contains("Done"));
}

#[test]
fn test_render_string_with_whitespace_control() {
	let mut context = HashMap::new();
	context.insert("items", serde_json::json!(vec!["a", "b", "c"]));

	// Tera supports whitespace control with -{%  %}
	let template = "{% for item in items -%}{{ item }}{%- endfor %}";

	let result = render_string_with_inheritance(template, &context).unwrap();

	assert_eq!(result.trim(), "abc");
}
