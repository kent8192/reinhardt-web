//! Namespace regression coverage for Issue #4507.
//!
//! Locks down the *exact* string output of the typed `ResolvedUrls`
//! accessors that Phase 9 unignored. The companion file
//! `url_patterns_viewset_typed_integration.rs` already asserts the same
//! string equalities, but pins them against the **typed** accessors only.
//! This file complements that by:
//!
//! 1. Pinning each typed accessor against a **hard-coded expected URL**
//!    (no router.reverse() involved), so that a future regression that
//!    silently changes the namespace prefix or path layout breaks here
//!    even if the typed accessor and the underlying reverser drift
//!    together.
//! 2. Living in a separate integration-test binary, which removes any
//!    chance of state-file or registration interference between the
//!    two suites' `installed_apps!` invocations and `#[routes]` linker
//!    markers.
//!
//! Why hard-coded values (no `router.reverse(...)`)? The Phase 5/6
//! `#[viewset]` + `#[action]` → introspection bridge for impl-form
//! action discovery is still partially manual (see
//! `install_routes_and_resolve()` in the companion file). Comparing the
//! typed accessor to its own underlying `router.reverse()` would test a
//! tautology rather than guard against drift. Hard-coding the
//! namespaced strings here lets the assertion fail loud when either
//! side breaks.
//!
//! Refs Issue #4507.

// The Phase 6.2 macro design routes the per-fn / per-impl viewset manifest
// macros through `$crate::__for_each_viewset_*!` (see
// `crates/reinhardt-core/macros/src/url_patterns.rs::build_viewset_meta_forwarder`).
// Those manifests are emitted by another attribute macro (`#[viewset]`) in
// the same crate, so the call hits the
// `macro_expanded_macro_exports_accessed_by_absolute_paths` future-incompat
// lint. Until the lint becomes a hard error (rust-lang/rust#52234), opt
// out at the crate level so user-written code can compose `#[viewset]` +
// `#[url_patterns]` + `#[routes]` in a single binary.
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]

use reinhardt::installed_apps;
use reinhardt_urls::routers::UnifiedRouter;
use rstest::rstest;
use serial_test::serial;

installed_apps! {
	snippets: "snippets",
}

#[allow(dead_code)]
#[reinhardt_macros::model(app_label = "snippets", table_name = "snippets")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snippet {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 255)]
	pub title: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SnippetSerializer;

pub mod apps {
	pub mod snippets {
		pub mod views {
			use reinhardt_http::{Response, ViewResult};
			use reinhardt_macros::{action, viewset};
			use reinhardt_views::viewsets::ModelViewSet;

			#[viewset]
			pub fn viewset()
			-> ModelViewSet<super::super::super::Snippet, super::super::super::SnippetSerializer>
			{
				ModelViewSet::new("snippet")
			}

			pub struct SnippetViewSet;

			#[viewset(basename = "snippet")]
			impl SnippetViewSet {
				#[action(methods = "POST", detail = true, url_name = "highlight")]
				pub async fn highlight(_id: String) -> ViewResult<Response> {
					Ok(Response::ok().with_body(b"highlight".to_vec()))
				}
			}
		}

		pub mod urls {
			use super::views;
			use reinhardt_macros::url_patterns;
			use reinhardt_urls::routers::ServerRouter;
			use std::marker::PhantomData;

			#[url_patterns(crate::InstalledApp::snippets, mode = server)]
			pub fn url_patterns() -> ServerRouter {
				ServerRouter::new().viewset_with_actions(
					"snippets-viewset",
					views::viewset(),
					PhantomData::<views::SnippetViewSet>,
				)
			}

			pub mod ws_urls {
				use reinhardt_macros::url_patterns;
				use reinhardt_urls::routers::UnifiedRouter;

				#[url_patterns(crate::InstalledApp::snippets, mode = ws)]
				pub fn ws_url_patterns() -> UnifiedRouter {
					UnifiedRouter::new()
				}
			}
		}
	}
}

#[reinhardt::routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().server(|_| apps::snippets::urls::url_patterns())
}

fn install_routes_and_resolve() -> crate::ResolvedUrls {
	// Action registration is emitted by the #[viewset(basename = ...)] impl-form
	// expansion (via a `#[ctor]` startup function). No manual seeding needed.
	// Refs Issue #4507 / Phase 5.1.
	let server = routes().into_server();
	reinhardt_urls::routers::register_router(server);
	crate::ResolvedUrls::from_global()
}

// === Tests ===

#[rstest]
#[serial(routes_global)]
fn namespace_pins_list_typed_accessor_string() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let typed = urls.server().snippets().snippet_list();

	// Assert: the namespaced gateway accessor must produce the
	// `<app-path>/<basename>-list` URL. A drift in `#[url_patterns]`'s
	// namespace wrapper or in `register_viewset`'s list-route shape
	// breaks this assertion.
	assert_eq!(typed, "/snippets-viewset/");
}

#[rstest]
#[serial(routes_global)]
fn namespace_pins_detail_typed_accessor_string() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let typed = urls.server().snippets().snippet_detail("42");

	// Assert: the detail accessor must substitute the lookup field (`id`
	// by default) into the namespaced detail-route pattern. A drift in
	// the `<id>` → `{id}` normalisation in `introspection.rs` breaks
	// this assertion.
	assert_eq!(typed, "/snippets-viewset/42/");
}

#[rstest]
#[serial(routes_global)]
fn namespace_pins_action_typed_accessor_string() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let typed = urls.server().snippets().highlight("42");

	// Assert: the `#[action]` accessor must surface under the per-app
	// gateway (`urls.server().<app>().<action>(id)`) and substitute the
	// detail id. Regression for defect #3 in the spec.
	assert_eq!(typed, "/snippets-viewset/42/highlight/");
}
