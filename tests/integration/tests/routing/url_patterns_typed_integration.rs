//! Integration tests for the typed `#[url_patterns]` syntax (Issue #3670).
//!
//! Verifies that:
//! - `#[url_patterns(InstalledApp::variant, mode = server|client|unified)]`
//!   applies the namespace at runtime via `AppLabel::path()`.
//! - `installed_apps!` generates the `AppLabel` impl correctly.

use reinhardt::installed_apps;
use reinhardt_apps::apps::AppLabel;
use reinhardt_urls::routers::ServerRouter;

installed_apps! {
	accounts: "accounts",
	blog: "blog",
}

// --- installed_apps! <-> AppLabel ---

#[test]
fn installed_app_implements_app_label() {
	// Arrange
	let app = InstalledApp::accounts;

	// Act
	let label: &'static str = <InstalledApp as AppLabel>::path(&app);

	// Assert
	assert_eq!(label, "accounts");
}

#[test]
fn installed_app_all_variants_resolve_through_app_label() {
	// Act & Assert
	assert_eq!(
		<InstalledApp as AppLabel>::path(&InstalledApp::accounts),
		"accounts"
	);
	assert_eq!(
		<InstalledApp as AppLabel>::path(&InstalledApp::blog),
		"blog"
	);
}

// --- Server mode: two apps in separate submodules (resolver modules collide
// at the same scope, so distinct apps must live in distinct modules). ---

mod accounts_app {
	use super::{InstalledApp, ServerRouter};
	use reinhardt::url_patterns;

	#[url_patterns(super::InstalledApp::accounts, mode = server)]
	pub fn server_url_patterns() -> ServerRouter {
		ServerRouter::new()
	}
}

mod blog_app {
	use super::{InstalledApp, ServerRouter};
	use reinhardt::url_patterns;

	#[url_patterns(super::InstalledApp::blog, mode = server)]
	pub fn server_url_patterns() -> ServerRouter {
		ServerRouter::new()
	}
}

#[test]
fn server_mode_applies_namespace_from_installed_app_variant() {
	// Arrange
	let router = accounts_app::server_url_patterns();

	// Act
	let namespace = router.namespace();

	// Assert
	assert_eq!(
		namespace,
		Some("accounts"),
		"namespace must be resolved from InstalledApp::accounts via AppLabel::path()"
	);
}

#[test]
fn server_mode_two_apps_in_separate_modules_have_distinct_namespaces() {
	// Arrange / Act
	let a = accounts_app::server_url_patterns();
	let b = blog_app::server_url_patterns();

	// Assert
	assert_eq!(a.namespace(), Some("accounts"));
	assert_eq!(b.namespace(), Some("blog"));
}

// --- Client / unified modes: the wrapper applies `.with_namespace(...)` to
// every router type, so verify the namespace reaches client-side named
// routes as well. ---

#[cfg(feature = "client-router")]
mod accounts_client_app {
	use super::InstalledApp;
	use reinhardt::url_patterns;
	use reinhardt_core::page::Page;
	use reinhardt_urls::routers::ClientRouter;

	#[url_patterns(super::InstalledApp::accounts, mode = client)]
	pub fn client_url_patterns() -> ClientRouter {
		ClientRouter::new().named_route("login", "/login/", || Page::Empty)
	}
}

#[cfg(feature = "client-router")]
mod accounts_unified_app {
	use super::InstalledApp;
	use reinhardt::url_patterns;
	use reinhardt_core::page::Page;
	use reinhardt_urls::routers::UnifiedRouter;

	#[url_patterns(super::InstalledApp::accounts, mode = unified)]
	pub fn unified_url_patterns() -> UnifiedRouter {
		UnifiedRouter::new().client(|c| c.named_route("login", "/login/", || Page::Empty))
	}
}

#[cfg(feature = "client-router")]
#[test]
fn client_mode_applies_namespace_to_named_route_keys() {
	// Arrange / Act
	let router = accounts_client_app::client_url_patterns();

	// Assert
	assert!(
		router.has_route("accounts:login"),
		"client-mode wrapper must prefix named routes with the AppLabel path"
	);
	assert!(
		!router.has_route("login"),
		"unprefixed name should no longer resolve after the wrapper applies the namespace"
	);
}

#[cfg(feature = "client-router")]
#[test]
fn unified_mode_applies_namespace_to_both_server_and_client_sides() {
	// Arrange / Act
	let router = accounts_unified_app::unified_url_patterns();

	// Assert: client side is namespaced via key rewriting.
	assert!(
		router.client_ref().has_route("accounts:login"),
		"unified-mode wrapper must propagate the namespace to the inner ClientRouter"
	);
	// Assert: server side carries the same namespace.
	assert_eq!(
		router.server_ref().namespace(),
		Some("accounts"),
		"unified-mode wrapper must propagate the namespace to the inner ServerRouter"
	);
}

// --- Typed `urls::*` helpers (Issue #4644) ---
//
// The macro emits a sibling `urls` module containing a `pub fn` per named
// route whose path parameters it can lift from the closure binding. The
// helpers delegate to the globally-registered `ClientUrlReverser`, so the
// tests below register the reverser explicitly under
// `#[serial(client_reverser)]` to avoid races with any other suite that
// touches the same singleton.

#[cfg(feature = "client-router")]
mod typed_accounts_app {
	use super::InstalledApp;
	use reinhardt::url_patterns;
	use reinhardt_core::page::Page;
	use reinhardt_urls::routers::ClientRouter;
	use reinhardt_urls::routers::client_router::Path as ClientPath;

	// Mix of zero-param and typed-param routes so the helper module exercises
	// both branches of the closure-extraction code path. `index` and
	// `question_new` produce `fn() -> String` helpers; the `_path` variants
	// produce `fn(i64) -> String` and `fn(i64, i64) -> String`.
	#[url_patterns(super::InstalledApp::accounts, mode = client)]
	pub fn client_url_patterns() -> ClientRouter {
		ClientRouter::new()
			.named_route("index", "/", || Page::Empty)
			.named_route("question_new", "/polls/new/", || Page::Empty)
			.named_route_path(
				"detail",
				"/polls/{question_id}/",
				|ClientPath(_question_id): ClientPath<i64>| Page::Empty,
			)
			.named_route_path2(
				"choice_edit",
				"/polls/{question_id}/choices/{choice_id}/edit/",
				|ClientPath(_question_id): ClientPath<i64>,
				 ClientPath(_choice_id): ClientPath<i64>| Page::Empty,
			)
	}
}

#[cfg(feature = "client-router")]
fn install_reverser_for(router: reinhardt_urls::routers::ClientRouter) {
	use reinhardt_urls::routers::client_router::{
		clear_client_reverser, register_client_reverser,
	};
	clear_client_reverser();
	register_client_reverser(router.to_reverser());
}

#[cfg(feature = "client-router")]
#[test]
#[serial_test::serial(client_reverser)]
fn typed_urls_zero_param_helper_returns_namespaced_pattern() {
	// Arrange
	install_reverser_for(typed_accounts_app::client_url_patterns());

	// Act
	let href: String = typed_accounts_app::urls::index();

	// Assert: pattern is the static "/" — namespacing is in the registry
	// key, not the URL itself.
	assert_eq!(
		href, "/",
		"the typed `urls::index()` helper must round-trip through the global reverser"
	);

	// Cleanup
	reinhardt_urls::routers::client_router::clear_client_reverser();
}

#[cfg(feature = "client-router")]
#[test]
#[serial_test::serial(client_reverser)]
fn typed_urls_single_param_helper_substitutes_path_segment() {
	// Arrange
	install_reverser_for(typed_accounts_app::client_url_patterns());

	// Act
	let href: String = typed_accounts_app::urls::detail(42);

	// Assert
	assert_eq!(
		href, "/polls/42/",
		"typed single-param helper must substitute the bound `question_id` into the pattern"
	);

	// Cleanup
	reinhardt_urls::routers::client_router::clear_client_reverser();
}

#[cfg(feature = "client-router")]
#[test]
#[serial_test::serial(client_reverser)]
fn typed_urls_two_param_helper_substitutes_both_segments_by_position() {
	// Arrange
	install_reverser_for(typed_accounts_app::client_url_patterns());

	// Act
	let href: String = typed_accounts_app::urls::choice_edit(7, 13);

	// Assert: the macro must pass parameters by position matching the
	// closure binding order, so the first argument fills `question_id`
	// and the second fills `choice_id`.
	assert_eq!(
		href, "/polls/7/choices/13/edit/",
		"typed two-param helper must respect binding-order positional substitution"
	);

	// Cleanup
	reinhardt_urls::routers::client_router::clear_client_reverser();
}

#[cfg(feature = "client-router")]
#[test]
#[serial_test::serial(client_reverser)]
fn typed_urls_helpers_panic_when_no_reverser_is_registered() {
	// Arrange
	reinhardt_urls::routers::client_router::clear_client_reverser();

	// Act
	let result = std::panic::catch_unwind(|| typed_accounts_app::urls::index());

	// Assert: helpers must surface "no reverser registered" loudly rather
	// than silently returning an empty string. The full panic string is
	// pinned with assert_eq! so any wording drift in the macro requires
	// an intentional update of this test, in line with the project's
	// "use strict assertions instead of loose matching" rule.
	let err = result.expect_err("typed helper must panic when no reverser is registered");
	let msg = err
		.downcast_ref::<&'static str>()
		.map(|s| (*s).to_string())
		.or_else(|| err.downcast_ref::<String>().cloned())
		.unwrap_or_default();
	assert_eq!(
		msg,
		"client URL reverser is not registered. \
		 Register the client URL reverser globally \
		 (e.g. via `UnifiedRouter::register_globally()` or \
		 `register_client_reverser(...)`) before calling \
		 the typed `urls::*` helpers.",
		"panic message must match the macro's emitted text verbatim"
	);
}

// --- Regression coverage for binding-name <-> placeholder pairing ---
//
// Helpers must pair each closure binding to its URL placeholder by *name*
// (with leading underscores stripped), not by position. The app below
// declares the closure inputs in the *opposite* order from the pattern
// placeholders: pattern `{question_id}/.../{choice_id}/...` but bindings
// `_choice_id, _question_id`. The emitted helper must therefore expose a
// signature `fn(choice_id, question_id) -> String` and substitute each
// placeholder from the matching named binding, not from positional order.

#[cfg(feature = "client-router")]
mod typed_accounts_app_swapped_order {
	use super::InstalledApp;
	use reinhardt::url_patterns;
	use reinhardt_core::page::Page;
	use reinhardt_urls::routers::ClientRouter;
	use reinhardt_urls::routers::client_router::Path as ClientPath;

	#[url_patterns(super::InstalledApp::accounts, mode = client)]
	pub fn client_url_patterns() -> ClientRouter {
		ClientRouter::new().named_route_path2(
			"choice_edit",
			"/polls/{question_id}/choices/{choice_id}/edit/",
			|ClientPath(_choice_id): ClientPath<i64>,
			 ClientPath(_question_id): ClientPath<i64>| Page::Empty,
		)
	}
}

#[cfg(feature = "client-router")]
#[test]
#[serial_test::serial(client_reverser)]
fn typed_urls_helper_pairs_bindings_to_placeholders_by_name() {
	// Arrange: register a reverser whose closure binds parameters in
	// the opposite order from the placeholders.
	install_reverser_for(typed_accounts_app_swapped_order::client_url_patterns());

	// Act: pass arguments in the closure's binding order
	// (choice_id, question_id) — the helper must place each value at
	// the named placeholder, not at the positionally-matching one.
	let href: String = typed_accounts_app_swapped_order::urls::choice_edit(13, 7);

	// Assert: `{question_id}` resolves to 7, `{choice_id}` resolves to 13.
	// A position-based implementation would (incorrectly) produce
	// "/polls/13/choices/7/edit/" here.
	assert_eq!(
		href, "/polls/7/choices/13/edit/",
		"typed helper must substitute each placeholder from its NAMED binding, not by position"
	);

	// Cleanup
	reinhardt_urls::routers::client_router::clear_client_reverser();
}
