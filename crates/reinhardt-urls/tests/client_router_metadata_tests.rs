#![cfg(not(target_arch = "wasm32"))]

use reinhardt_core::page::{IntoPage, Page, PageElement};
use reinhardt_urls::routers::{ClientRouter, RouteMetadata};

fn page_with_text(text: &'static str) -> Page {
	PageElement::new("main").child(text).into_page()
}

#[test]
fn test_client_route_metadata_is_available_from_match() {
	let router = ClientRouter::new()
		.route("todos", "/todos/", || page_with_text("Todos"))
		.with_route_metadata(
			"todos",
			RouteMetadata::new()
				.with_title("Todos")
				.with_requires_auth(true),
		);

	let matched = router.match_path("/todos/").expect("route should match");

	assert_eq!(matched.route.metadata().title(), Some("Todos"));
	assert!(matched.route.metadata().requires_auth());
}
