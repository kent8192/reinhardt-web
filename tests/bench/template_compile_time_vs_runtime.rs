//! Benchmark tests for Template Rendering Performance
//!
//! This test suite compares performance between:
//! - Direct Tera::one_off() calls
//! - TemplateHTMLRenderer (which uses Tera internally)
//!
//! Both approaches now use Tera as the underlying engine, so this benchmark
//! measures the overhead of the TemplateHTMLRenderer abstraction layer.
//!
//! # Performance Characteristics
//!
//! - **Direct Tera**: Minimal overhead, direct template engine access
//! - **TemplateHTMLRenderer**: Small overhead from JSON context conversion and abstraction
//!
//! # Template Features Supported
//!
//! Both approaches support full Tera template syntax:
//! - Variable substitution: `{{ variable }}`
//! - Conditionals: `{% if condition %}...{% endif %}`
//! - Loops: `{% for item in items %}...{% endfor %}`
//! - Filters: `{{ variable | filter }}`

use reinhardt_renderers::{Post, Renderer, TemplateHTMLRenderer};
use serde_json::json;
use std::time::Instant;
use tera::{Context, Tera};
use tokio::runtime::Runtime;

/// Benchmark: Simple template with single variable
///
/// Expected: Direct Tera slightly faster due to no abstraction overhead
#[test]
fn bench_simple_template_direct_vs_renderer() {
	let iterations = 10_000;
	let rt = Runtime::new().unwrap();

	// Direct Tera - Pre-create contexts
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

	// TemplateHTMLRenderer - Pre-create JSON contexts
	let renderer = TemplateHTMLRenderer::new();
	let runtime_contexts: Vec<_> = (0..iterations)
		.map(|_| {
			json!({
				"template_string": "<h1>{{ title }}</h1>",
				"title": "Hello World"
			})
		})
		.collect();

	let start = Instant::now();

	for context in &runtime_contexts {
		let _ = rt.block_on(renderer.render(context, None)).unwrap();
	}

	let renderer_duration = start.elapsed();

	println!("\nSimple Template Benchmark:");
	println!("  Direct Tera: {:?}", tera_duration);
	println!("  TemplateHTMLRenderer: {:?}", renderer_duration);
	println!(
		"  Ratio (Renderer/Direct): {:.2}x",
		renderer_duration.as_micros() as f64 / tera_duration.as_micros() as f64
	);
}

/// Benchmark: Complex template with 10 variables
///
/// Expected: Similar overhead ratio as simple template
#[test]
fn bench_complex_template_direct_vs_renderer() {
	let iterations = 10_000;
	let rt = Runtime::new().unwrap();

	// Direct Tera - Pre-create contexts
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

	// TemplateHTMLRenderer - Pre-create JSON contexts
	let renderer = TemplateHTMLRenderer::new();
	let runtime_contexts: Vec<_> = (0..iterations)
		.map(|_| {
			json!({
				"template_string": "<div>{{ v1 }}{{ v2 }}{{ v3 }}{{ v4 }}{{ v5 }}{{ v6 }}{{ v7 }}{{ v8 }}{{ v9 }}{{ v10 }}</div>",
				"v1": "val1", "v2": "val2", "v3": "val3", "v4": "val4", "v5": "val5",
				"v6": "val6", "v7": "val7", "v8": "val8", "v9": "val9", "v10": "val10"
			})
		})
		.collect();

	let start = Instant::now();

	for context in &runtime_contexts {
		let _ = rt.block_on(renderer.render(context, None)).unwrap();
	}

	let renderer_duration = start.elapsed();

	println!("\nComplex Template (10 variables) Benchmark:");
	println!("  Direct Tera: {:?}", tera_duration);
	println!("  TemplateHTMLRenderer: {:?}", renderer_duration);
	println!(
		"  Ratio (Renderer/Direct): {:.2}x",
		renderer_duration.as_micros() as f64 / tera_duration.as_micros() as f64
	);
}

/// Benchmark: List rendering with for loop
///
/// Both approaches now support {% for %} loops
#[test]
fn bench_list_template_direct_vs_renderer() {
	let iterations = 1_000;
	let rt = Runtime::new().unwrap();

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

	let template = r#"<h1>All Posts</h1><ul>{% for post in posts %}<li><h2>{{ post.title }}</h2><p>{{ post.content }}</p><small>by {{ post.author }}</small></li>{% endfor %}</ul>"#;

	// Direct Tera - Pre-create contexts with posts
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

	let start = Instant::now();

	for context in &contexts {
		let _ = Tera::one_off(template, context, true).unwrap();
	}

	let tera_duration = start.elapsed();

	// TemplateHTMLRenderer - Pre-create JSON contexts with posts
	let renderer = TemplateHTMLRenderer::new();
	let runtime_contexts: Vec<_> = (0..iterations)
		.map(|_| {
			let post_data: Vec<_> = posts
				.iter()
				.map(|p| {
					json!({
						"title": p.title,
						"content": p.content,
						"author": p.author
					})
				})
				.collect();
			json!({
				"template_string": template,
				"posts": post_data
			})
		})
		.collect();

	let start = Instant::now();

	for context in &runtime_contexts {
		let _ = rt.block_on(renderer.render(context, None)).unwrap();
	}

	let renderer_duration = start.elapsed();

	println!("\nList Template (100 items with for loop) Benchmark:");
	println!("  Direct Tera: {:?}", tera_duration);
	println!("  TemplateHTMLRenderer: {:?}", renderer_duration);
	println!(
		"  Ratio (Renderer/Direct): {:.2}x",
		renderer_duration.as_micros() as f64 / tera_duration.as_micros() as f64
	);
}

/// Benchmark: Real-world user profile template with conditional
///
/// Both approaches now support {% if %} conditionals
#[test]
fn bench_user_profile_direct_vs_renderer() {
	let iterations = 10_000;
	let rt = Runtime::new().unwrap();

	// Template with conditional logic - both approaches support this now
	let template = r#"
<div class="profile">
    <h1>{{ name }}</h1>
    <p>Email: {{ email }}</p>
    <p>Age: {{ age }}</p>
    {% if age >= 18 %}
    <span class="adult">Adult</span>
    {% else %}
    <span class="minor">Minor</span>
    {% endif %}
</div>
"#;

	// Direct Tera - Pre-create contexts
	let contexts: Vec<_> = (0..iterations)
		.map(|_| {
			let mut context = Context::new();
			context.insert("name", "Alice");
			context.insert("email", "alice@example.com");
			context.insert("age", &25);
			context
		})
		.collect();

	let start = Instant::now();

	for context in &contexts {
		let _ = Tera::one_off(template, context, true).unwrap();
	}

	let tera_duration = start.elapsed();

	// TemplateHTMLRenderer - Pre-create JSON contexts
	let renderer = TemplateHTMLRenderer::new();
	let runtime_contexts: Vec<_> = (0..iterations)
		.map(|_| {
			json!({
				"template_string": template,
				"name": "Alice",
				"email": "alice@example.com",
				"age": 25
			})
		})
		.collect();

	let start = Instant::now();

	for context in &runtime_contexts {
		let _ = rt.block_on(renderer.render(context, None)).unwrap();
	}

	let renderer_duration = start.elapsed();

	println!("\nUser Profile Template (with conditional) Benchmark:");
	println!("  Direct Tera: {:?}", tera_duration);
	println!("  TemplateHTMLRenderer: {:?}", renderer_duration);
	println!(
		"  Ratio (Renderer/Direct): {:.2}x",
		renderer_duration.as_micros() as f64 / tera_duration.as_micros() as f64
	);
}

/// Benchmark: Template with filters
///
/// Tests Tera filter support
#[test]
fn bench_template_with_filters() {
	let iterations = 10_000;
	let rt = Runtime::new().unwrap();

	let template = "<p>{{ name | upper }} - {{ description | truncate(length=20) }}</p>";

	// Direct Tera
	let contexts: Vec<_> = (0..iterations)
		.map(|_| {
			let mut context = Context::new();
			context.insert("name", "alice");
			context.insert(
				"description",
				"This is a very long description that should be truncated",
			);
			context
		})
		.collect();

	let start = Instant::now();

	for context in &contexts {
		let _ = Tera::one_off(template, context, true).unwrap();
	}

	let tera_duration = start.elapsed();

	// TemplateHTMLRenderer
	let renderer = TemplateHTMLRenderer::new();
	let runtime_contexts: Vec<_> = (0..iterations)
		.map(|_| {
			json!({
				"template_string": template,
				"name": "alice",
				"description": "This is a very long description that should be truncated"
			})
		})
		.collect();

	let start = Instant::now();

	for context in &runtime_contexts {
		let _ = rt.block_on(renderer.render(context, None)).unwrap();
	}

	let renderer_duration = start.elapsed();

	println!("\nTemplate with Filters Benchmark:");
	println!("  Direct Tera: {:?}", tera_duration);
	println!("  TemplateHTMLRenderer: {:?}", renderer_duration);
	println!(
		"  Ratio (Renderer/Direct): {:.2}x",
		renderer_duration.as_micros() as f64 / tera_duration.as_micros() as f64
	);
}

/// Summary benchmark report
///
/// This test prints a summary of template rendering performance characteristics
#[test]
fn bench_summary_report() {
	println!("\n=== Template Rendering Performance Summary ===\n");

	println!("Direct Tera:");
	println!("  - Minimal abstraction overhead");
	println!("  - Direct access to Tera context API");
	println!("  - Best for performance-critical paths\n");

	println!("TemplateHTMLRenderer (Tera internal):");
	println!("  - JSON-based context (easier integration with web APIs)");
	println!("  - Consistent interface with other renderers");
	println!("  - Small overhead from JSON->Tera context conversion");
	println!("  - Full Tera feature support (conditionals, loops, filters)\n");

	println!("Template Features (both approaches):");
	println!("  - Variable substitution: {{{{ variable }}}}");
	println!("  - Conditionals: {{% if condition %}}...{{% endif %}}");
	println!("  - Loops: {{% for item in items %}}...{{% endfor %}}");
	println!("  - Filters: {{{{ variable | filter }}}}\n");

	println!("Use Cases:");
	println!("  Direct Tera - Choose when:");
	println!("    - Maximum performance required");
	println!("    - Already working with Tera contexts");
	println!("    - Pre-compiled templates needed\n");

	println!("  TemplateHTMLRenderer - Choose when:");
	println!("    - JSON-based context is convenient");
	println!("    - Consistent renderer interface needed");
	println!("    - Integration with Reinhardt REST framework\n");

	println!("=== Both approaches use Tera for full template power ===\n");
}
