//! Transactional construction and application of WASM template patches.

use std::{
	collections::{BTreeMap, BTreeSet},
	fmt,
};

use wasm_bindgen::JsCast;

use super::{
	protocol::{
		DynamicSlotId, PatchGeneration, SlotPlacement, StaticTemplateNode, TemplateKey,
		TemplatePatch, TemplatePatchBatch,
	},
	template_instance::{BoundElement, DynamicRange, MountedSlot, TemplateInstance},
	template_registry::TemplateRegistry,
};

/// Plans and commits a compatible static-template replacement.
pub struct PatchTransaction;

impl PatchTransaction {
	/// Builds a detached DOM plan without mutating the live document.
	pub fn plan(
		batch: &TemplatePatchBatch,
		registry: &TemplateRegistry,
	) -> Result<PatchPlan, PatchError> {
		let (_, _, latest_generation) = registry.identity();
		// A browser registers descriptors lazily as routes and reactive branches
		// mount. Its handshake therefore cannot reliably contain the watcher’s
		// whole-source manifest. Per-template key and ABI validation below is the
		// compatibility boundary; build identity remains wire metadata for replay
		// and diagnostics, not a reason to reject an otherwise safe patch.
		if batch.generation < latest_generation {
			return Err(PatchError::GenerationMismatch);
		}

		let mut operations = Vec::new();
		let mut undo = Vec::new();
		let mut overlays = Vec::new();
		let mut seen_keys = BTreeSet::new();

		for patch in &batch.patches {
			if !seen_keys.insert(patch.key.clone()) {
				return Err(PatchError::PlanningFailed(format!(
					"duplicate patch for template {:?}",
					patch.key
				)));
			}
			let descriptor = registry
				.descriptor_for(&patch.key)
				.ok_or_else(|| PatchError::UnknownTemplate(patch.key.clone()))?;
			if descriptor.abi_hash != patch.abi_hash {
				return Err(PatchError::PlanningFailed(format!(
					"dynamic ABI mismatch for template {:?}",
					patch.key
				)));
			}
			validate_placements(&patch.static_tree, &patch.placements)?;

			let mut instance_error = None;
			registry.for_each_instance(&patch.key, |instance| {
				if instance_error.is_some() {
					return;
				}
				if let Err(error) = plan_instance(patch, instance, &mut operations, &mut undo) {
					instance_error = Some(error);
				}
			});
			if let Some(error) = instance_error {
				return Err(error);
			}
			overlays.push((
				patch.key.clone(),
				patch.static_tree.clone(),
				batch.generation,
			));
		}

		Ok(PatchPlan {
			generation: batch.generation,
			operations,
			undo,
			registry: registry.clone(),
			overlays,
		})
	}

	/// Applies every operation and rolls back all completed operations on failure.
	pub fn commit(plan: PatchPlan) -> Result<(), PatchError> {
		for (index, operation) in plan.operations.iter().enumerate() {
			if let Err(error) = apply_operation(operation) {
				for undo in plan.undo.iter().take(index).rev() {
					if let Err(rollback_error) = apply_undo(undo) {
						return Err(PatchError::RollbackFailed(format!(
							"{error}; rollback failed: {rollback_error}"
						)));
					}
				}
				return Err(PatchError::CommitFailed(error));
			}
		}

		plan.registry
			.publish_overlays(&plan.overlays, plan.generation);
		Ok(())
	}
}

/// Detached DOM operations that make up a patch plan.
pub enum DomOperation {
	/// Moves a retained node into a detached skeleton or the live root.
	MoveNode {
		/// Node whose identity is retained.
		node: web_sys::Node,
		/// Destination parent node.
		parent: web_sys::Node,
		/// Optional destination sibling inserted after the moved node.
		before: Option<web_sys::Node>,
	},
	/// Sets a static attribute.
	SetAttribute {
		/// Element receiving the attribute.
		element: web_sys::Element,
		/// Attribute name.
		name: String,
		/// Replacement attribute value.
		value: String,
	},
	/// Removes a static attribute.
	RemoveAttribute {
		/// Element losing the attribute.
		element: web_sys::Element,
		/// Attribute name.
		name: String,
	},
	/// Replaces one root node with another.
	ReplaceStaticRoot {
		/// Existing static root.
		old: web_sys::Node,
		/// Detached replacement root.
		new: web_sys::Node,
	},
	/// Removes a stale root node after the replacement is attached.
	RemoveNode {
		/// Stale node to detach.
		node: web_sys::Node,
	},
}

/// Inverse operations used to restore the DOM after a failed commit.
pub enum UndoOperation {
	/// Restores a node to its original parent and sibling position.
	RestoreNode {
		/// Node to restore.
		node: web_sys::Node,
		/// Original parent node.
		parent: web_sys::Node,
		/// Original next sibling, if any.
		before: Option<web_sys::Node>,
	},
	/// Restores an attribute's previous value.
	RestoreAttribute {
		/// Element receiving the restored attribute.
		element: web_sys::Element,
		/// Attribute name.
		name: String,
		/// Previous value, or `None` when it was absent.
		value: Option<String>,
	},
	/// Restores the old root in place of the new root.
	RestoreRoot {
		/// Original root node.
		old: web_sys::Node,
		/// Replacement root to remove.
		new: web_sys::Node,
	},
}

/// A fully detached patch plan.
pub struct PatchPlan {
	/// Source generation represented by this plan.
	pub generation: PatchGeneration,
	/// Forward DOM operations.
	pub operations: Vec<DomOperation>,
	/// Inverse operations in forward order.
	pub undo: Vec<UndoOperation>,
	registry: TemplateRegistry,
	overlays: Vec<(TemplateKey, StaticTemplateNode, PatchGeneration)>,
}

/// Failure while constructing or applying a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
	/// The patch was compiled for another WASM build.
	BuildMismatch,
	/// The patch was compiled from another template manifest.
	ManifestMismatch,
	/// The patch generation is stale.
	GenerationMismatch,
	/// The template is absent from the client registry.
	UnknownTemplate(TemplateKey),
	/// A dynamic slot is not present in the mounted instance.
	MissingSlot(DynamicSlotId),
	/// A retained bound element cannot be placed in the replacement tree.
	BoundElementTagMismatch,
	/// The detached plan could not be constructed.
	PlanningFailed(String),
	/// A DOM operation failed during commit.
	CommitFailed(String),
	/// An inverse operation failed while rolling back a commit.
	RollbackFailed(String),
}

impl fmt::Display for PatchError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for PatchError {}

struct DetachedSkeleton {
	root_nodes: Vec<web_sys::Node>,
	placements: BTreeMap<DynamicSlotId, PlacementTarget>,
	expected_paths: BTreeMap<DynamicSlotId, Vec<u32>>,
}

struct PlacementTarget {
	parent: web_sys::Node,
	before: Option<web_sys::Node>,
}

fn plan_instance(
	patch: &TemplatePatch,
	instance: &TemplateInstance,
	operations: &mut Vec<DomOperation>,
	undo: &mut Vec<UndoOperation>,
) -> Result<(), PatchError> {
	let old_nodes = instance.root_range.current_nodes();
	let old_parent = old_nodes
		.first()
		.and_then(web_sys::Node::parent_node)
		.or_else(|| instance.root_range.start.parent_node())
		.ok_or_else(|| {
			PatchError::PlanningFailed("mounted template has no root parent".to_owned())
		})?;
	if old_nodes
		.iter()
		.filter_map(web_sys::Node::parent_node)
		.any(|parent| !parent.is_same_node(Some(&old_parent)))
	{
		return Err(PatchError::PlanningFailed(
			"mounted template root nodes have different parents".to_owned(),
		));
	}

	let skeleton = build_skeleton(&patch.static_tree)?;
	let actual_paths = patch
		.placements
		.iter()
		.map(|placement| (placement.slot_id, placement.path.clone()))
		.collect::<BTreeMap<_, _>>();
	if skeleton.expected_paths != actual_paths {
		return Err(PatchError::PlanningFailed(
			"detached skeleton paths do not match patch placements".to_owned(),
		));
	}
	let mut retained_root_nodes = Vec::new();
	for placement in patch.placements.iter().rev() {
		let target = skeleton.placements.get(&placement.slot_id).ok_or_else(|| {
			PatchError::PlanningFailed(format!(
				"slot {:?} has no detached placement",
				placement.slot_id
			))
		})?;
		let mounted_slot = instance
			.slots
			.get(&placement.slot_id)
			.ok_or(PatchError::MissingSlot(placement.slot_id))?;
		let slot_nodes = stage_slot(mounted_slot, target, operations, undo)?;
		if matches!(patch.static_tree, StaticTemplateNode::Slot(slot) if slot == placement.slot_id)
		{
			retained_root_nodes.extend(slot_nodes);
		}
	}

	let new_root_nodes = if matches!(patch.static_tree, StaticTemplateNode::Slot(_)) {
		retained_root_nodes
	} else {
		skeleton.root_nodes
	};
	if new_root_nodes.is_empty() {
		return Err(PatchError::PlanningFailed(
			"replacement template has no root nodes".to_owned(),
		));
	}

	let old_first = old_nodes.first().cloned();
	// Insert each node before the stable old-root marker in source order. Inserting
	// in reverse here would reverse sibling order because `before` does not move.
	for node in &new_root_nodes {
		stage_move(node, &old_parent, old_first.as_ref(), operations, undo)?;
	}

	for old in &old_nodes {
		if new_root_nodes.iter().any(|new| new.is_same_node(Some(old))) {
			continue;
		}
		if old.parent_node().is_some() {
			let before = old.next_sibling();
			operations.push(DomOperation::RemoveNode { node: old.clone() });
			undo.push(UndoOperation::RestoreNode {
				node: old.clone(),
				parent: old_parent.clone(),
				before,
			});
		}
	}
	Ok(())
}

fn build_skeleton(tree: &StaticTemplateNode) -> Result<DetachedSkeleton, PatchError> {
	let document = crate::dom::document();
	let fragment = document.as_web_sys().create_document_fragment();
	let fragment_node: web_sys::Node = fragment.clone().unchecked_into();
	let mut placements = BTreeMap::new();
	let mut expected_paths = BTreeMap::new();
	let mut path = Vec::new();
	let root_nodes = build_node(
		tree,
		&fragment_node,
		&mut path,
		None,
		&mut placements,
		&mut expected_paths,
	)?;
	for root in &root_nodes {
		fragment_node
			.append_child(root)
			.map_err(|error| PatchError::PlanningFailed(format!("build static root: {error:?}")))?;
	}
	Ok(DetachedSkeleton {
		root_nodes,
		placements,
		expected_paths,
	})
}

fn build_node(
	tree: &StaticTemplateNode,
	parent: &web_sys::Node,
	path: &mut Vec<u32>,
	before: Option<&web_sys::Node>,
	placements: &mut BTreeMap<DynamicSlotId, PlacementTarget>,
	expected_paths: &mut BTreeMap<DynamicSlotId, Vec<u32>>,
) -> Result<Vec<web_sys::Node>, PatchError> {
	match tree {
		StaticTemplateNode::Text(text) => {
			let node: web_sys::Node = crate::dom::document()
				.as_web_sys()
				.create_text_node(text)
				.unchecked_into();
			Ok(vec![node])
		}
		StaticTemplateNode::Slot(slot_id) => {
			if placements
				.insert(
					*slot_id,
					PlacementTarget {
						parent: parent.clone(),
						before: before.cloned(),
					},
				)
				.is_some()
			{
				return Err(PatchError::PlanningFailed(format!(
					"slot {:?} appears more than once",
					slot_id
				)));
			}
			expected_paths.insert(*slot_id, path.clone());
			Ok(Vec::new())
		}
		StaticTemplateNode::Element {
			tag,
			static_attrs,
			children,
		} => {
			let element = crate::dom::document()
				.create_element(tag)
				.map_err(PatchError::PlanningFailed)?
				.as_web_sys()
				.clone();
			for (name, value) in static_attrs {
				element.set_attribute(name, value).map_err(|error| {
					PatchError::PlanningFailed(format!("set attribute: {error:?}"))
				})?;
			}
			let element_node: web_sys::Node = element.clone().unchecked_into();
			let mut child_before = None;
			for index in (0..children.len()).rev() {
				path.push(index as u32);
				let child_nodes = build_node(
					&children[index],
					&element_node,
					path,
					child_before.as_ref(),
					placements,
					expected_paths,
				)?;
				path.pop();
				for child in child_nodes.iter().rev() {
					element_node
						.insert_before(child, child_before.as_ref())
						.map_err(|error| {
							PatchError::PlanningFailed(format!("build static child: {error:?}"))
						})?;
					child_before = Some(child.clone());
				}
			}
			Ok(vec![element_node])
		}
	}
}

fn validate_placements(
	tree: &StaticTemplateNode,
	placements: &[SlotPlacement],
) -> Result<(), PatchError> {
	let mut expected = BTreeMap::new();
	let mut path = Vec::new();
	collect_slot_paths(tree, &mut path, &mut expected)?;
	let mut actual = BTreeMap::new();
	for placement in placements {
		if actual
			.insert(placement.slot_id, placement.path.clone())
			.is_some()
		{
			return Err(PatchError::PlanningFailed(format!(
				"slot {:?} has duplicate placement",
				placement.slot_id
			)));
		}
	}
	if expected != actual {
		return Err(PatchError::PlanningFailed(
			"slot placement paths do not match the replacement tree".to_owned(),
		));
	}
	Ok(())
}

fn collect_slot_paths(
	tree: &StaticTemplateNode,
	path: &mut Vec<u32>,
	paths: &mut BTreeMap<DynamicSlotId, Vec<u32>>,
) -> Result<(), PatchError> {
	match tree {
		StaticTemplateNode::Element { children, .. } => {
			for (index, child) in children.iter().enumerate() {
				path.push(index as u32);
				collect_slot_paths(child, path, paths)?;
				path.pop();
			}
		}
		StaticTemplateNode::Slot(slot_id) => {
			if paths.insert(*slot_id, path.clone()).is_some() {
				return Err(PatchError::PlanningFailed(format!(
					"slot {:?} appears more than once",
					slot_id
				)));
			}
		}
		StaticTemplateNode::Text(_) => {}
	}
	Ok(())
}

fn stage_slot(
	slot: &MountedSlot,
	target: &PlacementTarget,
	operations: &mut Vec<DomOperation>,
	undo: &mut Vec<UndoOperation>,
) -> Result<Vec<web_sys::Node>, PatchError> {
	let nodes = match slot {
		MountedSlot::DynamicRange(DynamicRange { range, .. }) => {
			let current_nodes = range.current_nodes();
			let mut nodes = Vec::with_capacity(current_nodes.len() + 2);
			nodes.push(range.start.clone().unchecked_into());
			nodes.extend(current_nodes);
			nodes.push(range.end.clone().unchecked_into());
			nodes
		}
		MountedSlot::BoundElement(BoundElement { element, .. }) => {
			vec![element.clone().unchecked_into()]
		}
	};
	// Keep range anchors and their current reactive contents in DOM order while
	// moving them into the detached skeleton.
	for node in &nodes {
		stage_move(
			node,
			&target.parent,
			target.before.as_ref(),
			operations,
			undo,
		)?;
	}
	Ok(nodes)
}

fn stage_move(
	node: &web_sys::Node,
	parent: &web_sys::Node,
	before: Option<&web_sys::Node>,
	operations: &mut Vec<DomOperation>,
	undo: &mut Vec<UndoOperation>,
) -> Result<(), PatchError> {
	let original_parent = node
		.parent_node()
		.ok_or_else(|| PatchError::PlanningFailed("retained node has no parent".to_owned()))?;
	let original_before = node.next_sibling();
	operations.push(DomOperation::MoveNode {
		node: node.clone(),
		parent: parent.clone(),
		before: before.cloned(),
	});
	undo.push(UndoOperation::RestoreNode {
		node: node.clone(),
		parent: original_parent,
		before: original_before,
	});
	Ok(())
}

fn apply_operation(operation: &DomOperation) -> Result<(), String> {
	match operation {
		DomOperation::MoveNode {
			node,
			parent,
			before,
		} => parent
			.insert_before(node, before.as_ref())
			.map(|_| ())
			.map_err(|error| format!("move node: {error:?}")),
		DomOperation::SetAttribute {
			element,
			name,
			value,
		} => element
			.set_attribute(name, value)
			.map_err(|error| format!("set attribute: {error:?}")),
		DomOperation::RemoveAttribute { element, name } => element
			.remove_attribute(name)
			.map_err(|error| format!("remove attribute: {error:?}")),
		DomOperation::ReplaceStaticRoot { old, new } => {
			let parent = old
				.parent_node()
				.ok_or_else(|| "old root has no parent".to_owned())?;
			parent
				.replace_child(new, old)
				.map(|_| ())
				.map_err(|error| format!("replace root: {error:?}"))
		}
		DomOperation::RemoveNode { node } => {
			if let Some(parent) = node.parent_node() {
				parent
					.remove_child(node)
					.map(|_| ())
					.map_err(|error| format!("remove node: {error:?}"))?;
			}
			Ok(())
		}
	}
}

fn apply_undo(undo: &UndoOperation) -> Result<(), String> {
	match undo {
		UndoOperation::RestoreNode {
			node,
			parent,
			before,
		} => parent
			.insert_before(node, before.as_ref())
			.map(|_| ())
			.map_err(|error| format!("restore node: {error:?}")),
		UndoOperation::RestoreAttribute {
			element,
			name,
			value,
		} => match value {
			Some(value) => element
				.set_attribute(name, value)
				.map_err(|error| format!("restore attribute: {error:?}")),
			None => element
				.remove_attribute(name)
				.map_err(|error| format!("remove restored attribute: {error:?}")),
		},
		UndoOperation::RestoreRoot { old, new } => {
			let parent = new
				.parent_node()
				.ok_or_else(|| "new root has no parent".to_owned())?;
			parent
				.replace_child(old, new)
				.map(|_| ())
				.map_err(|error| format!("restore root: {error:?}"))
		}
	}
}
