#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use reinhardt_pages::static_resolver::{
	browser_asset_snapshot, init_static_resolver, is_initialized, try_resolve_browser_static,
};
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

struct ProjectionElement(web_sys::Element);

impl Drop for ProjectionElement {
	fn drop(&mut self) {
		self.0.remove();
	}
}

#[wasm_bindgen_test]
fn invalid_injected_projection_is_not_treated_as_legacy_configuration() {
	// Arrange: this test binary has its own one-shot browser projection state.
	let document = web_sys::window().unwrap().document().unwrap();
	let element = ProjectionElement(document.create_element("script").unwrap());
	element.0.set_id("reinhardt-static-assets");
	element.0.set_attribute("type", "application/json").unwrap();
	element.0.set_text_content(Some("not JSON"));
	document.body().unwrap().append_child(&element.0).unwrap();
	init_static_resolver("/legacy/".into());

	// Act
	let error = try_resolve_browser_static("app.css").unwrap_err();

	// Assert
	assert!(!is_initialized());
	assert_eq!(browser_asset_snapshot().unwrap_err(), error);
	element.0.remove();
	assert_eq!(try_resolve_browser_static("app.css").unwrap_err(), error);
	assert!(!is_initialized());
}
