#![cfg(wasm)]

use reinhardt_pages::component::{IntoPage, PageElement};
use reinhardt_pages::portal::{PortalTarget, mount_portal};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn install_portal_target() -> web_sys::Element {
	let document = web_sys::window().unwrap().document().unwrap();
	if let Some(prev) = document.get_element_by_id("portal-target") {
		prev.remove();
	}
	if let Some(prev) = document.get_element_by_id("portal-child") {
		prev.remove();
	}

	let target = document.create_element("div").unwrap();
	target.set_id("portal-target");
	document.body().unwrap().append_child(&target).unwrap();
	target
}

#[wasm_bindgen_test]
fn portal_mounts_page_into_target_and_cleans_up_on_drop() {
	let target = install_portal_target();
	let document = web_sys::window().unwrap().document().unwrap();
	let handle = mount_portal(
		PortalTarget::element_id("portal-target"),
		PageElement::new("span")
			.attr("id", "portal-child")
			.child("PORTAL-CONTENT")
			.into_page(),
	)
	.expect("portal mount");

	assert!(handle.is_active());
	assert!(
		target
			.inner_html()
			.contains("<span id=\"portal-child\">PORTAL-CONTENT</span>")
	);
	assert!(document.get_element_by_id("portal-child").is_some());
	assert!(
		target
			.query_selector("[data-rh-portal-host]")
			.unwrap()
			.is_some()
	);

	drop(handle);

	assert!(document.get_element_by_id("portal-child").is_none());
	assert!(
		target
			.query_selector("[data-rh-portal-host]")
			.unwrap()
			.is_none()
	);
	target.remove();
}
