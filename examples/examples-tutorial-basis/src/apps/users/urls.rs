//! URL configuration for the users application.
//!
//! Both submodules use `#[url_patterns(InstalledApp::users, mode = ...)]`,
//! so the framework auto-registers them via inventory and auto-prefixes
//! the path with `/users/`.
#[cfg(wasm)]
pub mod client_router;
#[cfg(native)]
pub mod server_urls;
