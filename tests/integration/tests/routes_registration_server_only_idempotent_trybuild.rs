//! Positive-path coverage for `#[routes(server_only, no_ws_resolvers)]`
//! — the idempotency assertion called out in
//! `crates/reinhardt-core/macros/src/routes_registration.rs:193-194`.
//!
//! `server_only` is documented as a shorthand for
//! `no_client_resolvers, no_ws_resolvers`. Explicitly repeating
//! `no_ws_resolvers` *on top of* `server_only` must not produce
//! a duplicate-flag compile error — the macro is required to accept the
//! union and treat the repeat as a no-op. This file pins that contract.
//!
//! This file is *both* the entry test and the fixture: its top-level
//! `installed_apps!`, `#[url_patterns]`, and
//! `#[routes(server_only, no_ws_resolvers)]` invocations are expanded by
//! `cargo nextest run`. If macro expansion succeeds, the `compiles` test
//! below executes and the permutation is confirmed to be a valid
//! positive-path use of `#[routes]`.
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

#[routes(server_only, no_ws_resolvers)]
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
	// The presence of both `server_only` (which already implies
	// `no_ws_resolvers`) and an explicit `no_ws_resolvers` MUST NOT
	// produce a duplicate-flag error.
	let _ = routes();
}
