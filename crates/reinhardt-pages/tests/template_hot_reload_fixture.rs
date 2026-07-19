use reinhardt_pages::hmr::{
	DynamicAbiHash, DynamicSlotDescriptor, DynamicSlotId, StaticTemplateNode, TemplateDescriptor,
	TemplateKey,
};
use wasm_bindgen::JsCast;

pub(crate) fn key() -> TemplateKey {
	TemplateKey {
		source_id: reinhardt_pages::hmr::SourceId("tests/template.rs".to_owned()),
		line: 1,
		column: 1,
		nested_template_index: 0,
	}
}

pub(crate) fn static_tree(class_name: &str, text: &str) -> StaticTemplateNode {
	StaticTemplateNode::Element {
		tag: "div".to_owned(),
		static_attrs: vec![("class".to_owned(), class_name.to_owned())],
		children: vec![StaticTemplateNode::Text(text.to_owned())],
	}
}

pub(crate) fn descriptor(template: StaticTemplateNode) -> TemplateDescriptor {
	TemplateDescriptor {
		key: key(),
		abi_hash: DynamicAbiHash([7; 32]),
		static_tree: template,
		slots: vec![DynamicSlotDescriptor {
			slot_id: DynamicSlotId(0),
			semantic_kind: "expression".to_owned(),
			canonical_tokens: vec!["count.get()".to_owned()],
		}],
		nested: Vec::new(),
	}
}

pub(crate) fn append_static_tree(
	document: &web_sys::Document,
	parent: &web_sys::Node,
	tree: &StaticTemplateNode,
) -> web_sys::Node {
	let node: web_sys::Node = match tree {
		StaticTemplateNode::Element {
			tag,
			static_attrs,
			children,
		} => {
			let element = document.create_element(tag).expect("static element");
			for (name, value) in static_attrs {
				element
					.set_attribute(name, value)
					.expect("static attribute");
			}
			for child in children {
				append_static_tree(document, &element, child);
			}
			element.unchecked_into()
		}
		StaticTemplateNode::Text(text) => document.create_text_node(text).unchecked_into(),
		StaticTemplateNode::Slot(slot_id) => document
			.create_comment(&format!("hmr-slot-{}", slot_id.0))
			.unchecked_into(),
	};
	parent.append_child(&node).expect("append static tree");
	node
}
