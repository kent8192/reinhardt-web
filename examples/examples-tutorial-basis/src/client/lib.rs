//! WASM entry point.
//!
//! Bootstraps the SPA via `ClientLauncher::router_client`, which installs
//! the panic hook, history listener, and DOM mount on `#root`, and looks
//! up the polls app's client router (registered via
//! `#[url_patterns(InstalledApp::polls, mode = client)]`).

use reinhardt::pages::ClientLauncher;
use wasm_bindgen::prelude::*;

use crate::apps::polls::urls::client_router::client_url_patterns;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.router_client(client_url_patterns)
		.launch()
}
