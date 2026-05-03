#![cfg(not(target_arch = "wasm32"))]
//! Integration tests for Client-Side Router
//!
//! These tests verify the routing system functionality:
//! 1. Path pattern matching with parameters
//! 2. Named routes and reverse URL lookup
//! 3. Navigation and signal updates
//! 4. Router components (Link, RouterOutlet, Redirect)

use reinhardt_pages::component::{Component, Page};
use reinhardt_pages::router::{Link, PathPattern, Redirect, Router, RouterOutlet, guard, guard_or};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

fn home_view() -> Page {
	Page::text("Home")
}

fn users_view() -> Page {
	Page::text("Users")
}

fn user_detail_view() -> Page {
	Page::text("User Detail")
}

fn admin_view() -> Page {
	Page::text("Admin")
}

fn not_found_view() -> Page {
	Page::text("404 Not Found")
}

/// Success Criterion 1: Path pattern matching
#[test]
fn test_path_pattern_exact_match() {
	let pattern = PathPattern::new("/users/");

	assert!(pattern.matches("/users/").is_some());
	assert!(pattern.matches("/users").is_none()); // No trailing slash
	assert!(pattern.matches("/posts/").is_none());
}

/// Success Criterion 1: Path pattern with parameters
#[test]
fn test_path_pattern_with_params() {
	let pattern = PathPattern::new("/users/{id}/");

	let result = pattern.matches("/users/42/");
	assert!(result.is_some());

	let (params, _param_values) = result.unwrap();
	assert_eq!(params.get("id"), Some(&"42".to_string()));
}

/// Success Criterion 1: Path pattern with multiple parameters
#[test]
fn test_path_pattern_multiple_params() {
	let pattern = PathPattern::new("/users/{user_id}/posts/{post_id}/");

	let result = pattern.matches("/users/1/posts/99/");
	assert!(result.is_some());

	let (params, _param_values) = result.unwrap();
	assert_eq!(params.get("user_id"), Some(&"1".to_string()));
	assert_eq!(params.get("post_id"), Some(&"99".to_string()));
}

/// Success Criterion 1: Reverse URL from pattern
#[test]
fn test_path_pattern_reverse() {
	let pattern = PathPattern::new("/users/{id}/");

	let mut params = HashMap::new();
	params.insert("id".to_string(), "42".to_string());

	let url = pattern.reverse(&params);
	assert_eq!(url, Some("/users/42/".to_string()));
}

/// Success Criterion 2: Router with named routes
#[test]
#[serial(router)]
fn test_router_named_routes() {
	let router = Router::new()
		.named_route("home", "/", home_view)
		.named_route("users", "/users/", users_view)
		.named_route("user_detail", "/users/{id}/", user_detail_view);

	assert!(router.has_route("home"));
	assert!(router.has_route("users"));
	assert!(router.has_route("user_detail"));
	assert!(!router.has_route("nonexistent"));
}

/// Success Criterion 2: Reverse URL lookup by name
#[test]
#[serial(router)]
fn test_router_reverse_url() {
	let router = Router::new()
		.named_route("home", "/", home_view)
		.named_route("user_detail", "/users/{id}/", user_detail_view);

	// Reverse without params
	let url = router.reverse("home", &[]);
	assert_eq!(url.unwrap(), "/");

	// Reverse with params
	let url = router.reverse("user_detail", &[("id", "42")]);
	assert_eq!(url.unwrap(), "/users/42/");
}

/// Success Criterion 2: Reverse URL with invalid name
#[test]
#[serial(router)]
fn test_router_reverse_invalid_name() {
	let router = Router::new().named_route("home", "/", home_view);

	let result = router.reverse("nonexistent", &[]);
	assert!(result.is_err());
}

/// Success Criterion 3: Router path matching
#[test]
#[serial(router)]
fn test_router_match_path() {
	let router = Router::new()
		.route("/", home_view)
		.route("/users/", users_view)
		.route("/users/{id}/", user_detail_view);

	// Match exact paths
	assert!(router.match_path("/").is_some());
	assert!(router.match_path("/users/").is_some());

	// Match with parameters
	let route_match = router.match_path("/users/42/");
	assert!(route_match.is_some());

	let rm = route_match.unwrap();
	assert_eq!(rm.params.get("id"), Some(&"42".to_string()));

	// No match
	assert!(router.match_path("/nonexistent/").is_none());
}

/// Success Criterion 3: Router with guards
#[test]
#[serial(router)]
fn test_router_with_guards() {
	// Guard that always rejects
	let router = Router::new()
		.guarded_route("/admin/", admin_view, |_| false)
		.route("/public/", home_view);

	// Guard rejects
	assert!(router.match_path("/admin/").is_none());

	// No guard, matches
	assert!(router.match_path("/public/").is_some());
}

/// Success Criterion 3: Router not found handler
#[test]
#[serial(router)]
fn test_router_not_found() {
	let router = Router::new()
		.route("/", home_view)
		.not_found(not_found_view);

	let view = router.render_current();
	let html = view.render_to_string();

	// Initial path is "/" which matches home
	assert_eq!(html, "Home");
}

/// Success Criterion 4: Link component rendering
#[test]
fn test_link_component() {
	let link = Link::new("/users/", "View Users")
		.class("nav-link")
		.attr("data-test", "link");

	let html = link.render().render_to_string();

	assert!(html.contains("href=\"/users/\""));
	assert!(html.contains("class=\"nav-link\""));
	assert!(html.contains("data-test=\"link\""));
	assert!(html.contains("View Users"));
	assert!(html.contains("data-link=\"true\"")); // SPA marker
}

/// Success Criterion 4: Link component with replace
#[test]
fn test_link_with_replace() {
	let link = Link::new("/dashboard/", "Dashboard").replace(true);

	let html = link.render().render_to_string();

	assert!(html.contains("data-replace=\"true\""));
}

/// Success Criterion 4: External link
#[test]
fn test_link_external() {
	let link = Link::new("https://example.com", "External").external(true);

	let html = link.render().render_to_string();

	assert!(html.contains("target=\"_blank\""));
	assert!(html.contains("rel=\"noopener noreferrer\""));
	assert!(!html.contains("data-link")); // No SPA marker
}

/// Success Criterion 4: RouterOutlet component
#[test]
fn test_router_outlet() {
	use std::sync::Arc;
	let router = Arc::new(Router::new());
	let outlet = RouterOutlet::new(router)
		.id("main-content")
		.class("content-area");

	let html = outlet.render().render_to_string();

	assert!(html.contains("data-router-outlet=\"true\""));
	assert!(html.contains("id=\"main-content\""));
	assert!(html.contains("class=\"content-area\""));
}

/// Success Criterion 4: Redirect component
#[test]
fn test_redirect_component() {
	let redirect = Redirect::new("/login/");

	let html = redirect.render().render_to_string();

	assert!(html.contains("url=/login/"));
	assert!(html.contains("data-redirect=\"/login/\""));
	assert!(html.contains("data-replace=\"true\"")); // Default is replace
}

/// Success Criterion 4: Guard function
#[test]
fn test_guard_true() {
	let guarded = guard(|| true, "Allowed Content");
	let view = guarded();

	assert_eq!(view.render_to_string(), "Allowed Content");
}

/// Success Criterion 4: Guard function with false condition
#[test]
fn test_guard_false() {
	let guarded = guard(|| false, "Allowed Content");
	let view = guarded();

	assert_eq!(view.render_to_string(), ""); // Empty
}

/// Success Criterion 4: Guard with fallback
#[test]
fn test_guard_or_fallback() {
	let guarded = guard_or(|| false, "Main Content", "Fallback Content");
	let view = guarded();

	assert_eq!(view.render_to_string(), "Fallback Content");
}

/// Integration test: Full routing scenario
#[test]
#[serial(router)]
fn test_full_routing_scenario() {
	// 1. Create router with various routes
	let router = Router::new()
		.named_route("home", "/", home_view)
		.named_route("users", "/users/", users_view)
		.named_route("user_detail", "/users/{id}/", user_detail_view)
		.guarded_route("/admin/", admin_view, |_| true) // Admin with guard that passes
		.not_found(not_found_view);

	// 2. Test route matching
	assert_eq!(router.route_count(), 4);

	// 3. Match and extract params
	let user_match = router.match_path("/users/123/").unwrap();
	assert_eq!(user_match.params.get("id"), Some(&"123".to_string()));

	// 4. Reverse URL
	let url = router.reverse("user_detail", &[("id", "456")]).unwrap();
	assert_eq!(url, "/users/456/");

	// 5. Generate Link component
	let link = Link::new(&url, "View User 456");
	let html = link.render().render_to_string();
	assert!(html.contains("href=\"/users/456/\""));
}

/// Integration test: Router navigation (non-WASM)
#[test]
#[serial(router)]
fn test_router_navigation() {
	let router = Router::new()
		.named_route("home", "/", home_view)
		.named_route("users", "/users/", users_view);

	// Push navigation
	assert!(router.push("/users/").is_ok());

	// Replace navigation
	assert!(router.replace("/").is_ok());
}

/// Test pattern param names extraction
#[test]
fn test_path_pattern_param_names() {
	let pattern = PathPattern::new("/users/{user_id}/posts/{post_id}/comments/{comment_id}/");

	let result = pattern.matches("/users/1/posts/2/comments/3/");
	assert!(result.is_some());

	let (params, _param_values) = result.unwrap();
	assert_eq!(params.len(), 3);
	assert_eq!(params.get("user_id"), Some(&"1".to_string()));
	assert_eq!(params.get("post_id"), Some(&"2".to_string()));
	assert_eq!(params.get("comment_id"), Some(&"3".to_string()));
}

/// Test router route count
#[test]
#[serial(router)]
fn test_router_route_count() {
	let router = Router::new()
		.route("/a/", home_view)
		.route("/b/", home_view)
		.route("/c/", home_view);

	assert_eq!(router.route_count(), 3);
}

/// Test component names
#[test]
fn test_component_names() {
	assert_eq!(Link::name(), "Link");
	assert_eq!(RouterOutlet::name(), "RouterOutlet");
	assert_eq!(Redirect::name(), "Redirect");
}

/// Test 4 in spec for #4101: on_path_pattern callbacks must fire only
/// on transitions where the matched params differ from the previous
/// match (or the pattern transitions between matched and unmatched).
///
/// This test exercises the diff-detection logic via Router::on_navigate
/// directly (the launcher's on_path / on_path_pattern infrastructure
/// internally wraps the same logic). It locks in expected behaviour
/// before the on_path migration in PR #4101 swaps the underlying
/// reactive Effect for an on_navigate listener.
///
/// `PathPattern` and `Router` are re-exported at
/// `reinhardt_pages::router`. `Router::push` works on native targets
/// because `super::history::push_state` has a `#[cfg(native)]` no-op
/// stub.
///
/// Refs #4101.
#[rstest]
fn on_path_pattern_fires_on_param_diff_only() {
	use std::cell::RefCell;
	use std::rc::Rc;

	// Arrange
	let observed: Rc<RefCell<Vec<HashMap<String, String>>>> = Rc::new(RefCell::new(Vec::new()));
	let observed_inner = observed.clone();

	let router = Router::new()
		.route("/users/{id}/", || Page::text("user"))
		.route("/about/", || Page::text("about"));

	// Subscribe via on_navigate emulating on_path_pattern's diff
	// logic. (The launcher uses on_navigate listeners that wrap this
	// same logic post-Task-2.6.)
	let pattern = PathPattern::new("/users/{id}/");
	let last_params: Rc<RefCell<Option<HashMap<String, String>>>> = Rc::new(RefCell::new(None));
	let last_params_inner = last_params.clone();
	let _sub = router.on_navigate(move |path, _params| {
		let new_match = pattern.matches(path).map(|(p, _)| p);
		let should_fire = {
			let mut prev = last_params_inner.borrow_mut();
			let fire = match (&*prev, &new_match) {
				(None, Some(_)) => true,
				(Some(_), None) => false,
				(Some(a), Some(b)) => a != b,
				(None, None) => false,
			};
			*prev = new_match.clone();
			fire
		};
		if should_fire && let Some(params) = new_match {
			observed_inner.borrow_mut().push(params);
		}
	});

	// Act
	router.push("/users/1/").expect("push /users/1/");
	router.push("/users/1/").expect("push /users/1/ again");
	router.push("/users/2/").expect("push /users/2/");
	router.push("/about/").expect("push /about/");
	router
		.push("/users/2/")
		.expect("push /users/2/ after /about/");

	// Assert
	let calls = observed.borrow();
	assert_eq!(
		calls.len(),
		3,
		"callback should fire on transitions 1, 3, 5 only; got: {:?}",
		calls
	);
	assert_eq!(calls[0].get("id").map(String::as_str), Some("1"));
	assert_eq!(calls[1].get("id").map(String::as_str), Some("2"));
	assert_eq!(calls[2].get("id").map(String::as_str), Some("2"));

	// _sub is held until end of scope so the listener fires for every push.
}
