use reinhardt_pages::app::ClientLauncher;
use reinhardt_pages::component::Page;
use reinhardt_pages::{Loader, component, loader, page};
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

#[loader]
async fn login_loader() -> Result<String, String> {
	Ok("prepared route data".to_string())
}

#[component("/login", name = "login", loader = login_loader)]
fn login_page(Loader(data): Loader<String>) -> Page {
	page!(|data: String| {
		div {
			id: "route-login",
			{ format!("LOGIN VIEW: {data}") }
		}
	})(data)
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	#[cfg(debug_assertions)]
	console_error_panic_hook::set_once();

	ClientLauncher::new("#app")
		.router_client(|| {
			ClientRouter::new()
				.route("home", "/", home_page)
				.component(login_page)
		})
		.launch()
}
