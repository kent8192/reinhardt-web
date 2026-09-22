#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use reinhardt_pages::static_resolver::{
	AssetUrlError, AssetUrlSnapshot, browser_asset_snapshot, component_stylesheet_url,
	init_static_resolver, is_initialized, resolve_static, try_resolve_browser_static,
};
use std::collections::BTreeMap;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

struct ProjectionElement(web_sys::Element);

impl Drop for ProjectionElement {
	fn drop(&mut self) {
		self.0.remove();
	}
}

#[wasm_bindgen_test]
fn established_resolvers_use_and_pin_the_injected_projection() {
	// Arrange: an early legacy lookup must not permanently cache an absent projection.
	let document = web_sys::window().unwrap().document().unwrap();
	assert!(
		document
			.get_element_by_id("reinhardt-static-assets")
			.is_none()
	);
	assert!(!is_initialized());
	assert!(browser_asset_snapshot().is_err());
	assert_eq!(resolve_static("/logo.svg"), "/static/logo.svg");
	init_static_resolver("/legacy/".into());
	assert_eq!(resolve_static("logo.svg"), "/legacy/logo.svg");
	assert!(is_initialized());

	let id = "a".repeat(64);
	let prefix = "https://cdn.example.test/console/assets/";
	let projection = AssetUrlSnapshot::new(
		id.clone(),
		prefix.into(),
		BTreeMap::from([
			(
				"logo.svg".into(),
				format!("builds/{id}/vectors/logo.hash.svg"),
			),
			(
				"__reinhardt__/components.css".into(),
				format!("builds/{id}/css/components.hash.css"),
			),
		]),
	)
	.unwrap();
	let element = ProjectionElement(document.create_element("script").unwrap());
	element.0.set_id("reinhardt-static-assets");
	element.0.set_attribute("type", "application/json").unwrap();
	element
		.0
		.set_text_content(Some(&projection.to_json().unwrap()));
	document.body().unwrap().append_child(&element.0).unwrap();

	// Act
	let logo = resolve_static("logo.svg");
	let leading_slash = resolve_static("/logo.svg");
	let stylesheet = component_stylesheet_url();

	// Assert: existing component calls do not require an opt-in resolver API.
	assert_eq!(logo, format!("{prefix}builds/{id}/vectors/logo.hash.svg"));
	assert_eq!(leading_slash, logo);
	assert_eq!(
		stylesheet,
		format!("{prefix}builds/{id}/css/components.hash.css")
	);
	assert_eq!(browser_asset_snapshot().unwrap().build_id(), id);
	assert!(matches!(
		try_resolve_browser_static("missing.css"),
		Err(AssetUrlError::UnknownLogicalPath { .. })
	));

	// Changing or removing the DOM element cannot mix generations on an existing page.
	element.0.set_text_content(Some("invalid replacement"));
	assert_eq!(resolve_static("logo.svg"), logo);
	element.0.remove();
	assert_eq!(component_stylesheet_url(), stylesheet);
}
