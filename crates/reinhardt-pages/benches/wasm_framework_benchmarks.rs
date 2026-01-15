//! WASM Framework Benchmarks
//!
//! Comprehensive performance benchmarks for the reinhardt-pages framework.
//!
//! Categories:
//! 1. Reactive System (7 benchmarks)
//! 2. SSR Rendering (7 benchmarks)
//! 3. Routing (4 benchmarks)
//! Total: 18 benchmarks

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use reinhardt_pages::component::{Component, IntoPage, Page, PageElement};
use reinhardt_pages::reactive::{Effect, Memo, Signal};
use reinhardt_pages::router::Router;
use reinhardt_pages::ssr::{SsrOptions, SsrRenderer};

// ============================================================================
// Category 1: Reactive System Benchmarks (7 benchmarks)
// ============================================================================

/// Benchmark: Signal creation
fn bench_signal_new(c: &mut Criterion) {
	c.bench_function("signal_new", |b| b.iter(|| Signal::new(black_box(42))));
}

/// Benchmark: Signal read (get)
fn bench_signal_get(c: &mut Criterion) {
	let signal = Signal::new(42);
	c.bench_function("signal_get", |b| b.iter(|| signal.get()));
}

/// Benchmark: Signal write (set)
fn bench_signal_set(c: &mut Criterion) {
	let signal = Signal::new(42);
	c.bench_function("signal_set", |b| b.iter(|| signal.set(black_box(100))));
}

/// Benchmark: Signal update
fn bench_signal_update(c: &mut Criterion) {
	let signal = Signal::new(42);
	c.bench_function("signal_update", |b| b.iter(|| signal.update(|v| *v += 1)));
}

/// Benchmark: Effect creation and execution
fn bench_effect_execution(c: &mut Criterion) {
	let signal = Signal::new(0);
	c.bench_function("effect_execution", |b| {
		b.iter(|| {
			let signal_clone = signal.clone();
			let _effect = Effect::new(move || {
				let _value = signal_clone.get();
			});
		})
	});
}

/// Benchmark: Memo cache hit
fn bench_memo_cache_hit(c: &mut Criterion) {
	let signal = Signal::new(42);
	let memo = Memo::new({
		let signal = signal.clone();
		move || signal.get() * 2
	});

	c.bench_function("memo_cache_hit", |b| b.iter(|| memo.get()));
}

/// Benchmark: Large reactive graph
fn bench_reactive_graph_scale(c: &mut Criterion) {
	let mut group = c.benchmark_group("reactive_graph_scale");

	for size in [10, 100, 1000] {
		group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
			b.iter(|| {
				let signals: Vec<Signal<i32>> = (0..size).map(|i| Signal::new(i as i32)).collect();

				// Create effects that depend on signals
				let _effects: Vec<Effect> = signals
					.iter()
					.map(|signal| {
						let signal = signal.clone();
						Effect::new(move || {
							let _value = signal.get();
						})
					})
					.collect();

				// Update all signals
				for (i, signal) in signals.iter().enumerate() {
					signal.set((i + 1) as i32);
				}
			})
		});
	}

	group.finish();
}

// ============================================================================
// Category 2: SSR Rendering Benchmarks (7 benchmarks)
// ============================================================================

struct SimpleComponent {
	message: String,
}

impl Component for SimpleComponent {
	fn render(&self) -> Page {
		PageElement::new("div")
			.attr("class", "simple")
			.child(self.message.clone())
			.into_page()
	}

	fn name() -> &'static str {
		"SimpleComponent"
	}
}

/// Benchmark: Simple component rendering
fn bench_ssr_simple_component(c: &mut Criterion) {
	let component = SimpleComponent {
		message: "Hello, World!".to_string(),
	};
	let mut renderer = SsrRenderer::new();

	c.bench_function("ssr_simple_component", |b| {
		b.iter(|| renderer.render(black_box(&component)))
	});
}

/// Benchmark: Full page rendering
fn bench_ssr_full_page(c: &mut Criterion) {
	let component = SimpleComponent {
		message: "Test".to_string(),
	};
	let options = SsrOptions {
		title: Some("Benchmark Page".to_string()),
		css_links: vec!["/static/style.css".to_string()],
		js_scripts: vec!["/static/app.js".to_string()],
		..SsrOptions::default()
	};
	let mut renderer = SsrRenderer::with_options(options);

	c.bench_function("ssr_full_page", |b| {
		b.iter(|| renderer.render_page(black_box(&component)))
	});
}

struct NestedComponent {
	depth: usize,
	content: String,
}

impl Component for NestedComponent {
	fn render(&self) -> Page {
		if self.depth == 0 {
			PageElement::new("span")
				.child(self.content.clone())
				.into_page()
		} else {
			let nested = NestedComponent {
				depth: self.depth - 1,
				content: self.content.clone(),
			};
			PageElement::new("div")
				.attr("class", "nested")
				.child(nested.render())
				.into_page()
		}
	}

	fn name() -> &'static str {
		"NestedComponent"
	}
}

/// Benchmark: Nested component rendering
fn bench_ssr_nested_components(c: &mut Criterion) {
	let mut group = c.benchmark_group("ssr_nested_components");

	for depth in [5, 10, 20] {
		group.bench_with_input(BenchmarkId::from_parameter(depth), &depth, |b, &depth| {
			let component = NestedComponent {
				depth,
				content: "Nested".to_string(),
			};
			let mut renderer = SsrRenderer::new();
			b.iter(|| renderer.render(black_box(&component)))
		});
	}

	group.finish();
}

struct ListComponent {
	items: Vec<String>,
}

impl Component for ListComponent {
	fn render(&self) -> Page {
		let mut ul = PageElement::new("ul");
		for item in &self.items {
			ul = ul.child(PageElement::new("li").child(item.clone()).into_page());
		}
		ul.into_page()
	}

	fn name() -> &'static str {
		"ListComponent"
	}
}

/// Benchmark: List rendering
fn bench_ssr_list_rendering(c: &mut Criterion) {
	let mut group = c.benchmark_group("ssr_list_rendering");

	for count in [10, 100, 1000] {
		group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
			let component = ListComponent {
				items: (0..count).map(|i| format!("Item {}", i)).collect(),
			};
			let mut renderer = SsrRenderer::new();
			b.iter(|| renderer.render(black_box(&component)))
		});
	}

	group.finish();
}

/// Benchmark: SSR with state script
fn bench_ssr_with_state_script(c: &mut Criterion) {
	let component = SimpleComponent {
		message: "State test".to_string(),
	};
	let options = SsrOptions {
		title: Some("State Test".to_string()),
		..SsrOptions::default()
	};
	let mut renderer = SsrRenderer::with_options(options);

	// Add some state
	renderer.state_mut().add_metadata("user_id", 123);
	renderer.state_mut().add_metadata("username", "testuser");

	c.bench_function("ssr_with_state_script", |b| {
		b.iter(|| renderer.render_page(black_box(&component)))
	});
}

/// Benchmark: HTML minification
fn bench_ssr_minification(c: &mut Criterion) {
	let component = NestedComponent {
		depth: 10,
		content: "Minify test".to_string(),
	};
	let options = SsrOptions::new().minify();
	let mut renderer = SsrRenderer::with_options(options);

	c.bench_function("ssr_minification", |b| {
		b.iter(|| renderer.render_page(black_box(&component)))
	});
}

/// Benchmark: SSR with hydration markers
fn bench_ssr_with_hydration_markers(c: &mut Criterion) {
	let component = SimpleComponent {
		message: "Hydration test".to_string(),
	};
	let mut renderer = SsrRenderer::new();

	c.bench_function("ssr_with_hydration_markers", |b| {
		b.iter(|| renderer.render_with_marker(black_box(&component)))
	});
}

// ============================================================================
// Category 3: Routing Benchmarks (4 benchmarks)
// ============================================================================

/// Benchmark: Path matching with simple routes
fn bench_router_path_matching(c: &mut Criterion) {
	let router = Router::new()
		.route("/users/{id}", || {
			PageElement::new("div").child("User").into_page()
		})
		.route("/posts/{slug}/", || {
			PageElement::new("div").child("Post").into_page()
		})
		.route("/admin/users/{id}/edit", || {
			PageElement::new("div").child("Edit").into_page()
		});

	c.bench_function("router_path_matching", |b| {
		b.iter(|| router.match_path(black_box("/users/123")))
	});
}

/// Benchmark: Complex path matching with many routes
fn bench_router_complex_path_matching(c: &mut Criterion) {
	let mut router = Router::new();

	// Add many routes
	for i in 0..100 {
		router = router.route(&format!("/api/v1/resource{}/{{id}}", i), move || {
			PageElement::new("div")
				.child(format!("Resource {}", i))
				.into_page()
		});
	}

	c.bench_function("router_complex_path_matching", |b| {
		b.iter(|| router.match_path(black_box("/api/v1/resource50/123")))
	});
}

/// Benchmark: Parameter extraction
fn bench_router_parameter_extraction(c: &mut Criterion) {
	let router = Router::new().route(
		"/users/{user_id}/posts/{post_id}/comments/{comment_id}/",
		|| PageElement::new("div").child("Comment").into_page(),
	);

	c.bench_function("router_parameter_extraction", |b| {
		b.iter(|| {
			if let Some(matched) = router.match_path(black_box("/users/42/posts/123/comments/456"))
			{
				let _ = black_box(&matched.params);
			}
		})
	});
}

/// Benchmark: Named route creation
fn bench_router_named_routes(c: &mut Criterion) {
	c.bench_function("router_named_routes", |b| {
		b.iter(|| {
			let _router = Router::new()
				.named_route("user_profile", "/users/{id}/profile", || {
					PageElement::new("div").child("Profile").into_page()
				})
				.named_route("user_posts", "/users/{id}/posts", || {
					PageElement::new("div").child("Posts").into_page()
				})
				.named_route("user_settings", "/users/{id}/settings", || {
					PageElement::new("div").child("Settings").into_page()
				});
		})
	});
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
	reactive_benches,
	bench_signal_new,
	bench_signal_get,
	bench_signal_set,
	bench_signal_update,
	bench_effect_execution,
	bench_memo_cache_hit,
	bench_reactive_graph_scale
);

criterion_group!(
	ssr_benches,
	bench_ssr_simple_component,
	bench_ssr_full_page,
	bench_ssr_nested_components,
	bench_ssr_list_rendering,
	bench_ssr_with_state_script,
	bench_ssr_minification,
	bench_ssr_with_hydration_markers
);

criterion_group!(
	router_benches,
	bench_router_path_matching,
	bench_router_complex_path_matching,
	bench_router_parameter_extraction,
	bench_router_named_routes
);

criterion_main!(reactive_benches, ssr_benches, router_benches);
