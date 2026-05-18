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

#[routes(no_ws_resolvers)]
pub fn routes() -> UnifiedRouter {
	// Do NOT re-mount `snippets::urls::url_patterns()` here — it is
	// already registered via the `#[url_patterns]` attribute above and
	// discovered by the `#[routes]` expansion through the
	// `installed_apps!` registry. Manually mounting it would cause a
	// double-mount of the per-app server routes.
	UnifiedRouter::new()
}

#[test]
fn compiles() {
	// Reaching this assertion proves the macro expansion above compiled.
	let _ = routes();
}
