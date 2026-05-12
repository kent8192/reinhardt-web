//! WASM shim for `reinhardt_apps` (Issue #4161).
//!
//! `#[url_patterns(...)]` and `#[app_config(...)]` expand to code that
//! references `::reinhardt::reinhardt_apps::apps::AppLabel` and
//! `::reinhardt::reinhardt_apps::AppConfig`. The real `reinhardt-apps`
//! crate depends on `tokio` / `reinhardt-server` and is decidedly
//! native-only, so on wasm we expose only the surface the macro emits.
//!
//! These shims compile but never execute: the dashboard-style SPA imports
//! them transitively, but only constructs `UnifiedRouter` / `WebSocketRouter`,
//! which are themselves wasm-side stubs.
//!
//! Re-exported at `crate::reinhardt_apps` from `src/lib.rs` to preserve the
//! macro-expected canonical path.

/// Application label trait (wasm shim).
///
/// Mirrors the trait emitted by `installed_apps!` and required by
/// `#[url_patterns]` expansions. The native build re-exports the real
/// trait from `reinhardt-apps`.
//
// `clippy::module_inception` is suppressed because the nested `apps`
// namespace is required by the canonical macro path
// `::reinhardt::reinhardt_apps::apps::AppLabel`: this file is re-exported
// from `src/lib.rs` as `pub use compat::apps as reinhardt_apps;`, after
// which the inner `apps` module restores the original two-level structure
// that macro-generated code expects.
#[allow(clippy::module_inception)]
pub mod apps {
	pub trait AppLabel {
		const LABEL: &'static str;
		fn path(&self) -> &'static str {
			Self::LABEL
		}
	}
}

/// Application configuration (wasm shim).
///
/// `#[app_config(name = "...", label = "...")]` expands to
/// `pub fn config() -> AppConfig { AppConfig::new(name, label).with_verbose_name(...) }`.
/// On wasm we provide a builder-shaped stub with the same signatures so
/// the expansion compiles. None of these methods are intended to be
/// invoked at runtime in a wasm consumer.
pub struct AppConfig {
	_private: (),
}

impl AppConfig {
	pub fn new(_name: impl Into<String>, _label: impl Into<String>) -> Self {
		Self { _private: () }
	}

	pub fn with_verbose_name(self, _verbose_name: impl Into<String>) -> Self {
		self
	}
}
