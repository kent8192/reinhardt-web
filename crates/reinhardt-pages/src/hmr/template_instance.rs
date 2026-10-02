//! WASM-side mounted template ownership and slot records.

use std::collections::BTreeMap;

use super::protocol::DynamicSlotId;
#[cfg(wasm)]
use wasm_bindgen::JsCast;

/// A DOM range retained by a mounted template instance.
#[cfg(wasm)]
pub struct DomRange {
	/// Stable start marker for the range.
	pub start: web_sys::Comment,
	/// Stable end marker for the range.
	pub end: web_sys::Comment,
	/// Nodes currently contained by the range.
	pub nodes: Vec<web_sys::Node>,
}

#[cfg(wasm)]
impl DomRange {
	/// Returns the nodes currently enclosed by the range anchors.
	///
	/// Reactive conditionals and keyed lists can replace their contents after
	/// the template first mounts. Reading between the anchors at patch time
	/// keeps the retained dynamic range current instead of moving an obsolete
	/// snapshot from the initial render.
	pub(crate) fn current_nodes(&self) -> Vec<web_sys::Node> {
		let end: web_sys::Node = self.end.clone().unchecked_into();
		let mut current = self.start.next_sibling();
		let mut nodes = Vec::new();
		while let Some(node) = current {
			if node.is_same_node(Some(&end)) {
				break;
			}
			current = node.next_sibling();
			nodes.push(node);
		}
		nodes
	}
}

/// Owner handle for reactive resources belonging to a dynamic range.
#[cfg(wasm)]
#[derive(Default)]
pub struct ReactiveOwnerHandle {
	store: Option<crate::component::reactive_if::ReactiveNodeStore>,
}

#[cfg(wasm)]
impl ReactiveOwnerHandle {
	/// Retains an existing reactive-node store until this handle is dropped.
	pub(crate) fn from_store(store: crate::component::reactive_if::ReactiveNodeStore) -> Self {
		Self { store: Some(store) }
	}
}

#[cfg(wasm)]
impl Drop for ReactiveOwnerHandle {
	fn drop(&mut self) {
		if let Some(store) = self.store.take() {
			crate::component::reactive_if::clear_reactive_node_store(&store);
		}
	}
}

/// A dynamic range and the reactive owner that updates it.
#[cfg(wasm)]
pub struct DynamicRange {
	/// DOM anchors and current nodes.
	pub range: DomRange,
	/// RAII owner for reactive effects nested in the range.
	pub owner: ReactiveOwnerHandle,
}

/// Dynamic binding state retained by a bound element.
#[cfg(wasm)]
#[derive(Default)]
pub struct DynamicBindings {
	/// Dynamic attribute names currently owned by the element.
	pub attribute_names: Vec<String>,
}

/// A bound element retained across static-template patches.
#[cfg(wasm)]
pub struct BoundElement {
	/// Element identity that must survive a compatible patch.
	pub element: web_sys::Element,
	/// RAII event listeners attached to the element.
	pub event_handles: Vec<crate::dom::EventHandle>,
	/// Dynamic attributes and control bindings owned by the element.
	pub dynamic_bindings: DynamicBindings,
}

/// A dynamic slot mounted in a template instance.
#[cfg(wasm)]
pub enum MountedSlot {
	/// Reactive content range.
	DynamicRange(DynamicRange),
	/// Retained bound element.
	BoundElement(BoundElement),
}

/// All runtime ownership associated with one mounted template.
#[cfg(wasm)]
pub struct TemplateInstance {
	/// Root range retained by the instance.
	pub root_range: DomRange,
	/// Dynamic slots owned by this instance.
	pub slots: BTreeMap<DynamicSlotId, MountedSlot>,
	/// Guards for nested template instances.
	pub nested: Vec<super::template_registry::RegistrationGuard>,
}
