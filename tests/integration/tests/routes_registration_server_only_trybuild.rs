//! Positive-path coverage for `#[routes(server_only)]`.
//!
//! This file is *both* the entry test and the fixture: its top-level
//! `installed_apps!`, `#[url_patterns]`, and `#[routes(server_only)]`
//! invocations are expanded by `cargo nextest run`. If macro expansion
//! succeeds, the `compiles` test below executes and the permutation is
//! confirmed to be a valid positive-path use of `#[routes]`.
//!
//! `server_only` is a shorthand for `no_client_resolvers, no_ws_resolvers`
//! (see `crates/reinhardt-core/macros/src/routes_registration.rs`). It is
//! intended for REST-only apps that consume `installed_apps!` but do not
//! ship per-app `client_url_resolvers` / `ws_url_resolvers` modules — the
//! exact shape `examples-tutorial-rest` uses.
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

#[routes(server_only)]
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
