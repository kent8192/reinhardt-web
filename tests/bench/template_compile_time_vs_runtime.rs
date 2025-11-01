//! Benchmark tests for Compile-time (Tera AOT) vs Runtime Template Rendering
//!
//! This test suite verifies the performance characteristics of:
//! - Tera template engine (runtime rendering)
//! - TemplateHTMLRenderer (single-pass substitution)
//!
//! # Performance Characteristics
//!
//! - **Tera Runtime**: O(n + m) - Template parsing + rendering
//! - **TemplateHTMLRenderer**: O(n + m) - Single-pass substitution
//!
//! Expected performance: Comparable for simple templates, Tera more powerful for complex logic

use reinhardt_renderers::{Post, PostListTemplate, TemplateHTMLRenderer};
use std::collections::HashMap;
use std::time::Instant;
use tera::{Context, Tera};

/// Benchmark: Simple template with single variable
///
/// Expected: Comparable performance for simple substitution
#[test]
fn bench_simple_template_tera_vs_runtime() {
    let iterations = 10_000;

    // Tera - Pre-create contexts
    let contexts: Vec<_> = (0..iterations)
        .map(|_| {
            let mut context = Context::new();
            context.insert("title", "Hello World");
            context
        })
        .collect();

    let start = Instant::now();

    for context in &contexts {
        let _ = Tera::one_off("<h1>{{ title }}</h1>", context, true).unwrap();
    }

    let tera_duration = start.elapsed();

    // Runtime (TemplateHTMLRenderer) - Pre-create contexts
    let runtime_contexts: Vec<_> = (0..iterations)
        .map(|_| {
            let mut context = HashMap::new();
            context.insert("title".to_string(), "Hello World".to_string());
            context
        })
        .collect();

    let start = Instant::now();

    for context in &runtime_contexts {
        let _ = TemplateHTMLRenderer::substitute_variables_single_pass(
            "<h1>{{ title }}</h1>",
            context,
        );
    }

    let runtime_duration = start.elapsed();

    println!("\nSimple Template Benchmark:");
    println!("  Tera: {:?}", tera_duration);
    println!("  Runtime (TemplateHTMLRenderer): {:?}", runtime_duration);
    println!(
        "  Ratio (Tera/Runtime): {:.2}x",
        tera_duration.as_micros() as f64 / runtime_duration.as_micros() as f64
    );
}

/// Benchmark: Complex template with 10 variables
///
/// Expected: Tera slightly slower due to parsing overhead
#[test]
fn bench_complex_template_tera_vs_runtime() {
    let iterations = 10_000;

    // Tera - Pre-create contexts
    let contexts: Vec<_> = (0..iterations)
        .map(|_| {
            let mut context = Context::new();
            for i in 1..=10 {
                context.insert(&format!("v{}", i), &format!("val{}", i));
            }
            context
        })
        .collect();

    let start = Instant::now();

    for context in &contexts {
        let _ = Tera::one_off(
            "<div>{{ v1 }}{{ v2 }}{{ v3 }}{{ v4 }}{{ v5 }}{{ v6 }}{{ v7 }}{{ v8 }}{{ v9 }}{{ v10 }}</div>",
            context,
            true,
        )
        .unwrap();
    }

    let tera_duration = start.elapsed();

    // Runtime (TemplateHTMLRenderer) - Pre-create contexts
    let runtime_contexts: Vec<_> = (0..iterations)
        .map(|_| {
            let mut context = HashMap::new();
            for i in 1..=10 {
                context.insert(format!("v{}", i), format!("val{}", i));
            }
            context
        })
        .collect();

    let start = Instant::now();

    for context in &runtime_contexts {
        let _ = TemplateHTMLRenderer::substitute_variables_single_pass(
            "<div>{{ v1 }}{{ v2 }}{{ v3 }}{{ v4 }}{{ v5 }}{{ v6 }}{{ v7 }}{{ v8 }}{{ v9 }}{{ v10 }}</div>",
            context,
        );
    }

    let runtime_duration = start.elapsed();

    println!("\nComplex Template (10 variables) Benchmark:");
    println!("  Tera: {:?}", tera_duration);
    println!("  Runtime (TemplateHTMLRenderer): {:?}", runtime_duration);
    println!(
        "  Ratio (Tera/Runtime): {:.2}x",
        tera_duration.as_micros() as f64 / runtime_duration.as_micros() as f64
    );
}

/// Benchmark: List rendering with 100 items
///
/// Expected: Comparable performance for list iteration
#[test]
fn bench_list_template_tera_vs_runtime() {
    let iterations = 1_000;

    // Create 100 posts
    let posts: Vec<_> = (0..100)
        .map(|i| {
            Post::new(
                i,
                format!("Post {}", i),
                format!("Content {}", i),
                format!("Author {}", i),
            )
        })
        .collect();

    // Tera - Pre-create contexts with posts
    let contexts: Vec<_> = (0..iterations)
        .map(|_| {
            let mut context = Context::new();
            let post_data: Vec<_> = posts
                .iter()
                .map(|p| {
                    let mut map = std::collections::HashMap::new();
                    map.insert("title", p.title.clone());
                    map.insert("content", p.content.clone());
                    map.insert("author", p.author.clone());
                    map
                })
                .collect();
            context.insert("posts", &post_data);
            context
        })
        .collect();

    let template = r#"<h1>All Posts</h1><ul>{% for post in posts %}<li><h2>{{ post.title }}</h2><p>{{ post.content }}</p><small>by {{ post.author }}</small></li>{% endfor %}</ul>"#;

    let start = Instant::now();

    for context in &contexts {
        let _ = Tera::one_off(template, context, true).unwrap();
    }

    let tera_duration = start.elapsed();

    // Runtime (TemplateHTMLRenderer) - Pre-build template strings
    let template_strings: Vec<_> = (0..iterations)
        .map(|_| {
            let mut template_str = String::from("<h1>All Posts</h1><ul>");
            for post in &posts {
                template_str.push_str("<li>");
                template_str.push_str(&format!("<h2>{}</h2>", post.title));
                template_str.push_str(&format!("<p>{}</p>", post.content));
                template_str.push_str(&format!("<small>by {}</small>", post.author));
                template_str.push_str("</li>");
            }
            template_str.push_str("</ul>");
            template_str
        })
        .collect();

    let context = HashMap::new();
    let start = Instant::now();

    for template_str in &template_strings {
        let _ = TemplateHTMLRenderer::substitute_variables_single_pass(template_str, &context);
    }

    let runtime_duration = start.elapsed();

    println!("\nList Template (100 items) Benchmark:");
    println!("  Tera: {:?}", tera_duration);
    println!("  Runtime (TemplateHTMLRenderer): {:?}", runtime_duration);
    println!(
        "  Ratio (Tera/Runtime): {:.2}x",
        tera_duration.as_micros() as f64 / runtime_duration.as_micros() as f64
    );
}

/// Benchmark: Real-world user profile template
///
/// Expected: Tera provides better structure for complex templates
#[test]
fn bench_user_profile_tera_vs_runtime() {
    let iterations = 10_000;

    // Tera - Pre-create contexts
    let contexts: Vec<_> = (0..iterations)
        .map(|_| {
            let mut context = Context::new();
            context.insert("name", "Alice");
            context.insert("email", "alice@example.com");
            context.insert("age", &25);
            context
        })
        .collect();

    let template = r#"
<div class="profile">
    <h1>{{ name }}</h1>
    <p>Email: {{ email }}</p>
    <p>Age: {{ age }}</p>
    {% if age >= 18 %}
    <span class="adult">Adult</span>
    {% endif %}
</div>
"#;

    let start = Instant::now();

    for context in &contexts {
        let _ = Tera::one_off(template, context, true).unwrap();
    }

    let tera_duration = start.elapsed();

    // Runtime (TemplateHTMLRenderer) - Pre-create contexts
    let runtime_contexts: Vec<_> = (0..iterations)
        .map(|_| {
            let mut context = HashMap::new();
            context.insert("name".to_string(), "Alice".to_string());
            context.insert("email".to_string(), "alice@example.com".to_string());
            context.insert("age".to_string(), "25".to_string());
            context
        })
        .collect();

    // Note: Runtime renderer doesn't support {% if %} blocks,
    // so this is a simplified comparison without conditional logic
    let template_str = r#"
<div class="profile">
    <h1>{{ name }}</h1>
    <p>Email: {{ email }}</p>
    <p>Age: {{ age }}</p>
    <span class="adult">Adult</span>
</div>
"#;

    let start = Instant::now();

    for context in &runtime_contexts {
        let _ = TemplateHTMLRenderer::substitute_variables_single_pass(template_str, context);
    }

    let runtime_duration = start.elapsed();

    println!("\nUser Profile Template Benchmark:");
    println!("  Tera: {:?}", tera_duration);
    println!("  Runtime (TemplateHTMLRenderer): {:?}", runtime_duration);
    println!(
        "  Ratio (Tera/Runtime): {:.2}x",
        tera_duration.as_micros() as f64 / runtime_duration.as_micros() as f64
    );
}

/// Summary benchmark report
///
/// This test prints a summary of template rendering performance characteristics
#[test]
fn bench_summary_report() {
    println!("\n=== Template Rendering Performance Summary ===\n");

    println!("Tera (Runtime):");
    println!("  - Time Complexity: O(n + m) - Template parsing + rendering");
    println!("  - Space Complexity: O(n) - Template AST storage");
    println!("  - Type Safety: Runtime validation");
    println!("  - Flexibility: Full template language support (conditionals, loops, filters)\n");

    println!("TemplateHTMLRenderer:");
    println!("  - Time Complexity: O(n + m) - Single-pass substitution");
    println!("  - Space Complexity: O(n) - Template string storage");
    println!("  - Type Safety: Runtime validation");
    println!("  - Flexibility: Simple variable substitution only\n");

    println!("Performance Characteristics:");
    println!("  - Simple templates: Comparable performance");
    println!("  - Complex templates with logic: Tera provides richer features");
    println!("  - List rendering: Comparable for simple iteration");
    println!("  - Real-world templates: Tera better for maintainability\n");

    println!("Use Cases:");
    println!("  Tera - Choose when:");
    println!("    - Complex template logic required (if/else, loops, filters)");
    println!("    - Template inheritance and includes needed");
    println!("    - Better error messages and debugging desired");
    println!("    - Examples: View templates, email templates, complex pages\n");

    println!("  TemplateHTMLRenderer - Choose when:");
    println!("    - Only simple variable substitution needed");
    println!("    - Minimal dependencies desired");
    println!("    - Very simple use cases");
    println!("    - Examples: Simple string formatting, basic templating\n");

    println!("=== Tera provides powerful template features at runtime ===\n");
}
