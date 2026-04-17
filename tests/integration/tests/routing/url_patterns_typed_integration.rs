//! Integration tests for the typed `#[url_patterns]` syntax (Issue #3670).
//!
//! Verifies that:
//! - `#[url_patterns(InstalledApp::<variant>, mode = server|client|unified)]`
//!   applies the namespace at runtime via `AppLabel::path()`.
//! - `installed_apps!` generates the `AppLabel` impl correctly.

use reinhardt::installed_apps;
use reinhardt::url_patterns;
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
