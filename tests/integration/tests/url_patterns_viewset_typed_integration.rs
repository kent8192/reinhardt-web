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
				use reinhardt_core::Response;
				use reinhardt_core::views::ViewResult;
				use reinhardt_macros::{action, viewset};

				#[viewset]
				pub fn viewset() -> ModelViewSet<
					super::super::super::Snippet,
					super::super::super::SnippetSerializer,
				> {
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
		// Arrange
		let urls = reinhardt::ResolvedUrls::from_global();

		// Act
		let url = urls.server().snippets().snippet_list();

		// Assert
		assert_eq!(url, "/snippets-viewset/");
	}

	#[ignore = "Issue #4507 — unignore in Phase 9"]
	#[rstest]
	#[serial(routes_global)]
	fn viewset_detail_typed_accessor_substitutes_id() {
		// Arrange
		let urls = reinhardt::ResolvedUrls::from_global();

		// Act
		let url = urls.server().snippets().snippet_detail("42");

		// Assert
		assert_eq!(url, "/snippets-viewset/42/");
	}

	#[ignore = "Issue #4507 — unignore in Phase 9"]
	#[rstest]
	#[serial(routes_global)]
	fn action_typed_accessor_appears_under_app() {
		// Arrange
		let urls = reinhardt::ResolvedUrls::from_global();

		// Act
		let url = urls.server().snippets().highlight("42");

		// Assert
		// Regression: defect #3 from spec section 1
		assert_eq!(url, "/snippets-viewset/42/highlight/");
	}

	#[ignore = "Issue #4507 — unignore in Phase 9"]
	#[rstest]
	#[serial(routes_global)]
	fn flat_blanket_trait_is_deprecated_but_works() {
		// Arrange
		let urls = reinhardt::ResolvedUrls::from_global();
		use reinhardt::url_prelude::*;

		// Act
		#[allow(deprecated)]
		let url = urls.snippet_list();

		// Assert
		// Regression: defect #2 from spec section 1
		assert_eq!(url, "/snippets-viewset/");
	}
}
