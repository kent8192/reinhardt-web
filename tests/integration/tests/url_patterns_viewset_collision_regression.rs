//! Regression coverage for Issue #4523.
//!
//! Two `#[viewset]`-decorated free functions in different modules sharing
//! the conventional `pub fn viewset()` identifier must compile and resolve
//! cleanly inside the same consumer crate. Before the Issue #4523 fix the
//! `viewset_macro::emit_per_fn_manifest` emitter produced a
//! `#[macro_export] macro_rules! __for_each_viewset_meta_<fn_name>`, which
//! lands at the crate root regardless of where the attribute is applied.
//! Two `pub fn viewset()` in sibling modules both expanded to
//! `__for_each_viewset_meta_viewset`, triggering E0428 "the name … is
//! defined multiple times".
//!
//! This file pins:
//!
//! 1. The crate compiles with two `#[viewset] pub fn viewset()` functions
//!    in different modules (basenames `snippet` and `post`).
//! 2. Both typed list accessors (`snippet_list` / `post_list`) resolve to
//!    their respective namespaced URLs at runtime.
//! 3. Both typed detail accessors substitute the primary key correctly.
//!
//! The fix routes the consumer (`url_patterns::build_viewset_meta_forwarder`)
//! through the scope-respecting bundle module `__viewset_resolvers_<fn>`
//! under the fixed alias `__for_each_meta`, and embeds `<basename>` in
//! the manifest macro identifier so the crate-root namespace is
//! collision-free.
//!
//! Fixes Issue #4523, refs Issue #4507.

// Same reason as `url_patterns_viewset_typed_integration.rs`: the per-fn
// viewset manifest macros are macro-expanded `#[macro_export]` macros that
// `#[url_patterns]` reaches via absolute paths. Until rust-lang/rust#52234
// makes the future-incompat lint a hard error, opt out at the crate level
// so user-written code can compose `#[viewset]` + `#[url_patterns]` +
// `#[routes]` in a single binary.
#![allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]

use reinhardt::installed_apps;
use reinhardt_urls::routers::UnifiedRouter;
use rstest::rstest;
use serial_test::serial;

installed_apps! {
	snippets: "snippets",
	posts: "posts",
}

// === Two model fixtures, one per app ===
//
// The runtime serializer wiring is exercised elsewhere; this file targets
// macro-name collision and URL resolution only.

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
#[reinhardt_macros::model(app_label = "posts", table_name = "posts")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Post {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 255)]
	pub title: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SnippetSerializer;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PostSerializer;

// === Two apps, each with `pub fn viewset()` — sharing the fn identifier
// but with distinct basenames. ===

pub mod apps {
	pub mod snippets {
		pub mod views {
			use reinhardt_macros::viewset;
			use reinhardt_views::viewsets::ModelViewSet;

			// Basename "snippet" — emits `__for_each_viewset_meta_viewset_snippet`.
			#[viewset]
			pub fn viewset()
			-> ModelViewSet<super::super::super::Snippet, super::super::super::SnippetSerializer>
			{
				ModelViewSet::new("snippet")
			}
		}

		pub mod urls {
			use super::views;
			use reinhardt_macros::url_patterns;
			use reinhardt_urls::routers::ServerRouter;

			#[url_patterns(crate::InstalledApp::snippets, mode = server)]
			pub fn url_patterns() -> ServerRouter {
				ServerRouter::new().viewset("snippets-viewset", views::viewset())
			}

			// `#[routes]` walks every installed app and references
			// `crate::apps::<app>::urls::ws_urls::ws_url_resolvers`; emit the
			// required empty resolver module so the fan-out call compiles.
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

	pub mod posts {
		pub mod views {
			use reinhardt_macros::viewset;
			use reinhardt_views::viewsets::ModelViewSet;

			// Basename "post" — emits `__for_each_viewset_meta_viewset_post`.
			// Pre-fix this collided with `apps::snippets::views::viewset`'s
			// `__for_each_viewset_meta_viewset` at the crate root.
			#[viewset]
			pub fn viewset()
			-> ModelViewSet<super::super::super::Post, super::super::super::PostSerializer> {
				ModelViewSet::new("post")
			}
		}

		pub mod urls {
			use super::views;
			use reinhardt_macros::url_patterns;
			use reinhardt_urls::routers::ServerRouter;

			#[url_patterns(crate::InstalledApp::posts, mode = server)]
			pub fn url_patterns() -> ServerRouter {
				ServerRouter::new().viewset("posts-viewset", views::viewset())
			}

			pub mod ws_urls {
				use reinhardt_macros::url_patterns;
				use reinhardt_urls::routers::UnifiedRouter;

				#[url_patterns(crate::InstalledApp::posts, mode = ws)]
				pub fn ws_url_patterns() -> UnifiedRouter {
					UnifiedRouter::new()
				}
			}

		}
	}
}

// === Compose both apps into the global router ===
//
// `UnifiedRouter::server(|s| f(s))` accepts any `ServerRouter`. We chain
// both apps through `.with_namespace`-wrapped routers so each viewset
// resolves under its own per-app prefix.
#[reinhardt::routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().server(|_| {
		use reinhardt_urls::routers::ServerRouter;
		ServerRouter::new()
			.mount("/", apps::snippets::urls::url_patterns())
			.mount("/", apps::posts::urls::url_patterns())
	})
}

// === Fixture: install routes once per test ===

fn install_routes_and_resolve() -> crate::ResolvedUrls {
	let server = routes().into_server();
	reinhardt_urls::routers::register_router(server);
	crate::ResolvedUrls::from_global()
}

// === Tests ===

#[rstest]
#[serial(routes_global)]
fn two_viewsets_sharing_fn_name_both_resolve_list_urls() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let snippet_list = urls.server().snippets().snippet_list();
	let post_list = urls.server().posts().post_list();

	// Assert: the typed list accessor for each app resolves through its
	// own namespace — proving both manifest macros expanded without
	// crate-root collision (Issue #4523).
	assert_eq!(snippet_list, "/snippets-viewset/");
	assert_eq!(post_list, "/posts-viewset/");
}

#[rstest]
#[serial(routes_global)]
fn two_viewsets_sharing_fn_name_both_resolve_detail_urls() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let snippet_detail = urls.server().snippets().snippet_detail("42");
	let post_detail = urls.server().posts().post_detail("99");

	// Assert
	assert_eq!(snippet_detail, "/snippets-viewset/42/");
	assert_eq!(post_detail, "/posts-viewset/99/");
}
