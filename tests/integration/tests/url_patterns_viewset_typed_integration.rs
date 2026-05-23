//! End-to-end coverage for Issue #4507.
//!
//! Validates the full pipeline from `#[viewset]` / `#[action]` through
//! `#[url_patterns(..., mode = server)]` to the typed `ResolvedUrls`
//! accessors exposed via `urls.server().<app>().<route>()`.
//!
//! Specifically, the three tests below pin:
//!
//! 1. `viewset_list_typed_accessor_returns_namespaced_url` — `#[viewset]`
//!    fn-form generates a `<basename>_list()` accessor returning the
//!    namespaced URL (with trailing slash).
//! 2. `viewset_detail_typed_accessor_substitutes_id` — the `<basename>_detail`
//!    accessor takes the primary key as an argument and substitutes it
//!    into the URL pattern.
//! 3. `action_typed_accessor_appears_under_app` — `#[viewset]` impl-form
//!    paired with `#[action]` produces a typed accessor under the same
//!    namespaced gateway (regression for defect #3 in the spec).
//!
//! The legacy `urls.<basename>_list()` blanket-trait flat surface was
//! removed in 0.2.0 — see `instructions/MIGRATION_0.2.md` and Issue #4520.
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

// === Snippet model + serializer fixtures ===
//
// `#[model]` brings `Snippet` into the `Model` trait surface that
// `ModelViewSet<Snippet, _>` requires. `SnippetSerializer` is a placeholder
// type — the runtime serializer wiring is exercised by
// `tests/integration/tests/viewsets/model_viewset_crud_e2e.rs`; this file
// targets URL resolution only.

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

// === User app layout: must live under `crate::apps::<app>::urls` so that
// the `#[routes]`-emitted `crate::apps::<app>::urls::url_resolvers::__for_each_url_resolver!`
// invocation resolves. ===

pub mod apps {
	pub mod snippets {
		pub mod views {
			use reinhardt_http::{Response, ViewResult};
			use reinhardt_macros::{action, viewset};
			use reinhardt_views::viewsets::ModelViewSet;

			// Fn-form `#[viewset]`: emits the `snippet_list` / `snippet_detail`
			// typed accessors and the per-fn meta + manifest macros consumed
			// by `__for_each_url_resolver`.
			#[viewset]
			pub fn viewset()
			-> ModelViewSet<super::super::super::Snippet, super::super::super::SnippetSerializer>
			{
				ModelViewSet::new("snippet")
			}

			// Impl-form `#[viewset(basename = "...")]`: surfaces every
			// `#[action]`-decorated method as a typed accessor through the
			// per-impl manifest fan-out.
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

			// `viewset_with_actions` carries the marker type so that the
			// `#[url_patterns]` expansion can splice the impl-form action
			// manifest into the generated `__for_each_url_resolver` arm.
			#[url_patterns(crate::InstalledApp::snippets, mode = server)]
			pub fn url_patterns() -> ServerRouter {
				ServerRouter::new().viewset_with_actions(
					"snippets-viewset",
					views::viewset(),
					PhantomData::<views::SnippetViewSet>,
				)
			}

			// `#[routes]` iterates every installed app and references
			// `crate::apps::<app>::urls::ws_urls::ws_url_resolvers`.
			// `mode = ws` emits the required empty resolver module so the
			// fan-out call compiles cleanly without any streaming handlers.
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

// `#[routes]` requires the function to return a `UnifiedRouter` — the macro
// calls `.into_server()` on it to populate the global server router used by
// `ResolvedUrls::from_global()`.
//
// `no_client_resolvers` is required because the `snippets` app above declares
// only `#[url_patterns(..., mode = server)]` (plus `mode = ws`) and never emits
// a `client_url_resolvers` module. Without the flag, `#[routes]` expands a
// `crate::apps::snippets::urls::client_url_resolvers::__for_each_client_url_resolver!`
// reference that fails E0433 in `cargo check --tests`. The two sibling
// regression tests (`url_patterns_viewset_collision_regression.rs`,
// `url_patterns_viewset_namespace_regression.rs`) carry the same flag for
// the same reason.
#[reinhardt::routes(no_client_resolvers)]
pub fn routes() -> UnifiedRouter {
	// Replace the default empty `ServerRouter` with the snippets app's
	// `ServerRouter` directly. `UnifiedRouter::server(|s| f(s))` lets the
	// closure return any `ServerRouter`, which we use to install the
	// already-built (`with_namespace("snippets")`-wrapped) router from
	// `#[url_patterns]` without going through `mount("/" ...)` — the
	// latter would prepend a literal "/" prefix and corrupt URL reversal
	// to "///snippets-viewset/".
	UnifiedRouter::new().server(|_| apps::snippets::urls::url_patterns())
}

// === Test fixture helpers ===

/// Ensure the global server router is populated before constructing a
/// `ResolvedUrls`. `#[routes]` submits an inventory entry, but no one walks
/// the inventory inside an integration-test binary (unlike a real
/// application that calls `auto_register_router()` during startup). Register
/// the router manually on first use — subsequent calls are no-ops because
/// `register_router_arc` overwrites the same global slot with an equivalent
/// `Arc`.
///
/// Phase 5.1 of Issue #4507 closes the marker→runtime action-registry
/// bridge: the impl-form `#[viewset(basename = ...)] impl SnippetViewSet`
/// macro now `inventory::submit!`s each `#[action]` keyed by
/// `type_name::<SnippetViewSet>()`, and `viewset_with_actions::<V, M>`
/// copies those entries into the `type_name::<V>()` slot at registration
/// time. No manual `register_action` seeding is required anymore — the
/// `routes()` call below transitively triggers the bridge through
/// `apps::snippets::urls::url_patterns()`.
fn install_routes_and_resolve() -> crate::ResolvedUrls {
	let server = routes().into_server();
	reinhardt_urls::routers::register_router(server);
	crate::ResolvedUrls::from_global()
}

// === Tests ===

#[rstest]
#[serial(routes_global)]
fn viewset_list_typed_accessor_returns_namespaced_url() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let url = urls.server().snippets().snippet_list();

	// Assert
	assert_eq!(url, "/snippets-viewset/");
}

#[rstest]
#[serial(routes_global)]
fn viewset_detail_typed_accessor_substitutes_id() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let url = urls.server().snippets().snippet_detail("42");

	// Assert
	assert_eq!(url, "/snippets-viewset/42/");
}

#[rstest]
#[serial(routes_global)]
fn action_typed_accessor_appears_under_app() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let url = urls.server().snippets().highlight("42");

	// Assert: regression for defect #3 — `#[action]` accessors must appear
	// under the per-app gateway (`urls.server().<app>().<action>()`), not at
	// the top-level `ResolvedUrls` surface.
	assert_eq!(url, "/snippets-viewset/42/highlight/");
}
