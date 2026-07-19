#![cfg(not(target_arch = "wasm32"))]

use reinhardt_core::page::{Head, IntoPage, Page, PageElement};
use reinhardt_urls::routers::{ClientRouter, RouteLoaderId, RouteMetadata};

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

#[test]
fn route_metadata_composes_full_head_in_builder_order() {
	let metadata = RouteMetadata::new()
		.with_head(Head::new().meta_description("root").title("Root"))
		.with_title("Leaf")
		.with_head(Head::new().canonical("https://example.test/leaf"));

	assert_eq!(metadata.title(), Some("Leaf"));
	assert_eq!(metadata.head().meta_tags.len(), 1);
	assert_eq!(metadata.head().links.len(), 1);
}

#[test]
fn route_loader_id_preserves_stable_value() {
	const ID: RouteLoaderId = RouteLoaderId::new("module::loader");

	assert_eq!(ID.as_str(), "module::loader");
}
