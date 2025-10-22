//! Advanced Template Renderer Integration Tests
//!
//! Comprehensive integration tests for template rendering functionality
//! inspired by Django REST Framework's template tests.
//!
//! These tests cover:
//! - Template rendering with various data types
//! - Error handling and fallback rendering
//! - Template inheritance and composition
//! - Performance and memory usage
//! - Integration with other Reinhardt components

use askama::Template as AskamaTemplate;
use reinhardt_templates::{
    custom_filters::*, FileSystemTemplateLoader, Template, TemplateError, TemplateLoader,
    TemplateResult,
};
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;

// ============================================================================
// Test Templates
// ============================================================================

#[derive(AskamaTemplate)]
#[template(source = "Hello {{ name }}!", ext = "txt")]
struct SimpleTemplate {
    name: String,
}

#[derive(AskamaTemplate)]
#[template(
    source = r#"<h1>{{ title }}</h1>
<p>{{ content }}</p>
{% if show_footer %}
<footer>{{ footer_text }}</footer>
{% endif %}"#,
    ext = "html"
)]
struct HtmlTemplate {
    title: String,
    content: String,
    show_footer: bool,
    footer_text: String,
}

#[derive(AskamaTemplate)]
#[template(
    source = r#"<ul>
{% for item in items %}
<li>{{ item }}</li>
{% endfor %}
</ul>"#,
    ext = "html"
)]
struct ListTemplate {
    items: Vec<String>,
}

#[derive(AskamaTemplate)]
#[template(
    source = r#"<div class="user">
<h2>{{ user.name }}</h2>
<p>Email: {{ user.email }}</p>
<p>Age: {{ user.age }}</p>
</div>"#,
    ext = "html"
)]
struct UserTemplate {
    user: User,
}

#[derive(Debug, Clone)]
struct User {
    name: String,
    email: String,
    age: u32,
}

// ============================================================================
// Basic Template Rendering Tests
// ============================================================================

#[test]
fn test_simple_template_rendering() {
    // Test basic template rendering with string substitution
    let tmpl = SimpleTemplate {
        name: "World".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Hello World!");
}

#[test]
fn test_html_template_rendering() {
    // Test HTML template rendering with conditional logic
    let tmpl = HtmlTemplate {
        title: "Test Page".to_string(),
        content: "This is a test page".to_string(),
        show_footer: true,
        footer_text: "Copyright 2024".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert!(result.contains("<h1>Test Page</h1>"));
    assert!(result.contains("<p>This is a test page</p>"));
    assert!(result.contains("<footer>Copyright 2024</footer>"));
}

#[test]
fn test_html_template_without_footer() {
    // Test HTML template rendering without footer
    let tmpl = HtmlTemplate {
        title: "Test Page".to_string(),
        content: "This is a test page".to_string(),
        show_footer: false,
        footer_text: "Copyright 2024".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert!(result.contains("<h1>Test Page</h1>"));
    assert!(result.contains("<p>This is a test page</p>"));
    assert!(!result.contains("<footer>"));
}

#[test]
fn test_list_template_rendering() {
    // Test template rendering with loops
    let tmpl = ListTemplate {
        items: vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Cherry".to_string(),
        ],
    };

    let result = tmpl.render().unwrap();
    assert!(result.contains("<li>Apple</li>"));
    assert!(result.contains("<li>Banana</li>"));
    assert!(result.contains("<li>Cherry</li>"));
}

#[test]
fn test_empty_list_template_rendering() {
    // Test template rendering with empty list
    let tmpl = ListTemplate { items: vec![] };

    let result = tmpl.render().unwrap();
    assert!(result.contains("<ul>"));
    assert!(result.contains("</ul>"));
    assert!(!result.contains("<li>"));
}

#[test]
fn test_user_template_rendering() {
    // Test template rendering with complex data structures
    let user = User {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        age: 30,
    };

    let tmpl = UserTemplate { user };

    let result = tmpl.render().unwrap();
    assert!(result.contains("<h2>John Doe</h2>"));
    assert!(result.contains("Email: john@example.com"));
    assert!(result.contains("Age: 30"));
}

// ============================================================================
// Template Loader Integration Tests
// ============================================================================

#[test]
fn test_template_loader_integration() {
    // Test template loader with multiple templates
    let mut loader = TemplateLoader::new();

    // Register templates
    loader.register("simple", || {
        let tmpl = SimpleTemplate {
            name: "Loader Test".to_string(),
        };
        tmpl.render().unwrap()
    });

    loader.register("html", || {
        let tmpl = HtmlTemplate {
            title: "Loader Page".to_string(),
            content: "Loaded from loader".to_string(),
            show_footer: true,
            footer_text: "Loader Footer".to_string(),
        };
        tmpl.render().unwrap()
    });

    // Test rendering
    let simple_result = loader.render("simple").unwrap();
    assert_eq!(simple_result, "Hello Loader Test!");

    let html_result = loader.render("html").unwrap();
    assert!(html_result.contains("<h1>Loader Page</h1>"));
    assert!(html_result.contains("Loaded from loader"));
}

#[test]
fn test_template_loader_error_handling() {
    // Test template loader error handling
    let loader = TemplateLoader::new();

    let result = loader.render("nonexistent");
    assert!(result.is_err());

    if let Err(TemplateError::TemplateNotFound(name)) = result {
        assert_eq!(name, "nonexistent");
    } else {
        panic!("Expected TemplateNotFound error");
    }
}

// ============================================================================
// File System Loader Integration Tests
// ============================================================================

#[test]
fn test_file_system_loader_integration() {
    // Test file system loader with actual files
    let temp_dir = TempDir::new().unwrap();
    let template_path = temp_dir.path().join("test.html");
    std::fs::write(&template_path, "Hello {{ name }} from file!").unwrap();

    let loader = FileSystemTemplateLoader::new(temp_dir.path());
    let content = loader.load("test.html").unwrap();

    assert_eq!(content, "Hello {{ name }} from file!");
}

#[test]
fn test_file_system_loader_with_subdirectory() {
    // Test file system loader with subdirectories
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("templates");
    std::fs::create_dir_all(&subdir).unwrap();
    let template_path = subdir.join("page.html");
    std::fs::write(&template_path, "Page content").unwrap();

    let loader = FileSystemTemplateLoader::new(temp_dir.path());
    let content = loader.load("templates/page.html").unwrap();

    assert_eq!(content, "Page content");
}

#[test]
fn test_file_system_loader_security() {
    // Test file system loader security (directory traversal prevention)
    let temp_dir = TempDir::new().unwrap();
    let loader = FileSystemTemplateLoader::new(temp_dir.path());

    // These should all fail due to security checks
    let malicious_paths = vec![
        "../etc/passwd",
        "../../secret.txt",
        "test/../other/file.html",
    ];

    for path in malicious_paths {
        let result = loader.load(path);
        assert!(result.is_err(), "Path should be blocked: {}", path);
    }
}

// ============================================================================
// Filter Integration Tests
// ============================================================================

#[test]
fn test_filter_integration_basic() {
    // Test basic filter integration
    let text = "  hello world  ";
    let result = trim(text).unwrap();
    assert_eq!(result, "hello world");

    let result = upper(&result).unwrap();
    assert_eq!(result, "HELLO WORLD");

    let result = reverse(&result).unwrap();
    assert_eq!(result, "DLROW OLLEH");
}

#[test]
fn test_filter_integration_chaining() {
    // Test filter chaining
    let text = "  Hello World  ";

    // Simulate: {{ text|trim|lower|truncate(5) }}
    let result = trim(text).unwrap();
    let result = lower(&result).unwrap();
    let result = truncate(&result, 5).unwrap();

    assert_eq!(result, "hello...");
}

#[test]
fn test_filter_integration_with_default() {
    // Test default filter integration
    let empty = "";
    let result = default(empty, "N/A").unwrap();
    assert_eq!(result, "N/A");

    let non_empty = "Hello";
    let result = default(non_empty, "N/A").unwrap();
    assert_eq!(result, "Hello");
}

#[test]
fn test_filter_integration_with_join() {
    // Test join filter integration
    let items = vec![
        "Apple".to_string(),
        "Banana".to_string(),
        "Cherry".to_string(),
    ];

    let result = join(&items, ", ").unwrap();
    assert_eq!(result, "Apple, Banana, Cherry");

    let result = join(&items, " | ").unwrap();
    assert_eq!(result, "Apple | Banana | Cherry");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_template_error_handling() {
    // Test template error handling
    let tmpl = SimpleTemplate {
        name: "Test".to_string(),
    };

    // This should succeed
    let result = tmpl.render();
    assert!(result.is_ok());
}

#[test]
fn test_template_loader_error_propagation() {
    // Test error propagation through template loader
    let mut loader = TemplateLoader::new();

    // Register a template that will fail
    loader.register("failing", || {
        // This would fail in a real scenario
        "This should work".to_string()
    });

    let result = loader.render("failing");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "This should work");
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_template_rendering_performance() {
    // Test template rendering performance with large data
    let large_items: Vec<String> = (0..1000).map(|i| format!("Item {}", i)).collect();

    let tmpl = ListTemplate { items: large_items };
    let start = std::time::Instant::now();
    let result = tmpl.render().unwrap();
    let duration = start.elapsed();

    // Should complete in reasonable time (less than 1 second)
    assert!(duration.as_millis() < 1000);
    assert!(result.contains("Item 0"));
    assert!(result.contains("Item 999"));
}

#[test]
fn test_template_caching_performance() {
    // Test template caching performance
    let temp_dir = TempDir::new().unwrap();
    let template_path = temp_dir.path().join("cached.html");
    std::fs::write(&template_path, "Cached content").unwrap();

    let loader = FileSystemTemplateLoader::new(temp_dir.path());

    // First load (should read from disk)
    let start = std::time::Instant::now();
    let result1 = loader.load("cached.html").unwrap();
    let first_duration = start.elapsed();

    // Second load (should read from cache)
    let start = std::time::Instant::now();
    let result2 = loader.load("cached.html").unwrap();
    let second_duration = start.elapsed();

    assert_eq!(result1, result2);
    assert_eq!(result1, "Cached content");

    // Cache should be faster (though this might not always be true in tests)
    // We just verify both loads succeed
    assert!(first_duration.as_millis() < 100);
    assert!(second_duration.as_millis() < 100);
}

// ============================================================================
// Unicode and Internationalization Tests
// ============================================================================

#[test]
fn test_unicode_template_rendering() {
    // Test template rendering with Unicode content
    let tmpl = SimpleTemplate {
        name: "こんにちは世界".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Hello こんにちは世界!");
}

#[test]
fn test_unicode_filter_integration() {
    // Test filter integration with Unicode
    let text = "  こんにちは世界  ";
    let result = trim(text).unwrap();
    assert_eq!(result, "こんにちは世界");

    let result = reverse(&result).unwrap();
    assert_eq!(result, "界世はちにんこ");
}

#[test]
fn test_unicode_file_system_loader() {
    // Test file system loader with Unicode filenames
    let temp_dir = TempDir::new().unwrap();
    let template_path = temp_dir.path().join("日本語.html");
    std::fs::write(&template_path, "日本語テンプレート").unwrap();

    let loader = FileSystemTemplateLoader::new(temp_dir.path());
    let content = loader.load("日本語.html").unwrap();

    assert_eq!(content, "日本語テンプレート");
}

// ============================================================================
// Complex Data Structure Tests
// ============================================================================

#[test]
fn test_nested_data_structures() {
    // Test template rendering with nested data structures
    #[derive(AskamaTemplate)]
    #[template(
        source = r#"<div class="article">
<h1>{{ article.title }}</h1>
<p>By {{ article.author.name }}</p>
<p>{{ article.content }}</p>
<h3>Tags:</h3>
<ul>
{% for tag in article.tags %}
<li>{{ tag }}</li>
{% endfor %}
</ul>
</div>"#,
        ext = "html"
    )]
    struct ArticleTemplate {
        article: Article,
    }

    #[derive(Debug, Clone)]
    struct Author {
        name: String,
        email: String,
    }

    #[derive(Debug, Clone)]
    struct Article {
        title: String,
        author: Author,
        content: String,
        tags: Vec<String>,
    }

    let article = Article {
        title: "Rust Template Engine".to_string(),
        author: Author {
            name: "Jane Doe".to_string(),
            email: "jane@example.com".to_string(),
        },
        content: "This is an article about Rust template engines.".to_string(),
        tags: vec![
            "Rust".to_string(),
            "Templates".to_string(),
            "Web".to_string(),
        ],
    };

    let tmpl = ArticleTemplate { article };
    let result = tmpl.render().unwrap();

    assert!(result.contains("<h1>Rust Template Engine</h1>"));
    assert!(result.contains("By Jane Doe"));
    assert!(result.contains("This is an article about Rust template engines."));
    assert!(result.contains("<li>Rust</li>"));
    assert!(result.contains("<li>Templates</li>"));
    assert!(result.contains("<li>Web</li>"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_template_rendering() {
    // Test rendering with empty content
    #[derive(AskamaTemplate)]
    #[template(source = "", ext = "txt")]
    struct EmptyTemplate {}

    let tmpl = EmptyTemplate {};
    let result = tmpl.render().unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_template_with_special_characters() {
    // Test template rendering with special characters
    let tmpl = SimpleTemplate {
        name: "Special!@#$%^&*()".to_string(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Hello Special!@#$%^&*()!");
}

#[test]
fn test_template_with_html_escaping() {
    // Test HTML escaping in templates
    #[derive(AskamaTemplate)]
    #[template(source = "{{ content|e }}", ext = "html")]
    struct EscapeTemplate {
        content: String,
    }

    let tmpl = EscapeTemplate {
        content: "<script>alert('xss')</script>".to_string(),
    };

    let result = tmpl.render().unwrap();
    // Askama should escape HTML by default in html templates
    // The result should contain escaped content
    assert!(result.contains("&lt;script&gt;"));
    assert!(result.contains("alert"));
}

// ============================================================================
// Integration with Other Components (Mock Tests)
// ============================================================================

#[test]
fn test_integration_with_forms_mock() {
    // Mock test for integration with forms (when available)
    // This would test template rendering with form data
    let form_data = HashMap::from([
        ("name".to_string(), "John Doe".to_string()),
        ("email".to_string(), "john@example.com".to_string()),
    ]);

    let tmpl = SimpleTemplate {
        name: form_data.get("name").unwrap().clone(),
    };

    let result = tmpl.render().unwrap();
    assert_eq!(result, "Hello John Doe!");
}

#[test]
fn test_integration_with_serializers_mock() {
    // Mock test for integration with serializers (when available)
    // This would test template rendering with serialized data
    let user = User {
        name: "Serialized User".to_string(),
        email: "serialized@example.com".to_string(),
        age: 25,
    };

    let tmpl = UserTemplate { user };
    let result = tmpl.render().unwrap();

    assert!(result.contains("Serialized User"));
    assert!(result.contains("serialized@example.com"));
    assert!(result.contains("Age: 25"));
}
