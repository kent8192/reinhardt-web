//! Template system benchmarks
//!
//! Benchmarks for template loading and rendering operations:
//! - FileSystemTemplateLoader (with/without cache)
//! - TemplateLoader registration and rendering
//! - TemplateHTMLRenderer (Tera-based)

use criterion::{Criterion, criterion_group, criterion_main};
use reinhardt_renderers::TemplateHTMLRenderer;
use reinhardt_templates::FileSystemTemplateLoader;
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;
use tera::{Context, Tera};
use tokio::runtime::Runtime;

/// Create temporary directory with test template files
fn setup_test_templates() -> TempDir {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");

	// Simple template
	fs::write(
		temp_dir.path().join("simple.html"),
		"<html><body>{{ title }}</body></html>",
	)
	.expect("Failed to write template");

	// Complex template with conditionals
	fs::write(
		temp_dir.path().join("user.html"),
		r#"<div class="profile">
    <h1>{{ name }}</h1>
    <p>Email: {{ email }}</p>
    {% if age >= 18 %}
    <span class="adult">Adult</span>
    {% else %}
    <span class="minor">Minor</span>
    {% endif %}
</div>"#,
	)
	.expect("Failed to write template");

	// List template
	fs::write(
		temp_dir.path().join("list.html"),
		r#"<ul>{% for item in items %}<li>{{ item }}</li>{% endfor %}</ul>"#,
	)
	.expect("Failed to write template");

	temp_dir
}

fn benchmark_fs_loader(c: &mut Criterion) {
	let temp_dir = setup_test_templates();

	// FileSystemTemplateLoader creation (with cache)
	c.bench_function("fs_loader_create_with_cache", |b| {
		b.iter(|| black_box(FileSystemTemplateLoader::new(temp_dir.path())));
	});

	// FileSystemTemplateLoader creation (no cache)
	c.bench_function("fs_loader_create_no_cache", |b| {
		b.iter(|| black_box(FileSystemTemplateLoader::new_without_cache(temp_dir.path())));
	});

	// Template load (no cache) - measures disk I/O
	c.bench_function("fs_loader_load_no_cache", |b| {
		let loader = FileSystemTemplateLoader::new_without_cache(temp_dir.path());
		b.iter(|| black_box(loader.load("simple.html")));
	});

	// Template load (cached, first access)
	c.bench_function("fs_loader_load_cached_first", |b| {
		b.iter(|| {
			let loader = FileSystemTemplateLoader::new(temp_dir.path());
			black_box(loader.load("simple.html"))
		});
	});

	// Template load (cached, subsequent access)
	c.bench_function("fs_loader_load_cached_subsequent", |b| {
		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		// Warm up cache
		let _ = loader.load("simple.html");
		b.iter(|| black_box(loader.load("simple.html")));
	});
}

fn benchmark_tera_rendering(c: &mut Criterion) {
	let temp_dir = setup_test_templates();
	let loader = FileSystemTemplateLoader::new(temp_dir.path());

	// Simple variable substitution
	c.bench_function("tera_render_simple", |b| {
		let template = loader.load("simple.html").unwrap();
		let mut context = Context::new();
		context.insert("title", "Hello World");

		b.iter(|| black_box(Tera::one_off(&template, &context, true)));
	});

	// Conditional rendering
	c.bench_function("tera_render_conditional", |b| {
		let template = loader.load("user.html").unwrap();
		let mut context = Context::new();
		context.insert("name", "Alice");
		context.insert("email", "alice@example.com");
		context.insert("age", &25);

		b.iter(|| black_box(Tera::one_off(&template, &context, true)));
	});

	// List rendering
	c.bench_function("tera_render_list_10", |b| {
		let template = loader.load("list.html").unwrap();
		let items: Vec<&str> = (0..10).map(|_| "item").collect();
		let mut context = Context::new();
		context.insert("items", &items);

		b.iter(|| black_box(Tera::one_off(&template, &context, true)));
	});

	c.bench_function("tera_render_list_100", |b| {
		let template = loader.load("list.html").unwrap();
		let items: Vec<&str> = (0..100).map(|_| "item").collect();
		let mut context = Context::new();
		context.insert("items", &items);

		b.iter(|| black_box(Tera::one_off(&template, &context, true)));
	});
}

fn benchmark_template_html_renderer(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	// TemplateHTMLRenderer creation
	c.bench_function("template_html_renderer_create", |b| {
		b.iter(|| black_box(TemplateHTMLRenderer::new()));
	});

	// Simple rendering
	c.bench_function("template_html_renderer_simple", |b| {
		let renderer = TemplateHTMLRenderer::new();
		let context = serde_json::json!({
			"template_string": "<h1>{{ title }}</h1>",
			"title": "Hello World"
		});

		b.iter(|| {
			rt.block_on(async { black_box(renderer.render_template(&context).await) })
		});
	});

	// Conditional rendering
	c.bench_function("template_html_renderer_conditional", |b| {
		let renderer = TemplateHTMLRenderer::new();
		let context = serde_json::json!({
			"template_string": r#"<div>{% if age >= 18 %}Adult{% else %}Minor{% endif %}</div>"#,
			"age": 25
		});

		b.iter(|| {
			rt.block_on(async { black_box(renderer.render_template(&context).await) })
		});
	});

	// List rendering
	c.bench_function("template_html_renderer_list", |b| {
		let renderer = TemplateHTMLRenderer::new();
		let items: Vec<&str> = (0..10).map(|_| "item").collect();
		let context = serde_json::json!({
			"template_string": "<ul>{% for item in items %}<li>{{ item }}</li>{% endfor %}</ul>",
			"items": items
		});

		b.iter(|| {
			rt.block_on(async { black_box(renderer.render_template(&context).await) })
		});
	});
}

fn benchmark_tera_vs_renderer(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	let template = "<h1>{{ title }}</h1><p>{{ content }}</p>";

	// Direct Tera
	c.bench_function("compare_direct_tera", |b| {
		let mut context = Context::new();
		context.insert("title", "Test Title");
		context.insert("content", "Test content here");

		b.iter(|| black_box(Tera::one_off(template, &context, true)));
	});

	// TemplateHTMLRenderer (Tera internal)
	c.bench_function("compare_template_html_renderer", |b| {
		let renderer = TemplateHTMLRenderer::new();
		let context = serde_json::json!({
			"template_string": template,
			"title": "Test Title",
			"content": "Test content here"
		});

		b.iter(|| {
			rt.block_on(async { black_box(renderer.render_template(&context).await) })
		});
	});
}

criterion_group!(
	benches,
	benchmark_fs_loader,
	benchmark_tera_rendering,
	benchmark_template_html_renderer,
	benchmark_tera_vs_renderer
);
criterion_main!(benches);
