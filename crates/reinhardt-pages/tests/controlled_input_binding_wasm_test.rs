#![cfg(wasm)]

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_pages::component::{
	Component, ControlBinding, ControlBindingError, ControlKind, IntoPage, MountError, Page,
	PageExt,
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

struct HydratedReactiveInput {
	alternate: Signal<bool>,
	value: Signal<String>,
	observed: Rc<RefCell<String>>,
}

impl Component for HydratedReactiveInput {
	fn name() -> &'static str {
		"HydratedReactiveInput"
	}

	fn render(&self) -> Page {
		let alternate = self.alternate.clone();
		let value = self.value.clone();
		let observed = Rc::clone(&self.observed);
		PageElement::new("div")
			.child(Page::reactive(move || {
				let _rendered_value = value.get();
				let bound = value.clone();
				let handler_value = value.clone();
				let handler_observed = Rc::clone(&observed);
				let id = if alternate.get() {
					"replacement"
				} else {
					"original"
				};
				page!({
					input {
						a11y: off,
						id: id,
						bind: bound,
						@change: move |_| {
							*handler_observed.borrow_mut() = handler_value.get();
						},
					}
				})
			}))
			.into_page()
	}
}

#[wasm_bindgen_test]
fn hydrated_reactive_switch_drops_the_initial_branch_guards() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let raw_root = document.create_element("div").expect("root");
	let raw_input = document.create_element("input").expect("input");
	raw_input.set_id("original");
	let original: web_sys::HtmlInputElement = raw_input.clone().unchecked_into();
	original.set_value("restored");
	raw_root.append_child(&raw_input).expect("SSR child");
	let root = Element::new(raw_root);
	let alternate = Signal::new(false);
	let value = Signal::new("server".to_owned());
	let observed = Rc::new(RefCell::new(String::new()));
	let _state = SsrStateElement::install(&document);
	reinhardt_pages::hydration::hydrate(
		&HydratedReactiveInput {
			alternate: alternate.clone(),
			value: value.clone(),
			observed: Rc::clone(&observed),
		},
		&root,
	)
	.expect("hydrate");
	assert_eq!(value.get(), "restored");
	assert!(
		raw_input.is_same_node(
			root.as_web_sys()
				.first_element_child()
				.as_ref()
				.map(|element| &**element),
		)
	);

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
	original
		.dispatch_event(&web_sys::Event::new("change").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "restored");
	assert_eq!(&*observed.borrow(), "");

	value.set("fresh".to_owned());
	assert_eq!(original.value(), "stale");
	let fresh_control: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("fresh control")
		.unchecked_into();
	assert_eq!(fresh_control.value(), "fresh");
	fresh_control
		.dispatch_event(&web_sys::Event::new("change").expect("event"))
		.expect("dispatch");
	assert_eq!(&*observed.borrow(), "fresh");
	fresh_control.set_value("new branch");
	fresh_control
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "new branch");
	assert_eq!(&*observed.borrow(), "fresh");
	let live_control: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("live control")
		.unchecked_into();
	assert_eq!(live_control.value(), "new branch");
	reinhardt_pages::cleanup_reactive_nodes();
}

struct HydratedReactiveIfInput {
	alternate: Signal<bool>,
	value: Signal<String>,
	observed: Rc<RefCell<String>>,
}

impl Component for HydratedReactiveIfInput {
	fn name() -> &'static str {
		"HydratedReactiveIfInput"
	}

	fn render(&self) -> Page {
		let condition_alternate = self.alternate.clone();
		let condition_value = self.value.clone();
		let primary_value = self.value.clone();
		let primary_observed = Rc::clone(&self.observed);
		let replacement_value = self.value.clone();
		let replacement_observed = Rc::clone(&self.observed);
		PageElement::new("div")
			.child(Page::reactive_if(
				move || condition_alternate.get() || condition_value.get() == "server",
				move || {
					let bound = primary_value.clone();
					let handler_value = primary_value.clone();
					let handler_observed = Rc::clone(&primary_observed);
					page!({
						input {
							a11y: off,
							id: "primary",
							bind: bound,
							@input: move |_| {
								*handler_observed.borrow_mut() = handler_value.get();
							},
						}
					})
				},
				move || {
					let bound = replacement_value.clone();
					let handler_value = replacement_value.clone();
					let handler_observed = Rc::clone(&replacement_observed);
					page!({
						input {
							a11y: off,
							id: "replacement",
							bind: bound,
							@input: move |_| {
								*handler_observed.borrow_mut() = handler_value.get();
							},
						}
					})
				},
			))
			.into_page()
	}
}

#[wasm_bindgen_test]
fn hydrated_reactive_if_adopts_before_subscribing_and_transfers_guards() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let raw_root = document.create_element("div").expect("root");
	let raw_input = document.create_element("input").expect("input");
	raw_input.set_id("primary");
	let primary: web_sys::HtmlInputElement = raw_input.clone().unchecked_into();
	primary.set_value("restored");
	raw_root.append_child(&raw_input).expect("SSR child");
	let root = Element::new(raw_root);
	let alternate = Signal::new(false);
	let value = Signal::new("server".to_owned());
	let observed = Rc::new(RefCell::new(String::new()));
	let _state = SsrStateElement::install(&document);

	reinhardt_pages::hydration::hydrate(
		&HydratedReactiveIfInput {
			alternate: alternate.clone(),
			value: value.clone(),
			observed: Rc::clone(&observed),
		},
		&root,
	)
	.expect("hydrate");
	assert_eq!(value.get(), "restored");
	assert!(
		raw_input.is_same_node(
			root.as_web_sys()
				.first_element_child()
				.as_ref()
				.map(|element| &**element),
		)
	);
	primary
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(&*observed.borrow(), "restored");

	alternate.set(true);
	let switched: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("switched branch")
		.unchecked_into();
	assert!(!raw_input.is_same_node(Some(&switched)));
	primary.set_value("stale");
	primary
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "restored");
	assert_eq!(&*observed.borrow(), "restored");

	value.set("fresh".to_owned());
	assert_eq!(primary.value(), "stale");
	assert_eq!(switched.value(), "fresh");
	switched.set_value("new branch");
	switched
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "new branch");
	assert_eq!(&*observed.borrow(), "new branch");
	reinhardt_pages::cleanup_reactive_nodes();
}
