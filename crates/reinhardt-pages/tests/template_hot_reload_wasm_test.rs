#![cfg(all(target_arch = "wasm32", feature = "hmr"))]

#[path = "template_hot_reload_fixture.rs"]
mod fixture;

use std::{cell::Cell, collections::BTreeMap, rc::Rc};

use reinhardt_pages::hmr::patch_transaction::{DomOperation, PatchTransaction};
use reinhardt_pages::hmr::template_instance::{
	DomRange, DynamicRange, MountedSlot, ReactiveOwnerHandle, TemplateInstance,
};
use reinhardt_pages::hmr::template_registry::TemplateRegistry;
use reinhardt_pages::hmr::{
	CompiledBuildId, DynamicAbiHash, DynamicSlotId, HmrBridge, PatchGeneration, StaticTemplateNode,
	TemplatePatch, TemplatePatchBatch,
};
use reinhardt_pages::{
	component::{Page, PageExt},
	dom::Element,
	page,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn future_overlay_page() -> Page {
	page!(|| {
		div {
			id: "future-overlay-template",
			class: "compiled",
			"Compiled static"
		}
	})()
}

fn static_tree_with_dynamic_input(class_name: &str, text: &str) -> StaticTemplateNode {
	StaticTemplateNode::Element {
		tag: "div".to_owned(),
		static_attrs: vec![("class".to_owned(), class_name.to_owned())],
		children: vec![
			StaticTemplateNode::Text(text.to_owned()),
			StaticTemplateNode::Slot(DynamicSlotId(0)),
		],
	}
}

#[wasm_bindgen_test]
async fn static_patch_replaces_root_and_publishes_future_overlay() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let container = document.create_element("section").expect("container");
	let start = document.create_comment("template-start");
	let old = document.create_element("div").expect("old root");
	old.set_class_name("old");
	old.set_text_content(Some("old text"));
	let end = document.create_comment("template-end");
	container.append_child(&start).expect("start");
	container.append_child(&old).expect("old root");
	container.append_child(&end).expect("end");

	let registry = TemplateRegistry::new();
	registry.set_build_identity(CompiledBuildId([0; 32]), [0; 32]);
	registry.register_descriptor(fixture::descriptor(fixture::static_tree("old", "old text")));
	let instance = TemplateInstance {
		root_range: DomRange {
			start,
			end,
			nodes: vec![old.unchecked_into()],
		},
		slots: BTreeMap::new(),
		nested: Vec::new(),
	};
	let guard = registry.mount_instance(fixture::key(), instance);
	let mutation_seen = Rc::new(Cell::new(false));
	let mutation_seen_callback = Rc::clone(&mutation_seen);
	let callback = Closure::wrap(
		Box::new(move |_: js_sys::Array, _: web_sys::MutationObserver| {
			mutation_seen_callback.set(true);
		}) as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>,
	);
	let observer = web_sys::MutationObserver::new(callback.as_ref().unchecked_ref())
		.expect("mutation observer");
	let options = web_sys::MutationObserverInit::new();
	options.set_child_list(true);
	options.set_subtree(true);
	observer
		.observe_with_options(&container, &options)
		.expect("observe template root");

	let batch = TemplatePatchBatch {
		build_id: CompiledBuildId([0; 32]),
		manifest_digest: [0; 32],
		generation: PatchGeneration(1),
		patches: vec![TemplatePatch {
			key: fixture::key(),
			abi_hash: DynamicAbiHash([7; 32]),
			static_tree: fixture::static_tree("new", "new text"),
			placements: Vec::new(),
		}],
	};

	let plan = PatchTransaction::plan(&batch, &registry).expect("plan");
	PatchTransaction::commit(plan).expect("commit");
	JsFuture::from(js_sys::Promise::resolve(&JsValue::UNDEFINED))
		.await
		.expect("mutation microtask");
	assert!(mutation_seen.get(), "patch must be browser-observable");

	let replacement = container
		.children()
		.item(0)
		.expect("replacement")
		.unchecked_into::<web_sys::Element>();
	assert_eq!(replacement.tag_name(), "DIV");
	assert_eq!(replacement.class_name(), "new");
	assert_eq!(replacement.text_content().as_deref(), Some("new text"));
	assert_eq!(
		registry.overlay_for(&fixture::key()),
		Some(fixture::static_tree("new", "new text"))
	);
	assert_eq!(registry.instance_count(), 1);

	drop(guard);
	assert_eq!(registry.instance_count(), 0);
	container.remove();
	drop(observer);
	drop(callback);
}

#[wasm_bindgen_test]
fn static_patch_preserves_dynamic_input_identity_and_value() {
	// Arrange
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let container = document.create_element("section").expect("container");
	let template_start = document.create_comment("template-start");
	let root = document.create_element("div").expect("template root");
	root.set_class_name("old");
	root.append_child(&document.create_text_node("old text"))
		.expect("old text");
	let slot_start = document.create_comment("slot-start");
	let input = document
		.create_element("input")
		.expect("input")
		.unchecked_into::<web_sys::HtmlInputElement>();
	input.set_value("retained user value");
	let slot_end = document.create_comment("slot-end");
	root.append_child(&slot_start).expect("slot start");
	root.append_child(&input).expect("input");
	root.append_child(&slot_end).expect("slot end");
	let template_end = document.create_comment("template-end");
	container
		.append_child(&template_start)
		.expect("template start");
	container.append_child(&root).expect("template root");
	container.append_child(&template_end).expect("template end");

	let registry = TemplateRegistry::new();
	registry.register_descriptor(fixture::descriptor(static_tree_with_dynamic_input(
		"old", "old text",
	)));
	let input_node: web_sys::Node = input.clone().unchecked_into();
	let mut slots = BTreeMap::new();
	slots.insert(
		DynamicSlotId(0),
		MountedSlot::DynamicRange(DynamicRange {
			range: DomRange {
				start: slot_start,
				end: slot_end,
				nodes: vec![input_node],
			},
			owner: ReactiveOwnerHandle::default(),
		}),
	);
	let guard = registry.mount_instance(
		fixture::key(),
		TemplateInstance {
			root_range: DomRange {
				start: template_start,
				end: template_end,
				nodes: vec![root.unchecked_into()],
			},
			slots,
			nested: Vec::new(),
		},
	);
	let batch = TemplatePatchBatch {
		build_id: CompiledBuildId([0; 32]),
		manifest_digest: [0; 32],
		generation: PatchGeneration(1),
		patches: vec![TemplatePatch::static_replacement(
			&fixture::descriptor(static_tree_with_dynamic_input("old", "old text")),
			static_tree_with_dynamic_input("new", "new text"),
		)],
	};

	// Act
	let plan = PatchTransaction::plan(&batch, &registry).expect("plan");
	PatchTransaction::commit(plan).expect("commit");

	// Assert
	let replacement = container
		.children()
		.item(0)
		.expect("replacement")
		.unchecked_into::<web_sys::Element>();
	let retained_input = replacement
		.query_selector("input")
		.expect("input query")
		.expect("retained input")
		.unchecked_into::<web_sys::HtmlInputElement>();
	assert!(retained_input.is_same_node(Some(&input)));
	assert_eq!(retained_input.value(), "retained user value");
	assert_eq!(replacement.class_name(), "new");
	assert_eq!(replacement.text_content().as_deref(), Some("new text"));

	// Cleanup
	drop(guard);
	container.remove();
}

#[wasm_bindgen_test]
fn failed_commit_rolls_back_completed_operations() {
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let container = document.create_element("section").expect("container");
	let start = document.create_comment("template-start");
	let old = document.create_element("div").expect("old root");
	let end = document.create_comment("template-end");
	container.append_child(&start).expect("start");
	container.append_child(&old).expect("old root");
	container.append_child(&end).expect("end");

	let registry = TemplateRegistry::new();
	registry.register_descriptor(fixture::descriptor(fixture::static_tree("old", "old text")));
	let instance = TemplateInstance {
		root_range: DomRange {
			start,
			end,
			nodes: vec![old.clone().unchecked_into()],
		},
		slots: BTreeMap::new(),
		nested: Vec::new(),
	};
	let _guard = registry.mount_instance(fixture::key(), instance);
	let batch = TemplatePatchBatch {
		build_id: CompiledBuildId([0; 32]),
		manifest_digest: [0; 32],
		generation: PatchGeneration(1),
		patches: vec![TemplatePatch {
			key: fixture::key(),
			abi_hash: DynamicAbiHash([7; 32]),
			static_tree: fixture::static_tree("new", "new text"),
			placements: Vec::new(),
		}],
	};
	let mut plan = PatchTransaction::plan(&batch, &registry).expect("plan");
	let invalid_before = document.create_element("aside").expect("foreign before");
	plan.operations.push(DomOperation::MoveNode {
		node: document.create_text_node("invalid").unchecked_into(),
		parent: container.clone().unchecked_into(),
		before: Some(invalid_before.unchecked_into()),
	});

	assert!(PatchTransaction::commit(plan).is_err());
	assert!(old.parent_node().is_some());
	assert_eq!(old.class_name(), "");
	assert_eq!(registry.overlay_for(&fixture::key()), None);
	container.remove();
}

#[wasm_bindgen_test]
fn bridge_identity_and_future_mount_overlay_are_stable() {
	let registry = TemplateRegistry::new();
	registry.set_build_identity(CompiledBuildId([9; 32]), [8; 32]);
	let original = fixture::static_tree("old", "old text");
	registry.register_descriptor(fixture::descriptor(original.clone()));

	let hello = registry.client_hello();
	assert_eq!(hello.build_id, CompiledBuildId([9; 32]));
	assert_eq!(hello.manifest_digest, [8; 32]);
	assert_eq!(
		hello.abi_hashes,
		vec![(fixture::key(), DynamicAbiHash([7; 32]))]
	);
	assert_eq!(registry.static_tree_for(&fixture::key()), Some(original));

	let overlay = fixture::static_tree("new", "new text");
	registry
		.apply_overlay(&fixture::key(), overlay.clone(), PatchGeneration(4))
		.expect("future overlay");
	assert_eq!(registry.static_tree_for(&fixture::key()), Some(overlay));
	assert_eq!(
		registry.static_tree_for(&fixture::key()),
		registry.overlay_for(&fixture::key())
	);
	assert_eq!(registry.instance_count(), 0);
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let host = document.create_element("section").expect("future host");
	let future_tree = registry
		.static_tree_for(&fixture::key())
		.expect("future tree");
	let future_root = fixture::append_static_tree(&document, host.as_ref(), &future_tree)
		.unchecked_into::<web_sys::Element>();
	assert_eq!(future_root.class_name(), "new");
	assert_eq!(future_root.text_content().as_deref(), Some("new text"));
	host.remove();
}

#[wasm_bindgen_test]
fn page_mount_replays_the_latest_overlay_for_future_instances() {
	// Arrange
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let bridge = HmrBridge::new();
	bridge.install(&document).expect("install HMR bridge");
	let first_host = Element::new(document.create_element("section").expect("first host"));
	let first_page = future_overlay_page();
	let descriptor = first_page
		.dev_template_metadata()
		.and_then(|metadata| metadata.downcast_ref().cloned())
		.expect("page macro descriptor");
	first_page.mount(&first_host).expect("mount first template");
	assert_eq!(bridge.registry().instance_count(), 1);

	let hello = bridge.registry().client_hello();
	let patched_tree = StaticTemplateNode::Element {
		tag: "div".to_owned(),
		static_attrs: vec![
			("id".to_owned(), "future-overlay-template".to_owned()),
			("class".to_owned(), "patched".to_owned()),
		],
		children: vec![StaticTemplateNode::Text("Patched static".to_owned())],
	};
	let batch = TemplatePatchBatch {
		build_id: hello.build_id,
		manifest_digest: hello.manifest_digest,
		generation: PatchGeneration(1),
		patches: vec![TemplatePatch::static_replacement(&descriptor, patched_tree)],
	};

	// Act
	let plan = PatchTransaction::plan(&batch, bridge.registry()).expect("plan first patch");
	PatchTransaction::commit(plan).expect("commit first patch");
	let second_host = Element::new(document.create_element("section").expect("second host"));
	future_overlay_page()
		.mount(&second_host)
		.expect("mount future template");

	// Assert
	let first = first_host
		.as_web_sys()
		.query_selector("#future-overlay-template")
		.expect("first query")
		.expect("first template root");
	let second = second_host
		.as_web_sys()
		.query_selector("#future-overlay-template")
		.expect("second query")
		.expect("future template root");
	assert_eq!(first.class_name(), "patched");
	assert_eq!(second.class_name(), "patched");
	assert_eq!(second.text_content().as_deref(), Some("Patched static"));
	assert_eq!(bridge.registry().instance_count(), 2);

	// Cleanup
	reinhardt_pages::cleanup_reactive_nodes();
	first_host.as_web_sys().remove();
	second_host.as_web_sys().remove();
}

#[wasm_bindgen_test]
async fn bridge_defers_an_unmounted_template_patch_until_its_first_mount() {
	// Arrange
	let document = web_sys::window()
		.expect("window")
		.document()
		.expect("document");
	let bridge = HmrBridge::new();
	bridge.install(&document).expect("install HMR bridge");
	let seed = future_overlay_page();
	let descriptor = seed
		.dev_template_metadata()
		.and_then(|metadata| metadata.downcast_ref().cloned())
		.expect("page macro descriptor");
	let batch = TemplatePatchBatch {
		build_id: CompiledBuildId([1; 32]),
		manifest_digest: [2; 32],
		generation: PatchGeneration(3),
		patches: vec![TemplatePatch::static_replacement(
			&descriptor,
			StaticTemplateNode::Element {
				tag: "div".to_owned(),
				static_attrs: vec![
					("id".to_owned(), "future-overlay-template".to_owned()),
					("class".to_owned(), "deferred".to_owned()),
				],
				children: vec![StaticTemplateNode::Text("Deferred static".to_owned())],
			},
		)],
	};
	let patch_applier = js_sys::Reflect::get(
		&js_sys::global(),
		&JsValue::from_str("__REINHARDT_HMR_PATCH_APPLIER__"),
	)
	.expect("patch applier")
	.dyn_into::<js_sys::Function>()
	.expect("patch applier function");
	let payload = serde_wasm_bindgen::to_value(&batch).expect("serialize patch");

	// Act
	let promise = patch_applier
		.call1(&js_sys::global(), &payload)
		.expect("apply deferred patch")
		.dyn_into::<js_sys::Promise>()
		.expect("patch promise");
	JsFuture::from(promise).await.expect("defer patch");
	assert_eq!(bridge.registry().instance_count(), 0);
	let host = Element::new(document.create_element("section").expect("future host"));
	future_overlay_page()
		.mount(&host)
		.expect("mount deferred template");

	// Assert
	let root = host
		.as_web_sys()
		.query_selector("#future-overlay-template")
		.expect("query deferred template")
		.expect("deferred template root");
	assert_eq!(root.class_name(), "deferred");
	assert_eq!(root.text_content().as_deref(), Some("Deferred static"));

	// Cleanup
	reinhardt_pages::cleanup_reactive_nodes();
	host.as_web_sys().remove();
}
