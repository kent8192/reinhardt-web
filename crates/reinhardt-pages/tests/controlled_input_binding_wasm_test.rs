#![cfg(wasm)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use reinhardt_pages::component::{
	Component, ControlBinding, ControlBindingError, ControlKind, IntoPage, MountError, Page,
	PageExt,
};
use reinhardt_pages::dom::Element;
use reinhardt_pages::reactive::{Signal, with_runtime};
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
fn public_page_mount_applies_initial_select_one_after_mounting_options() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let selected = Signal::new("wasm".to_owned());
	page!({
		select {
			a11y: off,
			bind: selected,
			option {
				value: "rust",
				"Rust"
			}
			option {
				value: "wasm",
				"WebAssembly"
			}
		}
	})
	.mount(&root)
	.expect("mount");
	let select: web_sys::HtmlSelectElement = root
		.as_web_sys()
		.first_element_child()
		.expect("select")
		.unchecked_into();

	assert_eq!(select.value(), "wasm");
	reinhardt_pages::cleanup_reactive_nodes();
}

#[wasm_bindgen_test]
fn public_page_mount_applies_initial_select_many_after_mounting_options() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let selected = Signal::new(vec!["rust".to_owned(), "wasm".to_owned()]);
	page!({
		select {
			a11y: off,
			multiple: true,
			bind: selected,
			option {
				value: "rust",
				"Rust"
			}
			option {
				value: "wasm",
				"WebAssembly"
			}
		}
	})
	.mount(&root)
	.expect("mount");
	let select: web_sys::HtmlSelectElement = root
		.as_web_sys()
		.first_element_child()
		.expect("select")
		.unchecked_into();

	let rust: web_sys::HtmlOptionElement = select.item(0).expect("rust option").unchecked_into();
	let wasm: web_sys::HtmlOptionElement = select.item(1).expect("wasm option").unchecked_into();
	assert_eq!(select.selected_options().length(), 2);
	assert!(rust.selected());
	assert!(wasm.selected());
	reinhardt_pages::cleanup_reactive_nodes();
}

#[wasm_bindgen_test]
fn reactive_select_remount_applies_binding_after_mounting_replacement_options() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let alternate = Signal::new(false);
	let selected = Signal::new("wasm".to_owned());
	let render_alternate = alternate.clone();
	let render_selected = selected.clone();
	Page::reactive(move || {
		let bound = render_selected.clone();
		let id = if render_alternate.get() {
			"replacement"
		} else {
			"original"
		};
		page!({
			select {
				a11y: off,
				id: id,
				bind: bound,
				option {
					value: "rust",
					"Rust"
				}
				option {
					value: "wasm",
					"WebAssembly"
				}
			}
		})
	})
	.mount(&root)
	.expect("mount");

	alternate.set(true);
	let replacement: web_sys::HtmlSelectElement = root
		.as_web_sys()
		.first_element_child()
		.expect("replacement")
		.unchecked_into();
	assert_eq!(replacement.id(), "replacement");
	assert_eq!(replacement.value(), "wasm");
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
fn failed_select_mount_rolls_back_child_reactive_resources() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let parent_checked = Signal::new(false);
	let child_value = Signal::new("initial".to_owned());
	let render_count = Rc::new(std::cell::Cell::new(0));
	let listener_owner = Rc::new(());
	let weak_listener_owner = Rc::downgrade(&listener_owner);
	let render_value = child_value.clone();
	let render_count_for_child = Rc::clone(&render_count);
	let listener_owner_for_child = Rc::clone(&listener_owner);
	let page = Page::Element(
		PageElement::new("select")
			.control_binding(ControlBinding::checkbox(parent_checked))
			.child(Page::reactive(move || {
				render_count_for_child.set(render_count_for_child.get() + 1);
				let _ = render_value.get();
				let bound = render_value.clone();
				let listener_owner = Rc::clone(&listener_owner_for_child);
				page!({
					input {
						a11y: off,
						bind: bound,
						@input: move |_| drop(Rc::clone(&listener_owner)),
					}
				})
			})),
	);
	drop(listener_owner);

	let error = page.mount(&root).expect_err("parent binding mismatch");

	assert_eq!(
		error,
		MountError::ControlBinding(ControlBindingError::UnsupportedElement {
			control: ControlKind::Checkbox,
			actual_tag: "select".to_owned(),
		})
	);
	assert_eq!(render_count.get(), 1);
	assert!(weak_listener_owner.upgrade().is_none());
	assert_eq!(root.as_web_sys().first_element_child(), None);
	child_value.set("after failure".to_owned());
	assert_eq!(render_count.get(), 1);
}

#[wasm_bindgen_test]
fn reactive_failed_select_mount_rolls_back_child_reactive_resources() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let trigger = Signal::new(0_u32);
	let render_count = Rc::new(std::cell::Cell::new(0));
	let last_listener_owner = Rc::new(RefCell::new(None));
	let render_trigger = trigger.clone();
	let render_count_for_page = Rc::clone(&render_count);
	let last_listener_owner_for_page = Rc::clone(&last_listener_owner);
	let page = Page::reactive(move || {
		let _ = render_trigger.get();
		render_count_for_page.set(render_count_for_page.get() + 1);
		let listener_owner = Rc::new(());
		*last_listener_owner_for_page.borrow_mut() = Some(Rc::downgrade(&listener_owner));
		Page::Element(
			PageElement::new("select")
				.control_binding(ControlBinding::checkbox(Signal::new(false)))
				.child(page!({
					input {
						a11y: off,
						bind: Signal::new(String::new()),
						@input: move |_| drop(Rc::clone(&listener_owner)),
					}
				})),
		)
	});

	page.mount(&root).expect("reactive owner mount");

	assert_eq!(
		root.as_web_sys().query_selector("select").expect("query"),
		None
	);
	assert_eq!(render_count.get(), 1);
	assert!(
		last_listener_owner
			.borrow()
			.as_ref()
			.expect("owner observation")
			.upgrade()
			.is_none()
	);
	trigger.set(1);
	assert_eq!(render_count.get(), 2);
	assert!(
		last_listener_owner
			.borrow()
			.as_ref()
			.expect("rerendered owner observation")
			.upgrade()
			.is_none()
	);
	reinhardt_pages::cleanup_reactive_nodes();
}

#[wasm_bindgen_test]
fn reactive_invalid_nonselect_binding_is_omitted() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let page = Page::reactive(|| {
		PageElement::new("input")
			.attr("id", "invalid-nonselect")
			.control_binding(ControlBinding::select_one(Signal::new(String::new())))
			.into_page()
	});

	page.mount(&root).expect("reactive owner mount");

	assert_eq!(
		root.as_web_sys()
			.query_selector("#invalid-nonselect")
			.expect("query"),
		None
	);
	reinhardt_pages::cleanup_reactive_nodes();
}

#[wasm_bindgen_test]
fn reactive_nonselect_mount_keeps_parent_when_a_child_mount_fails() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let retained_trigger = Signal::new(0_u32);
	let retained_render_count = Rc::new(Cell::new(0));
	let failed_trigger = Signal::new(0_u32);
	let failed_render_count = Rc::new(Cell::new(0));
	let listener_owner = Rc::new(RefCell::new(None));
	let retained_trigger_for_child = retained_trigger.clone();
	let retained_render_count_for_child = Rc::clone(&retained_render_count);
	let failed_trigger_for_child = failed_trigger.clone();
	let failed_render_count_for_child = Rc::clone(&failed_render_count);
	let listener_owner_for_child = Rc::clone(&listener_owner);
	let page = Page::reactive(move || {
		let retained_trigger = retained_trigger_for_child.clone();
		let retained_render_count = Rc::clone(&retained_render_count_for_child);
		let failed_trigger = failed_trigger_for_child.clone();
		let failed_render_count = Rc::clone(&failed_render_count_for_child);
		let listener_owner_for_child = Rc::clone(&listener_owner_for_child);
		Page::Element(
			PageElement::new("div")
				.attr("id", "retained-parent")
				.child(Page::Element(
					PageElement::new("section")
						.attr("id", "retained-child")
						.child(Page::reactive(move || {
							let value = retained_trigger.get();
							retained_render_count.set(retained_render_count.get() + 1);
							PageElement::new("span")
								.attr("id", "retained-reactive")
								.child(value.to_string())
								.into_page()
						}))
						.child(Page::Element(
							PageElement::new("select")
								.control_binding(ControlBinding::checkbox(Signal::new(false)))
								.child(Page::reactive(move || {
									let _ = failed_trigger.get();
									failed_render_count.set(failed_render_count.get() + 1);
									let owner = Rc::new(());
									*listener_owner_for_child.borrow_mut() =
										Some(Rc::downgrade(&owner));
									page!({
										input {
											a11y: off,
											bind: Signal::new(String::new()),
											@input: move |_| drop(Rc::clone(&owner)),
										}
									})
								})),
						))
						.child(
							PageElement::new("span")
								.attr("id", "nested-valid-sibling")
								.child("nested ready"),
						),
				))
				.child(Page::Element(
					PageElement::new("span")
						.attr("id", "valid-sibling")
						.child(Page::Text("ready".into())),
				)),
		)
	});

	page.mount(&root).expect("reactive owner mount");

	let parent = root
		.as_web_sys()
		.query_selector("#retained-parent")
		.expect("query")
		.expect("non-select parent should remain mounted");
	assert_eq!(
		parent
			.query_selector("#retained-child")
			.expect("retained child query")
			.is_some(),
		true
	);
	assert_eq!(
		parent
			.query_selector("#nested-valid-sibling")
			.expect("nested sibling query")
			.expect("nested sibling")
			.text_content(),
		Some("nested ready".to_owned())
	);
	assert_eq!(
		parent
			.query_selector("#valid-sibling")
			.expect("sibling query")
			.expect("sibling")
			.text_content(),
		Some("ready".to_owned())
	);
	assert_eq!(
		(retained_render_count.get(), failed_render_count.get()),
		(1, 1)
	);
	assert!(
		listener_owner
			.borrow()
			.as_ref()
			.expect("owner observation")
			.upgrade()
			.is_none()
	);
	retained_trigger.set(1);
	failed_trigger.set(1);
	assert_eq!(retained_render_count.get(), 2);
	assert_eq!(failed_render_count.get(), 1);
	assert_eq!(
		parent
			.query_selector("#retained-reactive")
			.expect("reactive query")
			.expect("retained reactive")
			.text_content(),
		Some("1".to_owned())
	);
	reinhardt_pages::cleanup_reactive_nodes();
}

#[wasm_bindgen_test]
fn duplicate_final_input_reprojects_a_reentrant_compositionend_signal_change() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let root = Element::new(document.create_element("div").expect("root"));
	let value = Signal::new("old".to_owned());
	let end_value = value.clone();
	page!({
		input {
			a11y: off,
			bind: value,
			@compositionend: move |_| end_value.set("after-end".to_owned()),
		}
	})
	.mount(&root)
	.expect("mount");
	let input: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("input")
		.unchecked_into();
	input
		.dispatch_event(&web_sys::CompositionEvent::new("compositionstart").expect("start"))
		.expect("dispatch");
	input.set_value("かな");
	input
		.dispatch_event(&web_sys::CompositionEvent::new("compositionend").expect("end"))
		.expect("dispatch");
	assert_eq!(value.get(), "after-end");
	input.set_value("かな");

	input
		.dispatch_event(&web_sys::InputEvent::new("input").expect("input"))
		.expect("dispatch");

	assert_eq!(value.get(), "after-end");
	assert_eq!(input.value(), "after-end");
	reinhardt_pages::cleanup_reactive_nodes();
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

struct FailingHydrationRoot {
	trigger: Signal<u32>,
	render_count: Rc<Cell<u32>>,
	listener_count: Rc<Cell<u32>>,
}

impl Component for FailingHydrationRoot {
	fn name() -> &'static str {
		"FailingHydrationRoot"
	}

	fn render(&self) -> Page {
		let trigger = self.trigger.clone();
		let render_count = Rc::clone(&self.render_count);
		let listener_count = Rc::clone(&self.listener_count);
		PageElement::new("div")
			.child(Page::reactive(move || {
				let _ = trigger.get();
				render_count.set(render_count.get() + 1);
				let listener_count = Rc::clone(&listener_count);
				page!({
					button {
						id: "reactive-sibling",
						@input: move |_| listener_count.set(listener_count.get() + 1),
						"ready"
					}
				})
			}))
			.child(Page::Element(
				PageElement::new("select")
					.control_binding(ControlBinding::checkbox(Signal::new(false))),
			))
			.into_page()
	}
}

#[wasm_bindgen_test]
fn failed_root_hydration_rolls_back_earlier_reactive_siblings() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let raw_root = document.create_element("div").expect("root");
	let raw_button = document.create_element("button").expect("button");
	raw_button.set_id("reactive-sibling");
	raw_button.set_text_content(Some("ready"));
	raw_root
		.append_child(&raw_button)
		.expect("reactive sibling");
	let raw_select = document.create_element("select").expect("select");
	raw_root.append_child(&raw_select).expect("invalid sibling");
	let root = Element::new(raw_root.clone());
	let trigger = Signal::new(0_u32);
	let render_count = Rc::new(Cell::new(0));
	let listener_count = Rc::new(Cell::new(0));
	let _state = SsrStateElement::install(&document);

	let error = reinhardt_pages::hydration::hydrate(
		&FailingHydrationRoot {
			trigger: trigger.clone(),
			render_count: Rc::clone(&render_count),
			listener_count: Rc::clone(&listener_count),
		},
		&root,
	)
	.expect_err("later invalid binding");
	let renders_after_failure = render_count.get();
	raw_button
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	trigger.set(1);
	with_runtime(|runtime| runtime.flush_updates());
	let marker_count = (0..raw_root.child_nodes().length())
		.filter_map(|index| raw_root.child_nodes().item(index))
		.filter(|node| node.node_type() == web_sys::Node::COMMENT_NODE)
		.count();

	assert_eq!(
		error.to_string(),
		"Event attachment failed: checkbox control does not support a <select> element"
	);
	assert_eq!(
		(render_count.get(), listener_count.get(), marker_count),
		(renders_after_failure, 0, 0)
	);
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
	assert!(raw_input.is_same_node(root.as_web_sys().first_element_child().as_deref(),));

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
	assert!(raw_input.is_same_node(root.as_web_sys().first_element_child().as_deref(),));
	with_runtime(|runtime| runtime.flush_updates());
	let converged: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("converged false branch")
		.unchecked_into();
	assert_eq!(converged.id(), "replacement");
	assert!(!raw_input.is_same_node(Some(&converged)));
	primary.set_value("stale after convergence");
	primary
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "restored");
	assert_eq!(&*observed.borrow(), "");

	alternate.set(true);
	let switched: web_sys::HtmlInputElement = root
		.as_web_sys()
		.first_element_child()
		.expect("switched branch")
		.unchecked_into();
	assert_eq!(switched.id(), "primary");
	converged.set_value("stale");
	converged
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "restored");
	assert_eq!(&*observed.borrow(), "");

	value.set("fresh".to_owned());
	assert_eq!(primary.value(), "stale after convergence");
	assert_eq!(switched.value(), "fresh");
	switched.set_value("new branch");
	switched
		.dispatch_event(&web_sys::InputEvent::new("input").expect("event"))
		.expect("dispatch");
	assert_eq!(value.get(), "new branch");
	assert_eq!(&*observed.borrow(), "new branch");
	reinhardt_pages::cleanup_reactive_nodes();
}
