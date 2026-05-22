//! WASM entry point
//!
//! This is the main entry point for the WASM application.
use super::router;
use crate::apps::auth::client::state;
use reinhardt::pages::PageExt;
use reinhardt::pages::dom::Element;
pub use router::{init_global_router, with_router};
pub use state::{clear_auth_state, get_current_username, is_authenticated, set_current_user};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	console_error_panic_hook::set_once();
	reinhardt::pages::hydration::init_hydration_state();
	router::init_global_router();
	let window = web_sys::window().expect("no global `window` exists");
	let document = window.document().expect("should have a document on window");
	let root = document
		.get_element_by_id("root")
		.expect("should have #root element");
	root.set_inner_html("");
	router::with_router(|router| {
		let root_element = Element::new(root.clone());
		let current_path = window
			.location()
			.pathname()
			.unwrap_or_else(|_| "/".to_string());
		if let Err(e) = router.replace(&current_path) {
			web_sys::console::error_1(&format!("Failed to navigate: {:?}", e).into());
		}
		let view = router.render_current();
		if let Err(e) = view.mount(&root_element) {
			web_sys::console::error_1(&format!("Failed to mount view: {:?}", e).into());
			return;
		}
	});
	reinhardt::pages::hydration::mark_hydration_complete();
	Ok(())
}
