use reinhardt_pages::app::ClientLauncher;
use reinhardt_pages::component::Page;
use reinhardt_pages::page;
use reinhardt_pages::router::ClientRouter;
use wasm_bindgen::prelude::*;

fn home_page() -> Page {
	page!(|| {
		div {
			id: "route-home",
			a {
				href: "/login",
				id: "go-to-login",
				"Go to login"
			}
		}
	})()
}

fn login_page() -> Page {
	page!(|| {
		div {
			id: "route-login",
			"LOGIN VIEW"
		}
	})()
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	#[cfg(debug_assertions)]
	console_error_panic_hook::set_once();

	ClientLauncher::new("#app")
		.router_client(|| {
			ClientRouter::new()
				.route("home", "/", home_page)
				.route("login", "/login", login_page)
		})
		.launch()
}
