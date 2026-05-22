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

	// `register_router` and `register_client_reverser` overwrite global
	// `OnceCell` slots, so the second integration-test binary in this
	// crate that touches the global router would race with the first.
	// `tests/integration.rs` does not touch the router at all (it only
	// exercises raw SQL through `sqlx`), so a single-process `Once`
	// guard is sufficient here — every `#[rstest]` below shares the
	// same registered routes.
	//
	// Every test below is additionally marked `#[serial(routes_global)]`
	// so it runs serially with any future test (in this crate or another
	// integration binary in the same `cargo test` invocation) that
	// touches the same global router slots. This matches the project-
	// wide convention in `CLAUDE.md` § Testing for global-state tests
	// and mirrors the framework's own typed-viewset integration test
	// (see `tests/integration/tests/url_patterns_viewset_typed_integration.rs`).
	static INSTALL_ROUTES: Once = Once::new();

	fn install_routes_and_resolve() -> ResolvedUrls {
		INSTALL_ROUTES.call_once(|| {
			// `routes()` returns a `UnifiedRouter` that the framework
			// would normally consume via `register_globally()` during
			// server startup. We do the same thing manually here so
			// `ResolvedUrls::from_global()` finds both halves of the
			// global registration. Splitting `into_parts()` and pushing
			// each half explicitly mirrors what `register_globally()`
			// does internally — the indirection makes the registration
			// flow visible to readers learning the API.
			let (server, client) = routes().into_parts();
			let reverser = client.to_reverser();
			register_router(server);
			register_client_reverser(reverser);
		});
		ResolvedUrls::from_global()
	}

	// ------------------------------------------------------------------
	// Function-based endpoints — Tutorial 1-5
	//
	// Routes registered via `#[get("/snippets/", name = "snippets_list")]`
	// etc. in `apps::snippets::views`. The route names become methods on
	// the `SnippetsUrls<'_>` accessor returned by `urls.server().snippets()`,
	// with the `"snippets:"` namespace added transparently by
	// `#[url_patterns(InstalledApp::snippets, mode = server)]`.
	// ------------------------------------------------------------------

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_list() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippets_list();

		// Assert: the function-based `#[get("/snippets/", name = "snippets_list")]`
		// route is mounted under the project-wide `/api/` prefix.
		assert_eq!(url, "/api/snippets/");
	}

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_create() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippets_create();

		// Assert
		assert_eq!(url, "/api/snippets/");
	}

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_retrieve_with_id() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippets_retrieve("42");

		// Assert
		assert_eq!(url, "/api/snippets/42/");
	}

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_update_with_id() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippets_update("42");

		// Assert
		assert_eq!(url, "/api/snippets/42/");
	}

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_snippets_delete_with_id() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippets_delete("42");

		// Assert
		assert_eq!(url, "/api/snippets/42/");
	}

	// ------------------------------------------------------------------
	// ViewSet endpoints — Tutorial 6
	//
	// `ModelViewSet::new("snippet")` registers `<basename>_list`,
	// `<basename>_detail`, etc. with `basename = "snippet"`. The
	// generated accessors live next to the function-based ones on the
	// same `urls.server().snippets()` gateway.
	//
	// The viewset path composition is symmetric with the function-based
	// endpoints: `mount("/api/", url_patterns())` plants `/api/` on the
	// child router, and `.viewset("/snippets-viewset", _)` registers the
	// viewset under that prefix; the framework's URL-reversal layer joins
	// them through `path_utils::join_prefix_path`, which collapses the
	// trailing-slash-on-prefix + leading-slash-on-path boundary into a
	// single `/`. This wasn't always the case — see Issue #4581 (fixed)
	// for the historical triple-slash (`/api///snippets-viewset/`) bug.
	// ------------------------------------------------------------------

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_viewset_list() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippet_list();

		// Assert: single slash between `/api/` and the viewset prefix.
		assert_eq!(url, "/api/snippets-viewset/");
	}

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_viewset_detail_with_id() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippet_detail("42");

		// Assert: single slash between `/api/` and the viewset prefix.
		assert_eq!(url, "/api/snippets-viewset/42/");
	}

	// ------------------------------------------------------------------
	// The `urls_demo` helper module — wraps the typed accessor calls in
	// thin shims that take `id: i64` and stringify at the call site, the
	// pattern application code is expected to follow.
	// ------------------------------------------------------------------

	#[rstest]
	#[serial(routes_global)]
	fn urls_demo_helpers_match_typed_accessors() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act + Assert: every shim resolves to the same URL the underlying
		// typed accessor would produce. This pins the helper-to-accessor
		// mapping so a renamed route surfaces as a compile error rather
		// than a behaviour drift.
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

	// The deprecated flat `urls.snippet_list()` accessor (emitted by
	// `#[viewset]` until `0.1.0-rc.29`) was removed in 0.2.0 per Issue
	// #4520. The typed accessor demonstrated above is the only
	// supported surface.
}
