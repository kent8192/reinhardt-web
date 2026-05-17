//! WASM SPA entry point.
//!
//! The `#[routes]`-annotated function in
//! [`crate::config::urls::routes`] aggregates every app's
//! `client_url_patterns()` through `UnifiedRouter::mount_unified` and the
//! macro submits the resulting `ClientRouter` into `inventory` at compile
//! time as a `ClientRouterRegistration`.
//!
//! [`ClientLauncher::register_routes_from_inventory`] consumes those
//! registrations at launch time, merges them into a single SPA route
//! table, registers the project-level client reverser so
//! `ResolvedUrls::from_global()` lookups resolve in components and the
//! nav bar, and installs the router as the SPA mount on `#root`. Refs
//! #4453.

use reinhardt::pages::ClientLauncher;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.register_routes_from_inventory()
		.launch()
}
