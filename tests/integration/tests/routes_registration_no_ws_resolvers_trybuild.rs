//! Positive-path coverage for `#[routes(no_ws_resolvers)]`.
//!
//! This file is *both* the entry test and the fixture: its top-level
//! `installed_apps!`, `#[url_patterns]`, and `#[routes(no_ws_resolvers)]`
//! invocations are expanded by `cargo nextest run`. If macro expansion
//! succeeds, the `compiles` test below executes and the permutation is
//! confirmed to be a valid positive-path use of `#[routes]`.
//!
//! `no_ws_resolvers` suppresses the `WsUrls` gateway, per-app `<app>WsUrls`
//! structs, and the stub `WebSocketUrlResolver` impl while leaving client
//! resolvers intact — the shape an app uses when it ships server + WASM
//! client surface but no WebSocket endpoints (see
//! `crates/reinhardt-core/macros/src/routes_registration.rs`).
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
// `no_ws_resolvers` suppresses *only* the per-app WS resolver lookups;
// the client resolver path
// `crate::apps::<app>::urls::client_url_resolvers` is still emitted
// when the `client-router` feature is enabled (see
// `routes_registration.rs:999` — that branch is gated by
// `no_client_resolvers`, not by `no_ws_resolvers`). `mode = unified`
// emits both `url_resolvers` *and* `client_url_resolvers` modules in
// a single `#[url_patterns]` expansion, satisfying both fan-outs.
mod apps {
	pub(crate) mod snippets {
		pub(crate) mod urls {
			use reinhardt::UnifiedRouter;
			use reinhardt::url_patterns;

			use super::super::super::InstalledApp;

			#[url_patterns(InstalledApp::snippets, mode = unified)]
			pub fn url_patterns() -> UnifiedRouter {
				UnifiedRouter::new()
			}
		}
	}
}

#[routes(no_ws_resolvers)]
pub fn routes() -> UnifiedRouter {
	// `#[routes(...)]` preserves this function body verbatim (see
	// `routes_impl` in `crates/reinhardt-core/macros/src/routes_registration.rs`).
	// The `installed_apps!` registry is consulted only for URL *resolver*
	// lookups (`__for_each_url_resolver!`), not for route mounting — the
	// user is responsible for the `.mount()` call.
	UnifiedRouter::new().mount_unified("/api/", crate::apps::snippets::urls::url_patterns())
}

#[test]
fn compiles() {
	// Reaching this assertion proves the macro expansion above compiled.
	let _ = routes();
}
