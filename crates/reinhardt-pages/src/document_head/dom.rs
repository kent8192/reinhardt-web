use std::mem;

use wasm_bindgen::JsCast;

use super::DocumentHeadError;
use super::registry::ResolvedHeadEntry;
use crate::dom::{Document, Element};

const MANAGED_ATTRIBUTE: &str = "data-reinhardt-head";
const MANAGED_SELECTOR: &str = "[data-reinhardt-head]";

pub(crate) struct BrowserDocumentHead {
	document: Document,
	head: web_sys::HtmlHeadElement,
	managed_nodes: Vec<ManagedHeadNode>,
	unmanaged_titles: Option<Vec<DetachedSingleton>>,
	unmanaged_bases: Option<Vec<DetachedSingleton>>,
	marked_nodes_adopted: bool,
	last_entries: Vec<ResolvedHeadEntry>,
}

struct ManagedHeadNode {
	element: web_sys::Element,
	marker: String,
	identity: Option<MarkerIdentity>,
}

struct DetachedSingleton {
	element: web_sys::Element,
	next_sibling: Option<web_sys::Node>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeadEntryKind {
	Base,
	Meta,
	Title,
	Link,
	Style,
	Script,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MarkerIdentity {
	owner: u64,
	kind: HeadEntryKind,
	descriptor_hash: String,
}

impl BrowserDocumentHead {
	pub(crate) fn new() -> Result<Self, DocumentHeadError> {
		let window = web_sys::window().ok_or_else(|| {
			DocumentHeadError::DomOperation("window object is unavailable".to_owned())
		})?;
		let document = window.document().ok_or_else(|| {
			DocumentHeadError::DomOperation("document object is unavailable".to_owned())
		})?;
		let document = Document::new(document);
		let head = document
			.head()
			.ok_or_else(|| {
				DocumentHeadError::DomOperation("document head is unavailable".to_owned())
			})?
			.as_web_sys()
			.clone()
			.dyn_into::<web_sys::HtmlHeadElement>()
			.map_err(|_| {
				DocumentHeadError::DomOperation(
					"document head has an unexpected element type".to_owned(),
				)
			})?;

		Ok(Self {
			document,
			head,
			managed_nodes: Vec::new(),
			unmanaged_titles: None,
			unmanaged_bases: None,
			marked_nodes_adopted: false,
			last_entries: Vec::new(),
		})
	}

	pub(crate) fn adopt_marked_nodes(&mut self) -> Result<(), DocumentHeadError> {
		if self.marked_nodes_adopted {
			return Ok(());
		}

		let nodes = self
			.head
			.query_selector_all(MANAGED_SELECTOR)
			.map_err(|error| dom_error("query framework-managed head nodes", error))?;
		for index in 0..nodes.length() {
			let Some(element) = nodes
				.item(index)
				.and_then(|node| node.dyn_into::<web_sys::Element>().ok())
			else {
				continue;
			};
			let Some(marker) = element.get_attribute(MANAGED_ATTRIBUTE) else {
				continue;
			};
			self.managed_nodes
				.push(ManagedHeadNode::new(element, marker));
		}
		self.marked_nodes_adopted = true;
		Ok(())
	}

	pub(crate) fn reconcile(
		&mut self,
		desired_entries: &[ResolvedHeadEntry],
	) -> Result<(), DocumentHeadError> {
		let previous_entries = self.last_entries.clone();
		let had_adopted_nodes = self.marked_nodes_adopted && !self.managed_nodes.is_empty();
		match self.reconcile_inner(desired_entries) {
			Ok(()) => {
				self.last_entries = desired_entries.to_vec();
				Ok(())
			}
			Err(error) => {
				self.managed_nodes.clear();
				self.marked_nodes_adopted = false;
				let _ = self.adopt_marked_nodes();
				if previous_entries.is_empty() && had_adopted_nodes {
					return Err(error);
				}
				if self.reconcile_inner(&previous_entries).is_ok() {
					self.last_entries = previous_entries;
				}
				Err(error)
			}
		}
	}

	fn reconcile_inner(
		&mut self,
		desired_entries: &[ResolvedHeadEntry],
	) -> Result<(), DocumentHeadError> {
		let wants_title = desired_entries
			.iter()
			.any(|entry| matches!(entry, ResolvedHeadEntry::Title { .. }));
		let wants_base = desired_entries
			.iter()
			.any(|entry| matches!(entry, ResolvedHeadEntry::Base { .. }));

		if wants_title && self.unmanaged_titles.is_none() {
			self.unmanaged_titles = Some(self.detach_unmanaged_singletons("title")?);
		}
		if wants_base && self.unmanaged_bases.is_none() {
			self.unmanaged_bases = Some(self.detach_unmanaged_singletons("base")?);
		}

		let mut available_nodes = mem::take(&mut self.managed_nodes);
		let mut next_nodes = Vec::with_capacity(desired_entries.len());

		for (index, entry) in desired_entries.iter().enumerate() {
			let marker = entry.marker();
			let identity = MarkerIdentity::parse(&marker).ok_or_else(|| {
				DocumentHeadError::DomOperation(format!(
					"resolved head marker has an invalid identity: {marker}"
				))
			})?;
			let mut node = take_matching_node(&mut available_nodes, |node| {
				node.marker == marker && node.matches_kind(identity.kind)
			})
			.or_else(|| {
				take_matching_node(&mut available_nodes, |node| {
					node.has_descriptor(&identity) && node.matches_kind(identity.kind)
				})
			});

			if let Some(mut node) = node.take() {
				let descriptor_is_unchanged = node.has_descriptor(&identity);
				if !descriptor_is_unchanged && identity.kind.can_update_in_place() {
					apply_descriptor(&Element::new(node.element.clone()), entry)?;
				} else if !descriptor_is_unchanged {
					let insert_before = next_matching_desired_node(
						&desired_entries[index + 1..],
						&available_nodes,
					)?;
					let replacement =
						self.create_managed_node(entry, marker, identity, insert_before.as_ref())?;
					remove_managed_node(&node.element)?;
					next_nodes.push(replacement);
					continue;
				}
				if node.marker != marker {
					Element::new(node.element.clone())
						.set_attribute(MANAGED_ATTRIBUTE, &marker)
						.map_err(DocumentHeadError::DomOperation)?;
				}
				node.marker = marker;
				node.identity = Some(identity);
				next_nodes.push(node);
			} else {
				let insert_before =
					next_matching_desired_node(&desired_entries[index + 1..], &available_nodes)?;
				next_nodes.push(self.create_managed_node(
					entry,
					marker,
					identity,
					insert_before.as_ref(),
				)?);
			}
		}

		for obsolete in available_nodes {
			remove_managed_node(&obsolete.element)?;
		}
		self.managed_nodes = next_nodes;

		if !wants_title && let Some(snapshot) = self.unmanaged_titles.take() {
			self.restore_unmanaged_singletons(snapshot)?;
		}
		if !wants_base && let Some(snapshot) = self.unmanaged_bases.take() {
			self.restore_unmanaged_singletons(snapshot)?;
		}

		Ok(())
	}

	fn create_managed_node(
		&self,
		entry: &ResolvedHeadEntry,
		marker: String,
		identity: MarkerIdentity,
		insert_before: Option<&web_sys::Element>,
	) -> Result<ManagedHeadNode, DocumentHeadError> {
		let element = self
			.document
			.create_element(identity.kind.tag_name())
			.map_err(DocumentHeadError::DomOperation)?;
		apply_descriptor(&element, entry)?;
		element
			.set_attribute(MANAGED_ATTRIBUTE, &marker)
			.map_err(DocumentHeadError::DomOperation)?;
		let head_node: web_sys::Node = self.head.clone().unchecked_into();
		let element_node: web_sys::Node = element.as_web_sys().clone().unchecked_into();
		let anchor = insert_before
			.map(|element| element.clone().unchecked_into())
			.filter(|node: &web_sys::Node| {
				node.parent_node()
					.is_some_and(|parent| parent.is_same_node(Some(&head_node)))
			});
		head_node
			.insert_before(&element_node, anchor.as_ref())
			.map_err(|error| dom_error("insert framework-managed head node", error))?;

		Ok(ManagedHeadNode {
			element: element.as_web_sys().clone(),
			marker,
			identity: Some(identity),
		})
	}

	fn detach_unmanaged_singletons(
		&self,
		tag_name: &str,
	) -> Result<Vec<DetachedSingleton>, DocumentHeadError> {
		let selector = format!("{tag_name}:not([{MANAGED_ATTRIBUTE}])");
		let nodes = self
			.head
			.query_selector_all(&selector)
			.map_err(|error| dom_error("query unmanaged head singleton", error))?;
		let head_node: web_sys::Node = self.head.clone().unchecked_into();
		let mut snapshot = Vec::new();

		for index in 0..nodes.length() {
			let Some(element) = nodes
				.item(index)
				.and_then(|node| node.dyn_into::<web_sys::Element>().ok())
			else {
				continue;
			};
			let element_node: web_sys::Node = element.clone().unchecked_into();
			if !element_node
				.parent_node()
				.is_some_and(|parent| parent.is_same_node(Some(&head_node)))
			{
				continue;
			}
			snapshot.push(DetachedSingleton {
				element,
				next_sibling: element_node.next_sibling(),
			});
			head_node
				.remove_child(&element_node)
				.map_err(|error| dom_error("detach unmanaged head singleton", error))?;
		}

		Ok(snapshot)
	}

	fn restore_unmanaged_singletons(
		&self,
		snapshot: Vec<DetachedSingleton>,
	) -> Result<(), DocumentHeadError> {
		let head_node: web_sys::Node = self.head.clone().unchecked_into();
		for detached in snapshot.into_iter().rev() {
			let element_node: web_sys::Node = detached.element.unchecked_into();
			let anchor = detached.next_sibling.filter(|node| {
				node.parent_node()
					.is_some_and(|parent| parent.is_same_node(Some(&head_node)))
			});
			head_node
				.insert_before(&element_node, anchor.as_ref())
				.map_err(|error| dom_error("restore unmanaged head singleton", error))?;
		}
		Ok(())
	}
}

impl ManagedHeadNode {
	fn new(element: web_sys::Element, marker: String) -> Self {
		let identity = MarkerIdentity::parse(&marker);
		Self {
			element,
			marker,
			identity,
		}
	}

	fn has_descriptor(&self, desired: &MarkerIdentity) -> bool {
		self.identity.as_ref().is_some_and(|current| {
			current.kind == desired.kind && current.descriptor_hash == desired.descriptor_hash
		})
	}

	fn matches_kind(&self, kind: HeadEntryKind) -> bool {
		self.element
			.tag_name()
			.eq_ignore_ascii_case(kind.tag_name())
	}
}

impl HeadEntryKind {
	fn can_update_in_place(self) -> bool {
		matches!(self, Self::Base | Self::Title)
	}

	fn tag_name(self) -> &'static str {
		match self {
			Self::Base => "base",
			Self::Meta => "meta",
			Self::Title => "title",
			Self::Link => "link",
			Self::Style => "style",
			Self::Script => "script",
		}
	}
}

impl MarkerIdentity {
	fn parse(marker: &str) -> Option<Self> {
		let mut parts = marker.split('-');
		if parts.next()? != "slot" {
			return None;
		}
		let owner = parts.next()?.parse().ok()?;
		let kind = match parts.next()? {
			"base" => HeadEntryKind::Base,
			"meta" => HeadEntryKind::Meta,
			"title" => HeadEntryKind::Title,
			"link" => HeadEntryKind::Link,
			"style" => HeadEntryKind::Style,
			"script" => HeadEntryKind::Script,
			_ => return None,
		};
		let descriptor_hash = parts.next()?;
		if parts.next().is_some()
			|| descriptor_hash.len() != 16
			|| !descriptor_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
		{
			return None;
		}

		Some(Self {
			owner,
			kind,
			descriptor_hash: descriptor_hash.to_owned(),
		})
	}
}

fn take_matching_node(
	nodes: &mut Vec<ManagedHeadNode>,
	mut predicate: impl FnMut(&ManagedHeadNode) -> bool,
) -> Option<ManagedHeadNode> {
	let index = nodes.iter().position(&mut predicate)?;
	Some(nodes.remove(index))
}

fn next_matching_desired_node(
	desired_entries: &[ResolvedHeadEntry],
	available_nodes: &[ManagedHeadNode],
) -> Result<Option<web_sys::Element>, DocumentHeadError> {
	for entry in desired_entries {
		let marker = entry.marker();
		let identity = MarkerIdentity::parse(&marker).ok_or_else(|| {
			DocumentHeadError::DomOperation(format!(
				"resolved head marker has an invalid identity: {marker}"
			))
		})?;
		if let Some(node) = available_nodes.iter().find(|node| {
			(node.marker == marker || node.has_descriptor(&identity))
				&& node.matches_kind(identity.kind)
		}) {
			return Ok(Some(node.element.clone()));
		}
	}

	Ok(None)
}

fn apply_descriptor(element: &Element, entry: &ResolvedHeadEntry) -> Result<(), DocumentHeadError> {
	match entry {
		ResolvedHeadEntry::Base { descriptor, .. } => {
			element
				.set_attribute("href", descriptor)
				.map_err(DocumentHeadError::DomOperation)?;
		}
		ResolvedHeadEntry::Meta { descriptor, .. } => {
			set_optional_attribute(element, "name", descriptor.name.as_deref())?;
			set_optional_attribute(element, "property", descriptor.property.as_deref())?;
			element
				.set_attribute("content", &descriptor.content)
				.map_err(DocumentHeadError::DomOperation)?;
			set_optional_attribute(element, "charset", descriptor.charset.as_deref())?;
			set_optional_attribute(element, "http-equiv", descriptor.http_equiv.as_deref())?;
		}
		ResolvedHeadEntry::Title { descriptor, .. } => {
			element.set_text_content(descriptor);
		}
		ResolvedHeadEntry::Link { descriptor, .. } => {
			element
				.set_attribute("rel", &descriptor.rel)
				.map_err(DocumentHeadError::DomOperation)?;
			element
				.set_attribute("href", &descriptor.href)
				.map_err(DocumentHeadError::DomOperation)?;
			set_optional_attribute(element, "type", descriptor.type_attr.as_deref())?;
			set_optional_attribute(element, "as", descriptor.as_attr.as_deref())?;
			set_optional_attribute(element, "crossorigin", descriptor.crossorigin.as_deref())?;
			set_optional_attribute(element, "integrity", descriptor.integrity.as_deref())?;
			set_optional_attribute(element, "media", descriptor.media.as_deref())?;
			set_optional_attribute(element, "sizes", descriptor.sizes.as_deref())?;
		}
		ResolvedHeadEntry::Style { descriptor, .. } => {
			set_optional_attribute(element, "media", descriptor.media.as_deref())?;
			set_optional_attribute(element, "nonce", descriptor.nonce.as_deref())?;
			element.set_text_content(&descriptor.content);
		}
		ResolvedHeadEntry::Script { descriptor, .. } => {
			set_optional_attribute(element, "type", descriptor.type_attr.as_deref())?;
			set_boolean_attribute(element, "async", descriptor.is_async)?;
			element
				.set_property(
					"async",
					&wasm_bindgen::JsValue::from_bool(descriptor.is_async),
				)
				.map_err(DocumentHeadError::DomOperation)?;
			set_optional_attribute(element, "src", descriptor.src.as_deref())?;
			set_boolean_attribute(element, "defer", descriptor.is_defer)?;
			set_optional_attribute(element, "crossorigin", descriptor.crossorigin.as_deref())?;
			set_optional_attribute(element, "integrity", descriptor.integrity.as_deref())?;
			set_optional_attribute(element, "nonce", descriptor.nonce.as_deref())?;
			element.set_text_content(descriptor.content.as_deref().unwrap_or_default());
		}
	}
	Ok(())
}

fn set_optional_attribute(
	element: &Element,
	name: &str,
	value: Option<&str>,
) -> Result<(), DocumentHeadError> {
	match value {
		Some(value) => element
			.set_attribute(name, value)
			.map_err(DocumentHeadError::DomOperation),
		None => element
			.remove_attribute(name)
			.map_err(DocumentHeadError::DomOperation),
	}
}

fn set_boolean_attribute(
	element: &Element,
	name: &str,
	value: bool,
) -> Result<(), DocumentHeadError> {
	if value {
		element
			.set_attribute(name, "")
			.map_err(DocumentHeadError::DomOperation)
	} else {
		element
			.remove_attribute(name)
			.map_err(DocumentHeadError::DomOperation)
	}
}

fn remove_managed_node(element: &web_sys::Element) -> Result<(), DocumentHeadError> {
	if !element.has_attribute(MANAGED_ATTRIBUTE) {
		return Ok(());
	}
	let node: web_sys::Node = element.clone().unchecked_into();
	if let Some(parent) = node.parent_node() {
		parent
			.remove_child(&node)
			.map_err(|error| dom_error("remove obsolete framework-managed head node", error))?;
	}
	Ok(())
}

fn dom_error(context: &str, error: wasm_bindgen::JsValue) -> DocumentHeadError {
	DocumentHeadError::DomOperation(format!("{context}: {error:?}"))
}

#[cfg(test)]
mod tests {
	use wasm_bindgen::JsCast;
	use wasm_bindgen_test::*;

	use super::{BrowserDocumentHead, MANAGED_ATTRIBUTE};
	use crate::component::ScriptTag;
	use crate::document_head::registry::{HeadSlotId, ResolvedHeadEntry};
	use crate::dom::Element;

	wasm_bindgen_test_configure!(run_in_browser);

	struct BrowserDocumentHeadFixture {
		browser: BrowserDocumentHead,
	}

	impl BrowserDocumentHeadFixture {
		fn new() -> Self {
			Self {
				browser: BrowserDocumentHead::new().expect("browser document head"),
			}
		}

		fn reconcile(&mut self, entries: &[ResolvedHeadEntry]) {
			self.browser
				.reconcile(entries)
				.expect("document-head reconciliation");
		}

		fn managed_script_markers(&self) -> Vec<String> {
			let scripts = self
				.browser
				.head
				.query_selector_all(&format!("script[{MANAGED_ATTRIBUTE}]"))
				.expect("managed script selector");
			(0..scripts.length())
				.filter_map(|index| scripts.item(index))
				.filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
				.filter(|element| {
					self.browser
						.managed_nodes
						.iter()
						.any(|node| element.is_same_node(Some(&node.element)))
				})
				.filter_map(|element| element.get_attribute(MANAGED_ATTRIBUTE))
				.collect()
		}

		fn managed_element(&self, marker: &str) -> web_sys::Element {
			self.browser
				.managed_nodes
				.iter()
				.find(|node| node.marker == marker)
				.map(|node| node.element.clone())
				.expect("managed head node")
		}
	}

	impl Drop for BrowserDocumentHeadFixture {
		fn drop(&mut self) {
			let _ = self.browser.reconcile(&[]);
		}
	}

	fn external_script(owner: u64, src: &'static str) -> ResolvedHeadEntry {
		ResolvedHeadEntry::Script {
			owner: HeadSlotId(owner),
			descriptor: ScriptTag::external(src),
		}
	}

	#[wasm_bindgen_test]
	fn replacing_early_script_preserves_order_and_sync_property() {
		let mut fixture = BrowserDocumentHeadFixture::new();
		let first = external_script(1, "data:text/javascript,void%200");
		let second = external_script(2, "data:text/javascript,void%201");
		fixture.reconcile(&[first, second.clone()]);

		let replacement = external_script(1, "data:text/javascript,void%202");
		fixture.reconcile(&[replacement.clone(), second.clone()]);

		assert_eq!(
			fixture.managed_script_markers(),
			vec![replacement.marker(), second.marker()],
			"replacing an earlier script must retain the resolved collection order"
		);
		assert_eq!(
			Element::new(fixture.managed_element(&replacement.marker()))
				.get_property("async")
				.expect("script async property")
				.as_bool(),
			Some(false),
			"external scripts without with_async() must remain synchronous"
		);
	}
}
