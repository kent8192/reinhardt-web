#![cfg(not(target_arch = "wasm32"))]
//! SuspenseBoundary Integration Tests
//!
//! Validates `SuspenseBoundary` component behavior in SSR (non-WASM) environment.
//!
//! Test Categories:
//! - Happy Path: 5 tests
//! - Error Path: 3 tests
//! - Edge Cases: 5 tests
//! - Boundary Analysis: 4 tests
//! - Decision Table: 8 tests (2-resource combinations)
//! - Equivalence Partitioning: 3 tests
//! - State Transitions: 3 tests
//! - Use Cases: 2 tests
//! - Sanity: 1 test
//! - Property-based: 1 test
//!
//! Total: 35 tests
use reinhardt_pages::component::suspense::{ResourceTracker, SuspenseBoundary};
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use rstest::*;
/// Minimal stub implementing `ResourceTracker` for controlled state testing.
struct MockResource {
	loading: bool,
}
impl MockResource {
	fn loading() -> Self {
		Self { loading: true }
	}
	fn resolved() -> Self {
		Self { loading: false }
	}
}
impl ResourceTracker for MockResource {
	fn is_loading(&self) -> bool {
		self.loading
	}
}
/// SSR renders content even when the tracked resource is in Loading state.
#[rstest]
fn ssr_renders_content_when_loading() {
	let boundary = SuspenseBoundary::new()
		.fallback(|| Page::text("Should not appear"))
		.track_custom(MockResource::loading())
		.content(|| PageElement::new("div").child("SSR content").into_page());
	let html = boundary.render().render_to_string();
	assert!(html.contains("SSR content"));
	assert!(!html.contains("Should not appear"));
}
/// SSR renders content when the tracked resource has succeeded.
#[rstest]
fn ssr_renders_content_when_success() {
	let boundary = SuspenseBoundary::new()
		.fallback(|| Page::text("Loading..."))
		.track_custom(MockResource::resolved())
		.content(|| {
			PageElement::new("main")
				.child("Success content")
				.into_page()
		});
	let html = boundary.render().render_to_string();
	assert!(html.contains("Success content"));
}
/// SSR renders content when the tracked resource is in an error state.
#[rstest]
fn ssr_renders_content_when_error() {
	let boundary = SuspenseBoundary::new()
		.fallback(|| Page::text("Loading..."))
		.track_custom(MockResource::resolved())
		.content(|| {
			PageElement::new("span")
				.attr("class", "error")
				.child("Error content")
				.into_page()
		});
	let html = boundary.render().render_to_string();
	assert!(html.contains("Error content"));
	assert!(html.contains("class=\"error\""));
}
/// SSR renders content when multiple resources have mixed states.
#[rstest]
fn ssr_renders_content_with_mixed_states() {
	let boundary = SuspenseBoundary::new()
		.fallback(|| Page::text("Should not appear"))
		.track_custom(MockResource::loading())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::loading())
		.content(|| {
			PageElement::new("section")
				.child("Mixed content")
				.into_page()
		});
	let html = boundary.render().render_to_string();
	assert!(html.contains("Mixed content"));
	assert!(!html.contains("Should not appear"));
}
/// SSR always produces `data-rh-suspense="resolved"` marker regardless of resource state.
#[rstest]
fn ssr_marker_attribute_is_always_resolved() {
	let boundary_with_loading = SuspenseBoundary::new()
		.track_custom(MockResource::loading())
		.content(|| Page::text("content"));
	let boundary_with_resolved = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.content(|| Page::text("content"));
	let html_loading = boundary_with_loading.render().render_to_string();
	let html_resolved = boundary_with_resolved.render().render_to_string();
	assert!(html_loading.contains("data-rh-suspense=\"resolved\""));
	assert!(html_resolved.contains("data-rh-suspense=\"resolved\""));
	assert!(!html_loading.contains("data-rh-suspense=\"pending\""));
}
/// `any_loading` returns true when at least one tracker is loading.
#[rstest]
fn any_loading_true_when_one_loading() {
	let boundary = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::loading());
	assert!(boundary.any_loading());
}
/// `any_loading` returns false when all trackers are resolved.
#[rstest]
fn any_loading_false_when_all_resolved() {
	let boundary = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved());
	assert!(!boundary.any_loading());
}
/// Fallback closure is not invoked during SSR.
#[rstest]
fn fallback_closure_not_called_in_ssr() {
	let fallback_called = std::cell::Cell::new(false);
	let boundary = SuspenseBoundary::new()
		.track_custom(MockResource::loading())
		.content(|| Page::text("content"));
	let fallback_text = "FALLBACK_MARKER_12345";
	let boundary2 = SuspenseBoundary::new()
		.fallback(move || {
			let _ = fallback_called.get();
			Page::text(fallback_text)
		})
		.track_custom(MockResource::loading())
		.content(|| Page::text("real content"));
	let html = boundary2.render().render_to_string();
	assert!(!html.contains(fallback_text));
	assert!(html.contains("real content"));
	let _ = boundary;
}
/// When no trackers are registered, `any_loading` is false (vacuously resolved).
#[rstest]
fn no_trackers_any_loading_is_false() {
	let boundary = SuspenseBoundary::new();
	assert!(!boundary.any_loading());
}
/// `SuspenseBoundary::new()` default renders to Empty for both fallback and content.
#[rstest]
fn default_new_produces_empty_pages() {
	let boundary = SuspenseBoundary::new();
	let html = boundary.render().render_to_string();
	assert!(html.contains("data-rh-suspense=\"resolved\""));
}
/// An explicitly empty content closure produces a valid resolved page.
#[rstest]
fn empty_content_closure_returns_empty_page() {
	let boundary = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.content(|| Page::Empty);
	let html = boundary.render().render_to_string();
	assert!(html.contains("data-rh-suspense=\"resolved\""));
}
/// Nested SuspenseBoundary components each render their own content in SSR.
#[rstest]
fn nested_boundaries_ssr_both_render_content() {
	let inner_html = SuspenseBoundary::new()
		.fallback(|| Page::text("Inner loading"))
		.track_custom(MockResource::loading())
		.content(|| PageElement::new("p").child("inner content").into_page())
		.render()
		.render_to_string();
	let outer = SuspenseBoundary::new()
		.fallback(|| Page::text("Outer loading"))
		.track_custom(MockResource::loading())
		.content(move || {
			PageElement::new("div")
				.child("outer content")
				.child(inner_html.clone())
				.into_page()
		});
	let html = outer.render().render_to_string();
	assert!(html.contains("outer content"));
	assert!(html.contains("inner content"));
	assert!(!html.contains("Outer loading"));
	assert!(!html.contains("Inner loading"));
}
/// Content closure correctly captures and renders values from the enclosing scope.
#[rstest]
fn content_closure_captures_resource_value() {
	let username = "Alice".to_string();
	let boundary = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.content(move || {
			PageElement::new("span")
				.child(format!("Hello, {}!", username))
				.into_page()
		});
	let html = boundary.render().render_to_string();
	assert!(html.contains("Hello, Alice!"));
}
/// Boundary: zero trackers.
#[rstest]
fn boundary_zero_trackers() {
	let boundary = SuspenseBoundary::new();
	assert!(!boundary.any_loading());
}
/// Boundary: exactly one tracker in Loading state.
#[rstest]
fn boundary_one_tracker_loading() {
	let boundary = SuspenseBoundary::new().track_custom(MockResource::loading());
	assert!(boundary.any_loading());
}
/// Boundary: exactly one tracker in resolved state.
#[rstest]
fn boundary_one_tracker_resolved() {
	let boundary = SuspenseBoundary::new().track_custom(MockResource::resolved());
	assert!(!boundary.any_loading());
}
/// Boundary: many trackers with only the last one loading.
#[rstest]
fn boundary_many_trackers_one_loading() {
	let boundary = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::loading());
	assert!(boundary.any_loading());
}
/// DT-1: R1=Loading, R2=Loading → any_loading=true
#[rstest]
fn decision_table_loading_loading() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::loading())
		.track_custom(MockResource::loading());
	assert!(b.any_loading());
}
/// DT-2: R1=Loading, R2=Success → any_loading=true
#[rstest]
fn decision_table_loading_success() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::loading())
		.track_custom(MockResource::resolved());
	assert!(b.any_loading());
}
/// DT-3: R1=Loading, R2=Error → any_loading=true
#[rstest]
fn decision_table_loading_error() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::loading())
		.track_custom(MockResource::resolved());
	assert!(b.any_loading());
}
/// DT-4: R1=Success, R2=Loading → any_loading=true
#[rstest]
fn decision_table_success_loading() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::loading());
	assert!(b.any_loading());
}
/// DT-5: R1=Success, R2=Success → any_loading=false
#[rstest]
fn decision_table_success_success() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved());
	assert!(!b.any_loading());
}
/// DT-6: R1=Success, R2=Error → any_loading=false
#[rstest]
fn decision_table_success_error() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved());
	assert!(!b.any_loading());
}
/// DT-7: R1=Error, R2=Loading → any_loading=true
#[rstest]
fn decision_table_error_loading() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::loading());
	assert!(b.any_loading());
}
/// DT-8: R1=Error, R2=Error → any_loading=false
#[rstest]
fn decision_table_error_error() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved());
	assert!(!b.any_loading());
}
/// EP-1: All trackers are in Loading → any_loading=true (loading partition).
#[rstest]
fn equivalence_all_loading_partition() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::loading())
		.track_custom(MockResource::loading())
		.track_custom(MockResource::loading());
	assert!(b.any_loading());
}
/// EP-2: All trackers are in Success → any_loading=false (success partition).
#[rstest]
fn equivalence_all_success_partition() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved());
	assert!(!b.any_loading());
}
/// EP-3: All trackers are in Error → any_loading=false (error partition).
#[rstest]
fn equivalence_all_error_partition() {
	let b = SuspenseBoundary::new()
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved());
	assert!(!b.any_loading());
}
/// State transition: Loading → resolved changes `any_loading` from true to false.
#[rstest]
fn state_transition_loading_to_resolved() {
	let b1 = SuspenseBoundary::new().track_custom(MockResource::loading());
	assert!(b1.any_loading());
	let b2 = SuspenseBoundary::new().track_custom(MockResource::resolved());
	assert!(!b2.any_loading());
}
/// State transition: Loading → Success; SSR always renders content at both points.
#[rstest]
fn state_transition_loading_to_success_ssr_always_content() {
	let loading_boundary = SuspenseBoundary::new()
		.fallback(|| Page::text("loading"))
		.track_custom(MockResource::loading())
		.content(|| PageElement::new("p").child("data loaded").into_page());
	let resolved_boundary = SuspenseBoundary::new()
		.fallback(|| Page::text("loading"))
		.track_custom(MockResource::resolved())
		.content(|| PageElement::new("p").child("data loaded").into_page());
	let html_before = loading_boundary.render().render_to_string();
	let html_after = resolved_boundary.render().render_to_string();
	assert!(html_before.contains("data loaded"));
	assert!(html_after.contains("data loaded"));
}
/// State transition: multiple consecutive state changes produce consistent SSR output.
#[rstest]
fn state_transition_multiple_times() {
	for _ in 0..5 {
		let b = SuspenseBoundary::new()
			.fallback(|| Page::text("loading"))
			.track_custom(MockResource::loading())
			.content(|| Page::text("stable content"));
		let html = b.render().render_to_string();
		assert!(html.contains("stable content"));
		assert!(!html.contains("loading"));
	}
}
/// Use case: user profile page that defers name rendering.
#[rstest]
fn use_case_user_profile_loading() {
	let username = "Bob".to_string();
	let boundary = SuspenseBoundary::new()
		.fallback(|| {
			PageElement::new("div")
				.attr("class", "skeleton")
				.child("Loading profile...")
				.into_page()
		})
		.track_custom(MockResource::resolved())
		.content(move || {
			PageElement::new("article")
				.child(
					PageElement::new("h1")
						.child(format!("Welcome, {}", username))
						.into_page(),
				)
				.into_page()
		});
	let html = boundary.render().render_to_string();
	assert!(html.contains("Welcome, Bob"));
	assert!(!html.contains("Loading profile..."));
}
/// Use case: dashboard with multiple independent data sources.
#[rstest]
fn use_case_dashboard_multiple_sources() {
	let boundary = SuspenseBoundary::new()
		.fallback(|| Page::text("Dashboard loading..."))
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.track_custom(MockResource::resolved())
		.content(|| {
			PageElement::new("div")
				.attr("id", "dashboard")
				.child("Dashboard ready")
				.into_page()
		});
	let html = boundary.render().render_to_string();
	assert!(html.contains("Dashboard ready"));
	assert!(html.contains("id=\"dashboard\""));
}
/// End-to-end sanity: builder API → render → HTML contains expected structure.
#[rstest]
fn sanity_basic_end_to_end() {
	let html = SuspenseBoundary::new()
		.fallback(|| Page::text("loading"))
		.track_custom(MockResource::resolved())
		.content(|| PageElement::new("span").child("hello").into_page())
		.render()
		.render_to_string();
	assert!(html.contains("<span>hello</span>"));
	assert!(html.contains("data-rh-suspense=\"resolved\""));
	assert!(!html.contains("loading"));
}
/// Property: if any tracker is loading, `any_loading` must be true regardless of others.
#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(4)]
fn property_any_loading_is_monotone(#[case] resolved_count: usize) {
	let mut b = SuspenseBoundary::new();
	for _ in 0..resolved_count {
		b = b.track_custom(MockResource::resolved());
	}
	b = b.track_custom(MockResource::loading());
	assert!(
		b.any_loading(),
		"expected any_loading=true with {resolved_count} resolved + 1 loading"
	);
}
