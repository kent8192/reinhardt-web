//! Positive-path coverage for `#[routes(no_client_resolvers)]`.
//!
//! This file is *both* the entry test and the fixture: its top-level
//! `installed_apps!`, `#[url_patterns]`, and `#[routes(no_client_resolvers)]`
//! invocations are expanded by `cargo nextest run`. If macro expansion
//! succeeds, the `compiles` test below executes and the permutation is
//! confirmed to be a valid positive-path use of `#[routes]`.
//!
//! `no_client_resolvers` suppresses the `ClientUrls` gateway and per-app
//! `<app>ClientUrls` structs while leaving WebSocket resolvers intact —
//! the shape an app uses when it ships server + WS surface but no WASM
//! client (see `crates/reinhardt-core/macros/src/routes_registration.rs`).
//!
//! Refs #4553, follow-up to PR #4510 review item I-2.

use reinhardt::ServerRouter;
use reinhardt::UnifiedRouter;
use reinhardt::installed_apps;
use reinhardt::routes;

installed_apps! {
	snippets: "snippets",
}

mod snippets {
	pub mod urls {
		use reinhardt::ServerRouter;
		use reinhardt::url_patterns;

		use super::super::InstalledApp;

		#[url_patterns(InstalledApp::snippets, mode = server)]
		pub fn url_patterns() -> ServerRouter {
			ServerRouter::new()
		}
	}
}

#[routes(no_client_resolvers)]
pub fn routes() -> UnifiedRouter {
	// `#[routes(...)]` preserves this function body verbatim (see
	// `routes_impl` in `crates/reinhardt-core/macros/src/routes_registration.rs`).
	// The `installed_apps!` registry is consulted only for URL *resolver*
	// lookups (`__for_each_url_resolver!`), not for route mounting — the
	// user is responsible for the `.mount()` call.
	UnifiedRouter::new().mount("/api/", crate::snippets::urls::url_patterns())
}

#[test]
fn compiles() {
	// Reaching this assertion proves the macro expansion above compiled.
	let _ = routes();
}
