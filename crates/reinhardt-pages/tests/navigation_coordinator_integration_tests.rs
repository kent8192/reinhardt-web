#![cfg(not(target_arch = "wasm32"))]

//! Native integration coverage for the pages-owned navigation entry point.

use reinhardt_core::page::Page;
use reinhardt_core::reactive::ReactiveScope;
use reinhardt_pages::app::{
	__clear_spa_router_for_test, __current_path_for_test, __install_client_router_for_test,
};
use reinhardt_pages::reactive::hooks::RouterHandle;
use reinhardt_urls::routers::ClientRouter;

fn router() -> ClientRouter {
	ClientRouter::new()
		.route("home", "/", || Page::text("home"))
		.route("settings", "/settings/", || Page::text("settings"))
}

#[test]
fn router_handle_uses_coordinator_for_synchronous_routes() {
	ReactiveScope::run(|| {
		__install_client_router_for_test(router());

		let handle = RouterHandle;
		handle
			.push("/settings/")
			.expect("a matched route should commit through the coordinator");

		assert_eq!(__current_path_for_test().as_deref(), Some("/settings/"));
		__clear_spa_router_for_test();
	});
}

#[test]
fn router_handle_preserves_path_when_matching_rejects() {
	ReactiveScope::run(|| {
		__install_client_router_for_test(router());

		let error = RouterHandle
			.push("/missing/")
			.expect_err("an unknown route must be rejected before commit");

		assert!(error.to_string().contains("no route matches"));
		assert_eq!(__current_path_for_test().as_deref(), Some("/"));
		__clear_spa_router_for_test();
	});
}
