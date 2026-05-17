//! Integration coverage for Issue #4507.
//!
//! When this entire file is unignored (Phase 9), the test suite must prove that
//! `ViewSet` routes registered via `#[url_patterns(..., mode = server)]` produce
//! typed `ResolvedUrls` accessors equivalent to function-handler accessors.
//!
//! Refs Issue #4507.

// PHASE 9 CHECKLIST: remove this `#[cfg(any())]` and the four #[ignore]
// markers below once viewset_with_actions and #[viewset] impl-form are
// implemented (Phases 2 ~ 7 of the plan).
#[cfg(any())]
mod gated {
	use reinhardt::installed_apps;
	use reinhardt_urls::routers::ServerRouter;
	use rstest::rstest;
	use serial_test::serial;
	use std::marker::PhantomData;

	installed_apps! {
		snippets: "snippets",
	}

	mod apps {
		pub mod snippets {
			pub mod views {
				use reinhardt::ModelViewSet;
				use reinhardt_macros::{action, viewset};
				use reinhardt_core::views::ViewResult;
				use reinhardt_core::Response;

				#[viewset]
				pub fn viewset() -> ModelViewSet<super::super::super::Snippet, super::super::super::SnippetSerializer> {
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
				use reinhardt::url_patterns;
				use reinhardt_urls::routers::ServerRouter;
				use std::marker::PhantomData;

				#[url_patterns(crate::InstalledApp::snippets, mode = server)]
				pub fn url_patterns() -> ServerRouter {
					ServerRouter::new().viewset_with_actions(
						"/snippets-viewset",
						views::viewset(),
						PhantomData::<views::SnippetViewSet>,
					)
				}
			}
		}
	}

	// Stub model + serializer; Phase 9 replaces with real fixtures.
	pub struct Snippet;
	pub struct SnippetSerializer;

	#[reinhardt::routes]
	fn routes() -> ServerRouter {
		apps::snippets::urls::url_patterns()
	}

	#[ignore = "Issue #4507 — unignore in Phase 9 once macros support typed ViewSet accessors"]
	#[rstest]
	#[serial(routes_global)]
	fn viewset_list_typed_accessor_returns_namespaced_url() {
		let urls = reinhardt::ResolvedUrls::from_global();
		let url = urls.server().snippets().snippet_list();
		assert_eq!(url, "/snippets-viewset/");
	}

	#[ignore = "Issue #4507 — unignore in Phase 9"]
	#[rstest]
	#[serial(routes_global)]
	fn viewset_detail_typed_accessor_substitutes_id() {
		let urls = reinhardt::ResolvedUrls::from_global();
		let url = urls.server().snippets().snippet_detail("42");
		assert_eq!(url, "/snippets-viewset/42/");
	}

	#[ignore = "Issue #4507 — unignore in Phase 9"]
	#[rstest]
	#[serial(routes_global)]
	fn action_typed_accessor_appears_under_app() {
		let urls = reinhardt::ResolvedUrls::from_global();
		let url = urls.server().snippets().highlight("42");
		// Regression: defect #3 from spec section 1
		assert_eq!(url, "/snippets-viewset/42/highlight/");
	}

	#[ignore = "Issue #4507 — unignore in Phase 9"]
	#[rstest]
	#[serial(routes_global)]
	fn flat_blanket_trait_is_deprecated_but_works() {
		use reinhardt::url_prelude::*;
		let urls = reinhardt::ResolvedUrls::from_global();
		#[allow(deprecated)]
		let url = urls.snippet_list();
		// Regression: defect #2 from spec section 1
		assert_eq!(url, "/snippets-viewset/");
	}
}
