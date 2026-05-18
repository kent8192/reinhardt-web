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
	// KNOWN-DEFECT NOTE — TRIPLE-SLASH IN VIEWSET-MOUNT COMPOSITION:
	//
	// The example's `routes()` function uses
	// `UnifiedRouter::new().mount("/api/", url_patterns())`. The
	// `url_patterns()` body in turn calls
	// `ServerRouter::new().viewset("/snippets-viewset", ...)`, and the
	// outer `#[url_patterns(InstalledApp::snippets, mode = server)]`
	// applies `with_namespace("snippets")` which prepends another `"/"`
	// to the route's URL pattern. The three slashes compose to
	// `/api///snippets-viewset/`. The framework's own integration test
	// for the typed viewset accessor (see
	// `tests/integration/tests/url_patterns_viewset_typed_integration.rs`)
	// works around this by replacing the default `ServerRouter` directly
	// via `UnifiedRouter::new().server(|_| url_patterns())` so no `mount`
	// indirection is involved.
	//
	// The assertions below pin the *currently observable* URL strings
	// (with the duplicated slashes) so a fix in the framework will
	// surface as a deliberate, reviewable test diff rather than a silent
	// behaviour change. The function-based endpoints above are
	// unaffected because their URL pattern (`/snippets/`) already starts
	// with a leading slash that the namespace wrapper does not duplicate
	// — the duplication is specific to the `.viewset()`-internal mount
	// path. A follow-up framework Issue tracks the fix.
	// ------------------------------------------------------------------

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_viewset_list() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippet_list();

		// Assert: currently observed value — see the "TRIPLE-SLASH"
		// note above for the framework-side root cause and tracking.
		assert_eq!(url, "/api///snippets-viewset/");
	}

	#[rstest]
	#[serial(routes_global)]
	fn typed_accessor_resolves_viewset_detail_with_id() {
		// Arrange
		let urls = install_routes_and_resolve();

		// Act
		let url = urls.server().snippets().snippet_detail("42");

		// Assert: currently observed value — see the "TRIPLE-SLASH"
		// note above for the framework-side root cause and tracking.
		assert_eq!(url, "/api///snippets-viewset/42/");
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
		// than a behaviour drift. The viewset URLs reflect the same
		// triple-slash known-defect noted above.
		assert_eq!(urls_demo::snippets_list(&urls), "/api/snippets/");
		assert_eq!(urls_demo::snippets_create(&urls), "/api/snippets/");
		assert_eq!(urls_demo::snippets_retrieve(&urls, 1), "/api/snippets/1/");
		assert_eq!(urls_demo::snippets_update(&urls, 99), "/api/snippets/99/");
		assert_eq!(urls_demo::snippets_delete(&urls, 7), "/api/snippets/7/");
		assert_eq!(urls_demo::viewset_list(&urls), "/api///snippets-viewset/");
		assert_eq!(
			urls_demo::viewset_detail(&urls, 42),
			"/api///snippets-viewset/42/"
		);
	}

	// ------------------------------------------------------------------
	// Deprecated flat surface — kept as a single, isolated, opt-in test.
	//
	// The flat accessors (`urls.snippet_list()`, `urls.snippet_detail("id")`)
	// are emitted ONLY by `#[viewset]` (see
	// `crates/reinhardt-core/macros/src/viewset_macro.rs`), not by the
	// per-route `#[get(name = "...")]` / `#[post(name = "...")]` macros.
	// So `urls.snippets_list()` (plural, function-based) does NOT exist
	// as a flat accessor — only `urls.snippet_list()` (singular, from
	// the `ModelViewSet::new("snippet")` basename) does.
	//
	// The flat surface has been deprecated since `0.1.0-rc.16` and now
	// goes through the namespace-iterating `UrlResolverUnprefixed`
	// fallback. Production example code should NOT use this surface —
	// it's exercised here only to document the migration path and pin
	// the runtime equivalence with the typed accessor, so the
	// deprecation does not silently change observable behaviour before
	// the flat surface is removed (planned for v0.2.0 per Issue #4548).
	// ------------------------------------------------------------------

	#[rstest]
	#[serial(routes_global)]
	#[allow(deprecated)]
	fn deprecated_flat_viewset_accessor_matches_typed_accessor() {
		// Arrange
		let urls = install_routes_and_resolve();
		// Bring the deprecated flat-surface viewset trait methods into
		// scope via the `url_prelude` re-export module emitted by
		// `#[routes]`. The traits `ResolveSnippetList` and
		// `ResolveSnippetDetail` add the methods `snippet_list` /
		// `snippet_detail` on any `UrlResolverUnprefixed` implementor.
		use examples_tutorial_rest::config::urls::url_prelude::*;

		// Act
		let typed = urls.server().snippets().snippet_list();
		let flat = urls.snippet_list();

		// Assert: the two surfaces resolve to the same URL today — the
		// flat one will be removed in v0.2.0. See Issue #4548 §
		// "Deprecation removal milestone".
		assert_eq!(typed, flat);
		assert_eq!(typed, "/api///snippets-viewset/");
	}
}
