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

use reinhardt::UnifiedRouter;
use reinhardt::installed_apps;
use reinhardt::routes;

installed_apps! {
	snippets: "snippets",
}

// `#[routes(...)]` emits absolute paths of the form
// `crate::apps::<app>::urls::url_resolvers::...` (see
// `crates/reinhardt-core/macros/src/routes_registration.rs:905`). The
// fixture's app module must therefore live under a `mod apps { ... }`
// parent so the emitted path resolves; placing `mod snippets` at the
// crate root would yield E0433. See #4596.
//
// `no_client_resolvers` suppresses *only* the per-app client resolver
// lookups; the WS resolver path
// `crate::apps::<app>::urls::ws_urls::ws_url_resolvers` is still
// emitted (see `routes_registration.rs:1109` — that branch is gated by
// `no_ws_resolvers`, not by `no_client_resolvers`). So this fixture
// must also expose a `ws_urls` submodule populated by
// `#[url_patterns(..., mode = ws)]`.
mod apps {
	pub(crate) mod snippets {
		pub(crate) mod urls {
			use reinhardt::ServerRouter;
			use reinhardt::url_patterns;

			use super::super::super::InstalledApp;

			#[url_patterns(InstalledApp::snippets, mode = server)]
			pub fn url_patterns() -> ServerRouter {
				ServerRouter::new()
			}

			// Empty stub: `#[routes]` walks every installed app and
			// references `crate::apps::<app>::urls::ws_urls::ws_url_resolvers`.
			// `mode = ws` emits the required resolver module with no entries.
			pub(crate) mod ws_urls {
				use reinhardt::UnifiedRouter;
				use reinhardt::url_patterns;

				use super::super::super::super::InstalledApp;

				#[url_patterns(InstalledApp::snippets, mode = ws)]
				pub fn ws_url_patterns() -> UnifiedRouter {
					UnifiedRouter::new()
				}
			}
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
	UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}

#[test]
fn compiles() {
	// Reaching this assertion proves the macro expansion above compiled.
	let _ = routes();
}
