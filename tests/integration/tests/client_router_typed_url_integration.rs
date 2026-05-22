//! Integration test: `#[routes]`-generated client URL accessors compile
//! and resolve correctly under the `client-router` feature.
//!
//! Regression coverage for #4089 / PR #4090: the typed
//! `urls.client().<app>().<route>()` accessors and the
//! `<App>ClientUrls::resolve()` fallback both rely on
//! `reinhardt::ClientUrlResolver` being in scope inside macro-generated
//! code. Removing either `use ClientUrlResolver as _;` injection from
//! `crates/reinhardt-core/macros/src/routes_registration.rs` causes this
//! file to fail to compile — exactly the regression class PR #4090 fixed.
//!
//! Refs #4095.

#![cfg(feature = "client-router")]

use reinhardt::installed_apps;
use reinhardt_urls::routers::UnifiedRouter;
use reinhardt_urls::routers::client_router::clear_client_reverser;
use rstest::rstest;
use serial_test::serial;

// Mirror the app set declared by `url_patterns_typed_integration.rs`. The
// `installed_apps!` macro writes a shared state file under
// `target/reinhardt/.installed_apps`, and `#[routes]` reads it back. Keeping
// the app list identical avoids state-file divergence between parallel test
// binaries in this crate.
installed_apps! {
	accounts: "accounts",
	blog: "blog",
}

mod apps {
	pub(crate) mod accounts {
		pub(crate) mod urls {
			use reinhardt::url_patterns;
			use reinhardt_core::page::Page;
			use reinhardt_urls::routers::UnifiedRouter;

			// `mode = unified` emits both `url_resolvers` and
			// `client_url_resolvers` modules, which are the paths
			// `#[routes]` interpolates as `crate::apps::<app>::urls::*`.
			// Path parameter syntax is `{name}` (mirrors examples/*).
			#[url_patterns(crate::InstalledApp::accounts, mode = unified)]
			pub fn unified_url_patterns() -> UnifiedRouter {
				UnifiedRouter::new().client(|c| {
					c.named_route("home", "/", || Page::Empty).named_route(
						"user_detail",
						"/users/{id}/",
						|| Page::Empty,
					)
				})
			}

			// `#[routes]` also references
			// `crate::apps::<app>::urls::ws_urls::ws_url_resolvers` for every
			// installed app. `mode = ws` emits the required module; with no
			// `.consumer()` calls the resolver list is empty (compile-only stub).
			pub(crate) mod ws_urls {
				use reinhardt::url_patterns;
				use reinhardt_urls::routers::UnifiedRouter;

				#[url_patterns(crate::InstalledApp::accounts, mode = ws)]
				pub fn ws_url_patterns() -> UnifiedRouter {
					UnifiedRouter::new()
				}
			}
		}
	}

	// Stub app required by the shared `installed_apps!` declaration above.
	// `#[routes]` iterates every app and references
	// `crate::apps::<app>::urls::client_url_resolvers` (and `ws_urls`); an
	// absent path is a hard compile error, so we provide minimal stubs.
	pub(crate) mod blog {
		pub(crate) mod urls {
			use reinhardt::url_patterns;
			use reinhardt_core::page::Page;
			use reinhardt_urls::routers::UnifiedRouter;

			#[url_patterns(crate::InstalledApp::blog, mode = unified)]
			pub fn unified_url_patterns() -> UnifiedRouter {
				UnifiedRouter::new().client(|c| c.named_route("placeholder", "/", || Page::Empty))
			}

			pub(crate) mod ws_urls {
				use reinhardt::url_patterns;
				use reinhardt_urls::routers::UnifiedRouter;

				#[url_patterns(crate::InstalledApp::blog, mode = ws)]
				pub fn ws_url_patterns() -> UnifiedRouter {
					UnifiedRouter::new()
				}
			}
		}
	}
}

#[reinhardt::routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new()
		.mount_unified("/", apps::accounts::urls::unified_url_patterns())
		.mount_unified("/blog/", apps::blog::urls::unified_url_patterns())
}

// === Test fixture helpers ===

/// Register the test routes globally and return a `ResolvedUrls` snapshot.
///
/// Tests are serialised on the `client_reverser` group because the global
/// `ClientUrlReverser` registry is shared across all tests in this binary.
fn install_routes_and_resolve() -> ResolvedUrls {
	clear_client_reverser();
	// `register_globally` consumes `self` and returns the `ClientRouter`,
	// which we drop — only the global registration matters here.
	let _client = routes().register_globally();
	ResolvedUrls::from_global()
}

// === Tests ===

#[rstest]
#[serial(client_reverser)]
fn typed_accessor_resolves_parameterless_route() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let resolved = urls.client().accounts().home();

	// Assert
	assert_eq!(
		resolved, "/",
		"typed accessor for `home` must resolve to its registered path"
	);

	// Cleanup
	clear_client_reverser();
}

#[rstest]
#[serial(client_reverser)]
fn typed_accessor_resolves_parameterised_route() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let resolved = urls.client().accounts().user_detail("42");

	// Assert
	assert_eq!(
		resolved, "/users/42/",
		"typed accessor must substitute the path parameter"
	);

	// Cleanup
	clear_client_reverser();
}

#[rstest]
#[serial(client_reverser)]
fn resolve_fallback_returns_namespaced_path() {
	// Arrange
	let urls = install_routes_and_resolve();

	// Act
	let resolved = urls.client().accounts().resolve("home", &[]);

	// Assert
	assert_eq!(
		resolved, "/",
		"`resolve()` fallback must look up the namespaced route key `accounts:home`"
	);

	// Cleanup
	clear_client_reverser();
}
