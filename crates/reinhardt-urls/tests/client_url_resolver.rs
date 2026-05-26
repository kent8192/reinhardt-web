#![cfg(feature = "client-router")]
//! Integration tests for client-side URL resolution.
//!
//! After the URL routing simplification (Issue #4784), client URL reversal
//! uses `ClientRouter::reverse()` directly instead of a separate
//! `ClientUrlReverser` struct.

use rstest::*;

use reinhardt_core::page::Page;
use reinhardt_urls::routers::client_router::ClientRouter;
use reinhardt_urls::routers::resolver::ClientUrlResolver;

/// Stub page component for route registration.
fn stub_page() -> Page {
	Page::Empty
}

#[rstest]
fn test_client_url_reverser_round_trip_via_client_router() {
	// Arrange
	let router = ClientRouter::new()
		.route("app:home", "/", stub_page)
		.route("app:user_detail", "/users/{id}/", stub_page)
		.route(
			"app:user_posts",
			"/users/{user_id}/posts/{post_id}/",
			stub_page,
		);

	// Act & Assert
	assert_eq!(router.reverse("app:home", &[]), Some("/".to_string()));
	assert_eq!(
		router.reverse("app:user_detail", &[("id", "42")]),
		Some("/users/42/".to_string())
	);
	assert_eq!(
		router.reverse("app:user_posts", &[("user_id", "5"), ("post_id", "10")]),
		Some("/users/5/posts/10/".to_string())
	);
	assert_eq!(router.reverse("nonexistent", &[]), None);
}

/// Test that ClientUrlResolver trait works with ClientRouter
#[rstest]
fn test_client_url_resolver_trait_impl() {
	// Arrange
	let router = ClientRouter::new()
		.route("myapp:index", "/", stub_page)
		.route("myapp:detail", "/items/{id}/", stub_page);

	// Create a simple resolver wrapper
	struct TestResolver {
		router: ClientRouter,
	}

	impl ClientUrlResolver for TestResolver {
		fn resolve_client_url(&self, name: &str, params: &[(&str, &str)]) -> String {
			self.router
				.reverse(name, params)
				.unwrap_or_else(|| panic!("Route '{}' not found", name))
		}
	}

	let resolver = TestResolver { router };

	// Act & Assert
	assert_eq!(resolver.resolve_client_url("myapp:index", &[]), "/");
	assert_eq!(
		resolver.resolve_client_url("myapp:detail", &[("id", "99")]),
		"/items/99/"
	);
}

#[rstest]
#[should_panic(expected = "not found")]
fn test_client_url_resolver_panics_on_unknown_route() {
	// Arrange
	let router = ClientRouter::new();

	struct TestResolver {
		router: ClientRouter,
	}

	impl ClientUrlResolver for TestResolver {
		fn resolve_client_url(&self, name: &str, params: &[(&str, &str)]) -> String {
			self.router
				.reverse(name, params)
				.unwrap_or_else(|| panic!("Route '{}' not found", name))
		}
	}

	let resolver = TestResolver { router };

	// Act -- should panic
	resolver.resolve_client_url("nonexistent", &[]);
}
