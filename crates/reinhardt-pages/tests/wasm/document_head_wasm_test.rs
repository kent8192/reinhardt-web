#![cfg(wasm)]

use js_sys::Reflect;
use reinhardt_pages::component::{Head, Page, PageExt, ScriptTag, cleanup_reactive_nodes};
use reinhardt_pages::dom::Element;
use reinhardt_pages::reactive::Signal;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

struct DocumentHeadFixture {
	document: web_sys::Document,
	root: web_sys::Element,
	original_title: String,
	extra_head_nodes: Vec<web_sys::Element>,
}

impl DocumentHeadFixture {
	fn new() -> Self {
		cleanup_reactive_nodes();
		let document = web_sys::window().unwrap().document().unwrap();
		let root = document.create_element("div").unwrap();
		document.body().unwrap().append_child(&root).unwrap();

		Self {
			original_title: document.title(),
			document,
			root,
			extra_head_nodes: Vec::new(),
		}
	}

	fn append_head_element(&mut self, tag: &str) -> web_sys::Element {
		let element = self.document.create_element(tag).unwrap();
		self.document
			.head()
			.unwrap()
			.append_child(&element)
			.unwrap();
		self.extra_head_nodes.push(element.clone());
		element
	}

	fn mount(&self, page: Page) {
		page.mount(&Element::new(self.root.clone())).unwrap();
	}
}

impl Drop for DocumentHeadFixture {
	fn drop(&mut self) {
		cleanup_reactive_nodes();
		for node in self.extra_head_nodes.drain(..) {
			node.remove();
		}
		self.root.remove();
		self.document.set_title(&self.original_title);
	}
}

#[wasm_bindgen_test]
fn static_page_head_updates_managed_nodes_without_removing_unmanaged_nodes() {
	// Arrange
	let mut fixture = DocumentHeadFixture::new();
	let third_party = fixture.append_head_element("meta");
	third_party.set_attribute("name", "third-party").unwrap();

	// Act
	fixture.mount(
		Page::text("body").with_head(Head::new().title("Managed").meta_description("managed")),
	);

	// Assert
	assert_eq!(fixture.document.title(), "Managed");
	assert!(
		fixture
			.document
			.query_selector("meta[name='description'][data-reinhardt-head]")
			.unwrap()
			.is_some()
	);
	assert!(
		fixture
			.document
			.head()
			.unwrap()
			.contains(Some(&third_party))
	);
}

#[wasm_bindgen_test]
fn nested_static_pages_follow_structural_order_and_cleanup() {
	// Arrange
	let mut fixture = DocumentHeadFixture::new();
	let original_title = fixture.document.title();
	let page = Page::fragment([Page::text("inner").with_head(Head::new().title("Inner"))])
		.with_head(Head::new().title("Outer"));

	// Act
	fixture.mount(page);

	// Assert
	assert_eq!(fixture.document.title(), "Inner");
	cleanup_reactive_nodes();
	assert_eq!(fixture.document.title(), original_title);
}

#[wasm_bindgen_test]
fn reactive_branch_cleanup_restores_earlier_singletons_then_unmanaged_snapshots() {
	// Arrange
	let mut fixture = DocumentHeadFixture::new();
	fixture.document.set_title("Unmanaged title");
	let unmanaged_title = fixture
		.document
		.query_selector("title")
		.unwrap()
		.expect("document title element");
	let unmanaged_base = fixture.append_head_element("base");
	unmanaged_base.set_attribute("href", "/unmanaged/").unwrap();
	let show_later = Signal::new(true);
	let show_later_for_view = show_later.clone();
	let page = Page::fragment([
		Page::text("earlier").with_head(Head::new().title("Earlier").base_url("/earlier/")),
		Page::reactive(move || {
			if show_later_for_view.get() {
				Page::text("later").with_head(Head::new().title("Later").base_url("/later/"))
			} else {
				Page::empty()
			}
		}),
	]);

	// Act
	fixture.mount(page);

	// Assert
	assert_eq!(fixture.document.title(), "Later");
	assert_eq!(
		fixture
			.document
			.query_selector("base[data-reinhardt-head]")
			.unwrap()
			.unwrap()
			.get_attribute("href")
			.as_deref(),
		Some("/later/")
	);

	show_later.set(false);
	assert_eq!(fixture.document.title(), "Earlier");
	assert_eq!(
		fixture
			.document
			.query_selector("base[data-reinhardt-head]")
			.unwrap()
			.unwrap()
			.get_attribute("href")
			.as_deref(),
		Some("/earlier/")
	);

	cleanup_reactive_nodes();
	let restored_title = fixture
		.document
		.query_selector("title")
		.unwrap()
		.expect("restored title element");
	let restored_base = fixture
		.document
		.query_selector("base")
		.unwrap()
		.expect("restored base element");
	assert_eq!(fixture.document.title(), "Unmanaged title");
	assert!(restored_title.is_same_node(Some(&unmanaged_title)));
	assert_eq!(restored_title.get_attribute("data-reinhardt-head"), None);
	assert!(restored_base.is_same_node(Some(&unmanaged_base)));
	assert_eq!(
		restored_base.get_attribute("href").as_deref(),
		Some("/unmanaged/")
	);
	assert_eq!(restored_base.get_attribute("data-reinhardt-head"), None);
}

#[wasm_bindgen_test]
fn duplicate_script_transfers_representative_without_reexecution() {
	// Arrange
	let fixture = DocumentHeadFixture::new();
	let window = web_sys::window().unwrap();
	let counter_name = JsValue::from_str("__reinhardtHeadScriptRuns");
	Reflect::set(&window, &counter_name, &JsValue::from_f64(0.0)).unwrap();
	let script = ScriptTag::inline(
		"window.__reinhardtHeadScriptRuns = (window.__reinhardtHeadScriptRuns || 0) + 1;",
	);
	let show_first = Signal::new(true);
	let show_first_for_view = show_first.clone();
	let first_script = script.clone();
	let page = Page::fragment([
		Page::reactive(move || {
			if show_first_for_view.get() {
				Page::text("first").with_head(Head::new().script(first_script.clone()))
			} else {
				Page::empty()
			}
		}),
		Page::text("second").with_head(Head::new().script(script)),
	]);

	// Act
	fixture.mount(page);
	let original_script = fixture
		.document
		.query_selector("script[data-reinhardt-head]")
		.unwrap()
		.expect("managed script");

	// Assert
	assert_eq!(
		fixture
			.document
			.query_selector_all("script[data-reinhardt-head]")
			.unwrap()
			.length(),
		1
	);
	assert_eq!(
		Reflect::get(&window, &counter_name)
			.unwrap()
			.as_f64()
			.unwrap(),
		1.0
	);

	show_first.set(false);
	let transferred_script = fixture
		.document
		.query_selector("script[data-reinhardt-head]")
		.unwrap()
		.expect("transferred managed script");
	assert!(transferred_script.is_same_node(Some(&original_script)));
	assert_eq!(
		Reflect::get(&window, &counter_name)
			.unwrap()
			.as_f64()
			.unwrap(),
		1.0
	);

	Reflect::delete_property(&window, &counter_name).unwrap();
}
