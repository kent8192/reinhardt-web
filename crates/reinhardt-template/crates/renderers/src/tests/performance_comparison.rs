//! Performance comparison tests between Runtime and Tera template rendering
//!
//! These tests demonstrate the performance characteristics of both approaches:
//!
//! - **Runtime (Phase 2)**: O(n + m) single-pass variable substitution
//! - **Tera (Embedded)**: Templates embedded at compile time with runtime rendering
//!
//! Expected results: Tera provides good balance between flexibility and performance.

use crate::tera_renderer::{UserData, UserListTemplate, UserTemplate};
use crate::template_html_renderer::TemplateHTMLRenderer;
use std::collections::HashMap;
use std::time::Instant;

/// Performance test: Simple variable substitution
///
/// Compares runtime vs compile-time for a template with 10 variables
#[test]
fn test_performance_simple_template() {
    const ITERATIONS: usize = 1000;

    // Compile-time (Tera)
    let template = UserTemplate::new(
        "Test User".to_string(),
        "test@example.com".to_string(),
        25,
    );

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = template.render_user().expect("Failed to render");
    }
    let compile_time_duration = start.elapsed();

    // Runtime (TemplateHTMLRenderer)
    let mut context = HashMap::new();
    context.insert("name".to_string(), "Test User".to_string());
    context.insert("email".to_string(), "test@example.com".to_string());
    context.insert("age".to_string(), "25".to_string());

    let template_str = "<h1>{{ name }}</h1><p>Email: {{ email }}</p><p>Age: {{ age }}</p>";

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = TemplateHTMLRenderer::substitute_variables_single_pass(template_str, &context);
    }
    let runtime_duration = start.elapsed();

    println!("\n=== Simple Template Performance (10 variables, {} iterations) ===", ITERATIONS);
    println!("Tera (Embedded):   {:?}", compile_time_duration);
    println!("Runtime (Phase 2): {:?}", runtime_duration);

    if runtime_duration > compile_time_duration {
        let speedup = runtime_duration.as_nanos() as f64 / compile_time_duration.as_nanos() as f64;
        println!("Speedup: {:.1}x faster with Tera", speedup);
    }
}

/// Performance test: List rendering with 100 items
///
/// Demonstrates compile-time advantage for complex templates
#[test]
fn test_performance_list_template() {
    const LIST_SIZE: usize = 100;
    const ITERATIONS: usize = 100;

    // Compile-time (Tera)
    let users: Vec<UserData> = (0..LIST_SIZE)
        .map(|i| UserData::new(format!("User {}", i), format!("user{}@test.com", i)))
        .collect();

    let template = UserListTemplate::new(users, "User List".to_string());

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = template.render_list().expect("Failed to render");
    }
    let compile_time_duration = start.elapsed();

    // Runtime (TemplateHTMLRenderer) - Simulated list rendering
    let mut context = HashMap::new();
    context.insert("title".to_string(), "User List".to_string());

    // Simulate list items
    let mut list_html = String::new();
    for i in 0..LIST_SIZE {
        list_html.push_str(&format!(
            "<li>User {} (user{}@test.com)</li>",
            i, i
        ));
    }
    context.insert("user_list".to_string(), list_html);

    let template_str = "<h1>{{ title }}</h1><ul>{{ user_list }}</ul>";

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = TemplateHTMLRenderer::substitute_variables_single_pass(template_str, &context);
    }
    let runtime_duration = start.elapsed();

    println!("\n=== List Template Performance ({} items, {} iterations) ===", LIST_SIZE, ITERATIONS);
    println!("Tera (Embedded):   {:?}", compile_time_duration);
    println!("Runtime (Phase 2): {:?}", runtime_duration);

    if runtime_duration > compile_time_duration {
        let speedup = runtime_duration.as_nanos() as f64 / compile_time_duration.as_nanos() as f64;
        println!("Speedup: {:.1}x faster with Tera", speedup);
    }
}

/// Benchmark: Tera rendering scalability
///
/// Demonstrates Tera's performance with embedded templates
#[test]
fn test_tera_scalability() {
    const ITERATIONS: usize = 1000;

    let sizes = vec![10, 50, 100, 500, 1000];

    println!("\n=== Tera Scalability Test ===");

    for size in sizes {
        let users: Vec<UserData> = (0..size)
            .map(|i| UserData::new(format!("User {}", i), format!("user{}@test.com", i)))
            .collect();

        let template = UserListTemplate::new(users, format!("List of {}", size));

        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = template.render_list().expect("Failed to render");
        }
        let duration = start.elapsed();

        println!("  {} items: {:?}", size, duration);
    }
}

/// Benchmark: Runtime rendering scalability
///
/// Demonstrates O(n + m) complexity - rendering time increases with template size and variables
#[test]
fn test_runtime_scalability() {
    const ITERATIONS: usize = 1000;

    // Test with different numbers of variables
    let var_counts = vec![10, 50, 100, 500, 1000];

    println!("\n=== Runtime Scalability Test ===");

    for count in var_counts {
        let mut context = HashMap::new();
        let mut template = String::new();

        for i in 0..count {
            let var_name = format!("var{}", i);
            let var_value = format!("value{}", i);
            context.insert(var_name.clone(), var_value);

            template.push_str(&format!("<div>{{{{ {} }}}}</div>", var_name));
        }

        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _ = TemplateHTMLRenderer::substitute_variables_single_pass(&template, &context);
        }
        let duration = start.elapsed();

        println!("  {} variables: {:?}", count, duration);
    }

    // Runtime rendering shows linear growth with number of variables
    // but Phase 2 optimization (single-pass) keeps it efficient
}

/// Memory usage comparison
///
/// This test demonstrates memory characteristics of both approaches
#[test]
fn test_memory_characteristics() {
    // Tera: Templates embedded at compile time
    // - Template string embedded in binary
    // - Runtime parsing and rendering

    let tera_template = UserTemplate::new(
        "Memory Test".to_string(),
        "memory@test.com".to_string(),
        30,
    );

    let tera_size = std::mem::size_of_val(&tera_template);

    // Runtime: Template string needs to be stored and parsed
    let runtime_template = "<h1>{{ name }}</h1><p>Email: {{ email }}</p><p>Age: {{ age }}</p>";
    let runtime_context = {
        let mut ctx = HashMap::new();
        ctx.insert("name".to_string(), "Memory Test".to_string());
        ctx.insert("email".to_string(), "memory@test.com".to_string());
        ctx.insert("age".to_string(), "30".to_string());
        ctx
    };

    let runtime_template_size = runtime_template.len();
    let runtime_context_size = std::mem::size_of_val(&runtime_context);

    println!("\n=== Memory Characteristics ===");
    println!("Tera template data:      {} bytes", tera_size);
    println!("Runtime template string: {} bytes", runtime_template_size);
    println!("Runtime context:         {} bytes", runtime_context_size);
    println!(
        "Runtime total:           {} bytes",
        runtime_template_size + runtime_context_size
    );
}

/// End-to-end rendering comparison
///
/// Realistic scenario: Rendering a user profile page 10,000 times
#[test]
fn test_end_to_end_comparison() {
    const ITERATIONS: usize = 10_000;

    println!("\n=== End-to-End Rendering Comparison ({} iterations) ===", ITERATIONS);

    // Tera
    let template = UserTemplate::new(
        "John Doe".to_string(),
        "john@example.com".to_string(),
        35,
    );

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = template.render_user().expect("Failed to render");
    }
    let tera_duration = start.elapsed();

    // Runtime
    let mut context = HashMap::new();
    context.insert("name".to_string(), "John Doe".to_string());
    context.insert("email".to_string(), "john@example.com".to_string());
    context.insert("age".to_string(), "35".to_string());

    let template_str = concat!(
        "<!DOCTYPE html><html><head><title>User Profile</title></head><body>",
        "<h1>{{ name }}</h1>",
        "<p>Email: {{ email }}</p>",
        "<p>Age: {{ age }}</p>",
        "</body></html>"
    );

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = TemplateHTMLRenderer::substitute_variables_single_pass(template_str, &context);
    }
    let runtime_duration = start.elapsed();

    println!("Tera (Embedded):   {:?}", tera_duration);
    println!("Runtime (Phase 2): {:?}", runtime_duration);

    if runtime_duration > tera_duration {
        let speedup = runtime_duration.as_nanos() as f64 / tera_duration.as_nanos() as f64;
        println!("Speedup: {:.1}x faster with Tera", speedup);
    }
}
