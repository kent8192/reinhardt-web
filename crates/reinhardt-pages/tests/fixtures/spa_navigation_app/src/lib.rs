//! Minimal SPA fixture used by `spa_navigation_e2e_test` to verify
//! that `<a href="/...">` clicks trigger SPA navigation and route
//! re-rendering against a real Chrome browser via CDP.
//!
//! Two routes:
//! - `/` renders `<div id="route-home"><a href="/login">Go to login</a></div>`
//! - `/login` renders `<div id="route-login">LOGIN VIEW</div>`
//!
//! Refs #4088.

use reinhardt_pages::app::ClientLauncher;
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::router::Router;
use wasm_bindgen::prelude::*;

fn home_page() -> Page {
	PageElement::new("div")
		.attr("id", "route-home")
		.child(
			PageElement::new("a")
				.attr("href", "/login")
				.attr("id", "go-to-login")
				.child("Go to login"),
		)
		.into_page()
}

fn login_page() -> Page {
	PageElement::new("div")
		.attr("id", "route-login")
		.child("LOGIN VIEW")
		.into_page()
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	console_error_panic_hook::set_once();

	ClientLauncher::new("#app")
		.router(|| {
			Router::new()
				.route("/", home_page)
				.route("/login", login_page)
		})
		.launch()
}
