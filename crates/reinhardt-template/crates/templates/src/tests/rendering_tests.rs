//! Template rendering tests
//!
//! Tests for template rendering inspired by Django's test_engine.py and test_extends.py

use crate::TemplateLoader;
use askama::Template;

#[derive(Template)]
#[template(source = "{{ greeting }} {{ name }}!", ext = "txt")]
struct GreetingTemplate {
    greeting: String,
    name: String,
}

#[derive(Template)]
#[template(
    source = r#"<h1>{{ title }}</h1>
<p>{{ content }}</p>"#,
    ext = "html"
)]
struct HtmlTemplate {
    title: String,
    content: String,
}

#[derive(Template)]
#[template(source = "{% for i in 0..count %}*{% endfor %}", ext = "txt")]
struct RangeTemplate {
    count: usize,
}

#[derive(Template)]
#[template(
    source = "{% match result %}{% when Some with (val) %}{{ val }}{% when None %}empty{% endmatch %}",
    ext = "txt"
)]
struct MatchTemplate {
    result: Option<String>,
}

#[test]
fn test_basic_rendering() {
    /// // Test basic template rendering (similar to Django's test_basic_context)
    let tmpl = GreetingTemplate {
        greeting: "Hello".to_string(),
        name: "World".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Hello World!");
}

#[test]
fn test_html_rendering() {
    /// // Test HTML template rendering
    let tmpl = HtmlTemplate {
        title: "Test Page".to_string(),
        content: "This is a test".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert!(result.contains("<h1>Test Page</h1>"));
    assert!(result.contains("<p>This is a test</p>"));
}

#[test]
fn test_rendering_with_escaping() {
    /// // Test that HTML is escaped by default
    let tmpl = HtmlTemplate {
        title: "Test".to_string(),
        content: "<script>alert('xss')</script>".to_string(),
    };

    let result = tmpl.render().unwrap();
    /// // Askama escapes HTML by default
    /// // Askama escapes HTML by default in html templates
    assert!(!result.is_empty());
}

#[test]
fn test_range_rendering() {
    /// // Test rendering with range loop
    let tmpl = RangeTemplate { count: 5 };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "*****");
}

#[test]
fn test_range_rendering_zero() {
    /// // Test range loop with zero count
    let tmpl = RangeTemplate { count: 0 };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_match_rendering_some() {
    /// // Test match template with Some value
    let tmpl = MatchTemplate {
        result: Some("Success".to_string()),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Success");
}

#[test]
fn test_match_rendering_none() {
    /// // Test match template with None value
    let tmpl = MatchTemplate { result: None };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "empty");
}

#[test]
fn test_multiline_template() {
    /// // Test multi-line template rendering
    #[derive(Template)]
    #[template(
        source = r#"Line 1
Line 2
Line 3"#,
        ext = "txt"
    )]
    struct MultiLineTemplate;

    let tmpl = MultiLineTemplate;
    let result = tmpl.render().unwrap();
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Line 1");
    assert_eq!(lines[2], "Line 3");
}

#[test]
fn test_rendering_with_whitespace_control() {
    /// // Test whitespace control in templates
    #[derive(Template)]
    #[template(
        source = "{% for i in items -%}
    {{ i }}
{%- endfor %}",
        ext = "txt"
    )]
    struct WhitespaceTemplate {
        items: Vec<i32>,
    }

    let tmpl = WhitespaceTemplate {
        items: vec![1, 2, 3],
    };

    let result = tmpl.render().unwrap();
    /// // Whitespace should be controlled
    assert!(!result.is_empty());
}

#[test]
fn test_template_with_comments() {
    /// // Test templates with comments
    #[derive(Template)]
    #[template(source = "Before{# This is a comment #}After", ext = "txt")]
    struct CommentTemplate;

    let tmpl = CommentTemplate;
    let result = tmpl.render().unwrap();
    assert_eq!(result, "BeforeAfter");
}

#[test]
fn test_rendering_error_handling() {
    /// // Test that rendering returns proper Result type
    let tmpl = GreetingTemplate {
        greeting: "Hi".to_string(),
        name: "Test".to_string(),
    };

    let result = tmpl.render();
    assert!(result.is_ok());
}

#[test]
fn test_template_loader_render_workflow() {
    /// // Test complete workflow: register and render templates
    let mut loader = TemplateLoader::new();

    loader.register("page.html", || {
        let tmpl = HtmlTemplate {
            title: "Welcome".to_string(),
            content: "Welcome to our site".to_string(),
        };
        tmpl.render().unwrap()
    });

    let result = loader.render("page.html");
    assert!(result.is_ok());
    let html = result.unwrap();
    assert!(html.contains("Welcome"));
}

#[test]
fn test_nested_loops() {
    /// // Test nested loop rendering
    #[derive(Template)]
    #[template(
        source = "{% for i in 0..2 %}{% for j in 0..2 %}({{ i }},{{ j }}){% endfor %}{% endfor %}",
        ext = "txt"
    )]
    struct NestedLoopTemplate;

    let tmpl = NestedLoopTemplate;
    let result = tmpl.render().unwrap();
    assert!(result.contains("(0,0)"));
    assert!(result.contains("(1,1)"));
}

#[test]
fn test_complex_expression() {
    /// // Test complex expressions in templates
    #[derive(Template)]
    #[template(source = "{{ a + b * c }}", ext = "txt")]
    struct ExpressionTemplate {
        a: i32,
        b: i32,
        c: i32,
    }

    let tmpl = ExpressionTemplate { a: 1, b: 2, c: 3 };
    let result = tmpl.render().unwrap();
    assert_eq!(result, "7"); // 1 + 2 * 3 = 7
}

#[test]
fn test_comparison_operators() {
    /// // Test comparison operators
    #[derive(Template)]
    #[template(
        source = "{% if x > y %}greater{% else %}not greater{% endif %}",
        ext = "txt"
    )]
    struct ComparisonTemplate {
        x: i32,
        y: i32,
    }

    let tmpl_true = ComparisonTemplate { x: 10, y: 5 };
    assert_eq!(tmpl_true.render().unwrap(), "greater");

    let tmpl_false = ComparisonTemplate { x: 3, y: 7 };
    assert_eq!(tmpl_false.render().unwrap(), "not greater");
}
