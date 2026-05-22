//! End-to-end demonstration of the typed `ResolvedUrls` accessor pattern.
//!
//! This file is the executable counterpart of the documentation in
//! `examples_tutorial_rest::urls_demo` (and the README's
//! "URL Resolution: Typed Accessors" section). It registers the example's
//! routes globally — exactly as `cargo run --bin manage -- runserver`
//! would do at startup — then asserts that every route registered in
//! `apps::snippets::urls` resolves to the expected URL via the typed
//! `urls.server().snippets().<route>()` accessor.
//!
//! Refs Issue #4548 (acceptance criterion: "At least one example app
//! under examples/ uses the typed accessor exclusively").
#[cfg(with_reinhardt)]
mod tests {
	use examples_tutorial_rest::config::urls::{ResolvedUrls, routes};
	use examples_tutorial_rest::urls_demo;
	use reinhardt::register_client_reverser;
	use reinhardt::register_router;
	use rstest::rstest;
	use serial_test::serial;
	use std::sync::Once;
	static INSTALL_ROUTES: Once = Once::new();
	fn install_routes_and_resolve() -> ResolvedUrls {
		INSTALL_ROUTES.call_once(|| {
			let (server, client) = routes().into_parts();
			let reverser = client.to_reverser();
			register_router(server);
			register_client_reverser(reverser);
		});
		ResolvedUrls::from_global()
	}
	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_list() {
		let urls = install_routes_and_resolve();
		let url = urls.server().snippets().snippets_list();
		assert_eq!(url, "/api/snippets/");
	}
	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_create() {
		let urls = install_routes_and_resolve();
		let url = urls.server().snippets().snippets_create();
		assert_eq!(url, "/api/snippets/");
	}
	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_retrieve_with_id() {
		let urls = install_routes_and_resolve();
		let url = urls.server().snippets().snippets_retrieve("42");
		assert_eq!(url, "/api/snippets/42/");
	}
	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_update_with_id() {
		let urls = install_routes_and_resolve();
		let url = urls.server().snippets().snippets_update("42");
		assert_eq!(url, "/api/snippets/42/");
	}
	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_delete_with_id() {
		let urls = install_routes_and_resolve();
		let url = urls.server().snippets().snippets_delete("42");
		assert_eq!(url, "/api/snippets/42/");
	}
	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_viewset_list() {
		let urls = install_routes_and_resolve();
		let url = urls.server().snippets().snippet_list();
		assert_eq!(url, "/api/snippets-viewset/");
	}
	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_viewset_detail_with_id() {
		let urls = install_routes_and_resolve();
		let url = urls.server().snippets().snippet_detail("42");
		assert_eq!(url, "/api/snippets-viewset/42/");
	}
	#[rstest]
	#[serial(routes_global)]
	fn urls_demo_helpers_match_typed_accessors() {
		let urls = install_routes_and_resolve();
		assert_eq!(urls_demo::snippets_list(&urls), "/api/snippets/");
		assert_eq!(urls_demo::snippets_create(&urls), "/api/snippets/");
		assert_eq!(urls_demo::snippets_retrieve(&urls, 1), "/api/snippets/1/");
		assert_eq!(urls_demo::snippets_update(&urls, 99), "/api/snippets/99/");
		assert_eq!(urls_demo::snippets_delete(&urls, 7), "/api/snippets/7/");
		assert_eq!(urls_demo::viewset_list(&urls), "/api/snippets-viewset/");
		assert_eq!(
			urls_demo::viewset_detail(&urls, 42),
			"/api/snippets-viewset/42/"
		);
	}
	#[rstest]
	#[serial(routes_global)]
	#[allow(deprecated)]
	fn deprecated_flat_viewset_accessor_matches_typed_accessor() {
		let urls = install_routes_and_resolve();
		use examples_tutorial_rest::config::urls::url_prelude::*;
		let typed = urls.server().snippets().snippet_list();
		let flat = urls.snippet_list();
		assert_eq!(typed, flat);
		assert_eq!(typed, "/api/snippets-viewset/");
	}
}
