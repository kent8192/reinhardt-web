#![cfg(wasm)]

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_pages::component::{
	Component, ControlBinding, ControlBindingError, ControlKind, MountError, Page, PageExt,
};
use reinhardt_pages::dom::Element;
use reinhardt_pages::reactive::Signal;
use reinhardt_pages::{PageElement, page};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

struct SsrStateElement(web_sys::Element);

impl SsrStateElement {
	fn install(document: &web_sys::Document) -> Self {
		if let Some(existing) = document.get_element_by_id("ssr-state") {
			existing.remove();
		}
		let element = document.create_element("script").expect("state element");
		element.set_id("ssr-state");
		element.set_text_content(Some(""));
		document
			.body()
			.expect("body")
			.append_child(&element)
			.expect("state mount");
		Self(element)
	}
}

impl Drop for SsrStateElement {
	fn drop(&mut self) {
		self.0.remove();
	}
}

#[wasm_bindgen_test]
fn public_page_mount_installs_control_binding() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let value = Signal::new("signal".to_owned());
	let observed = Rc::new(RefCell::new(String::new()));
	let handler_value = value.clone();
	let handler_observed = Rc::clone(&observed);
	page!({
		input {
			a11y: off,
			bind: value,
			@input: move |_| *handler_observed.borrow_mut() = handler_value.get(),
		}
	})
	.mount(&root)
	.expect("mount");
	let input: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("input")
		.unchecked_into();

	assert_eq!(input.value(), "signal");
	input.set_value("dom");
	input
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "dom");
	assert_eq!(&*observed.borrow(), "dom");
	reinhardt_pages::cleanup_reactive_nodes();
}

#[wasm_bindgen_test]
fn public_page_mount_preserves_a_structured_binding_error() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let checked = Signal::new(false);
	let page = Page::Element(
		PageElement::new("select").control_binding(ControlBinding::checkbox(checked)),
	);

	let error = page.mount(&root).expect_err("mismatch");

	assert_eq!(
		error,
		MountError::ControlBinding(ControlBindingError::UnsupportedElement {
			control: ControlKind::Checkbox,
			actual_tag: "select".to_owned(),
		})
	);
}

#[wasm_bindgen_test]
fn reactive_remount_drops_the_replaced_control_owner() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let alternate = Signal::new(false);
	let value = Signal::new("initial".to_owned());
	let render_alternate = alternate.clone();
	let render_value = value.clone();
	Page::reactive(move || {
		let bound = render_value.clone();
		if render_alternate.get() {
			page!({
				input {
					a11y: off,
					id: "replacement",
					bind: bound,
				}
			})
		} else {
			page!({
				input {
					a11y: off,
					id: "original",
					bind: bound,
				}
			})
		}
	})
	.mount(&root)
	.expect("mount");
	let original: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("original")
		.unchecked_into();

	alternate.set(true);
	let replacement: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("replacement")
		.unchecked_into();
	assert_eq!(replacement.id(), "replacement");
	original.set_value("stale");
	original
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "initial");
	value.set("current".to_owned());
	assert_eq!(replacement.value(), "current");
	reinhardt_pages::cleanup_reactive_nodes();
}

struct HydratedInput {
	value: Signal<String>,
	observed: Rc<RefCell<String>>,
}

impl Component for HydratedInput {
	fn name() -> &'static str {
		"HydratedInput"
	}

	fn render(&self) -> Page {
		let value = self.value.clone();
		let handler_value = self.value.clone();
		let handler_observed = Rc::clone(&self.observed);
		page!({
			input {
				a11y: off,
				bind: value,
				@input: move |_| *handler_observed.borrow_mut() = handler_value.get(),
			}
		})
	}
}

#[wasm_bindgen_test]
fn public_hydration_adopts_the_live_dom_property() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let raw = document.create_element("input").expect("input");
	let input: web_sys::HtmlInputElement = raw.clone().unchecked_into();
	input.set_value("restored");
	let root = Element::new(raw);
	let value = Signal::new("server".to_owned());
	let observed = Rc::new(RefCell::new(String::new()));
	let _state = SsrStateElement::install(&document);

	reinhardt_pages::hydration::hydrate(
		&HydratedInput {
			value: value.clone(),
			observed: Rc::clone(&observed),
		},
		&root,
	)
	.expect("hydrate");

	assert_eq!(value.get(), "restored");
	input.set_value("edited");
	input
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "edited");
	assert_eq!(&*observed.borrow(), "edited");
	reinhardt_pages::cleanup_reactive_nodes();
}
