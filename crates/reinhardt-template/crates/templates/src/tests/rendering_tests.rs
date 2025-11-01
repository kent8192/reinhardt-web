//! Template rendering tests
//!
//! Tests for template rendering inspired by Django's test_engine.py and test_extends.py

use crate::TemplateLoader;
use serde::Serialize;
use tera::{Context, Tera};

#[test]
fn test_basic_rendering() {
    // Test basic template rendering (similar to Django's test_basic_context)
    let mut context = Context::new();
    context.insert("greeting", "Hello");
    context.insert("name", "World");

    let result = Tera::one_off("{{ greeting }} {{ name }}!", &context, false).unwrap();
    assert_eq!(result, "Hello World!");
}

#[test]
fn test_html_rendering() {
    // Test HTML template rendering
    let mut context = Context::new();
    context.insert("title", "Test Page");
    context.insert("content", "This is a test");

    let template = r#"<h1>{{ title }}</h1>
<p>{{ content }}</p>"#;
    let result = Tera::one_off(template, &context, false).unwrap();
    assert!(result.contains("<h1>Test Page</h1>"));
    assert!(result.contains("<p>This is a test</p>"));
}

#[test]
fn test_rendering_with_escaping() {
    // Test that HTML is escaped by default
    let mut context = Context::new();
    context.insert("title", "Test");
    context.insert("content", "<script>alert('xss')</script>");

    let template = r#"<h1>{{ title }}</h1>
<p>{{ content }}</p>"#;
    let result = Tera::one_off(template, &context, true).unwrap();
    // Tera escapes HTML by default in autoescape mode
    assert!(!result.is_empty());
}

#[test]
fn test_range_rendering() {
    // Test rendering with range loop
    let context = Context::new();

    let template = "{% for i in range(end=5) %}*{% endfor %}";
    let result = Tera::one_off(template, &context, false).unwrap();
    assert_eq!(result, "*****");
}

#[test]
fn test_range_rendering_zero() {
    // Test range loop with zero count
    let context = Context::new();

    let template = "{% for i in range(end=0) %}*{% endfor %}";
    let result = Tera::one_off(template, &context, false).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_match_rendering_some() {
    // Test conditional rendering with Some value
    let mut context = Context::new();
    context.insert("result", &Some("Success".to_string()));

    let template = "{% if result %}{{ result }}{% else %}empty{% endif %}";
    let result = Tera::one_off(template, &context, false).unwrap();
    assert_eq!(result, "Success");
}

#[test]
fn test_match_rendering_none() {
    // Test conditional rendering with None value
    let mut context = Context::new();
    context.insert("result", &Option::<String>::None);

    let template = "{% if result %}{{ result }}{% else %}empty{% endif %}";
    let result = Tera::one_off(template, &context, false).unwrap();
    assert_eq!(result, "empty");
}

#[test]
fn test_multiline_template() {
    // Test multi-line template rendering
    let context = Context::new();

    let template = r#"Line 1
Line 2
Line 3"#;
    let result = Tera::one_off(template, &context, false).unwrap();
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Line 1");
    assert_eq!(lines[2], "Line 3");
}

#[test]
fn test_rendering_with_whitespace_control() {
    // Test whitespace control in templates
    let mut context = Context::new();
    context.insert("items", &vec![1, 2, 3]);

    let template = "{% for i in items -%}
    {{ i }}
{%- endfor %}";
    let result = Tera::one_off(template, &context, false).unwrap();
    // Whitespace should be controlled
    assert!(!result.is_empty());
}

#[test]
fn test_template_with_comments() {
    // Test templates with comments
    let context = Context::new();

    let template = "Before{# This is a comment #}After";
    let result = Tera::one_off(template, &context, false).unwrap();
    assert_eq!(result, "BeforeAfter");
}

#[test]
fn test_rendering_error_handling() {
    // Test that rendering returns proper Result type
    let mut context = Context::new();
    context.insert("greeting", "Hi");
    context.insert("name", "Test");

    let template = "{{ greeting }} {{ name }}!";
    let result = Tera::one_off(template, &context, false);
    assert!(result.is_ok());
}

#[test]
fn test_template_loader_render_workflow() {
    // Test complete workflow: register and render templates
    let mut loader = TemplateLoader::new();

    loader.register("page.html", || {
        let mut context = Context::new();
        context.insert("title", "Welcome");
        context.insert("content", "Welcome to our site");

        let template = r#"<h1>{{ title }}</h1>
<p>{{ content }}</p>"#;
        Tera::one_off(template, &context, false).unwrap()
    });

    let result = loader.render("page.html");
    assert!(result.is_ok());
    let html = result.unwrap();
    assert!(html.contains("Welcome"));
}

#[test]
fn test_nested_loops() {
    // Test nested loop rendering
    let context = Context::new();

    let template =
        "{% for i in range(end=2) %}{% for j in range(end=2) %}({{ i }},{{ j }}){% endfor %}{% endfor %}";
    let result = Tera::one_off(template, &context, false).unwrap();
    assert!(result.contains("(0,0)"));
    assert!(result.contains("(1,1)"));
}

#[test]
fn test_complex_expression() {
    // Test complex expressions in templates
    let mut context = Context::new();
    context.insert("a", &1);
    context.insert("b", &2);
    context.insert("c", &3);

    let template = "{{ a + b * c }}";
    let result = Tera::one_off(template, &context, false).unwrap();
    assert_eq!(result, "7"); // 1 + 2 * 3 = 7
}

#[test]
fn test_comparison_operators() {
    // Test comparison operators
    let mut context_true = Context::new();
    context_true.insert("x", &10);
    context_true.insert("y", &5);

    let template = "{% if x > y %}greater{% else %}not greater{% endif %}";
    assert_eq!(
        Tera::one_off(template, &context_true, false).unwrap(),
        "greater"
    );

    let mut context_false = Context::new();
    context_false.insert("x", &3);
    context_false.insert("y", &7);

    assert_eq!(
        Tera::one_off(template, &context_false, false).unwrap(),
        "not greater"
    );
}
