#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::router::RouterOutlet;
use reinhardt_urls::routers::ClientRouter;

fn page_with_text(text: &'static str) -> Page {
	PageElement::new("main").child(text).into_page()
}

#[test]
fn test_router_outlet_renders_current_client_route() {
	let router = ClientRouter::new()
		.route("home", "/", || page_with_text("Home"))
		.route("todos", "/todos/", || page_with_text("Todos"));

	router
		.push("/todos/")
		.expect("route navigation should succeed");

	let html = RouterOutlet::new(router).into_page().render_to_string();

	assert_eq!(html, "<main>Todos</main>");
}
