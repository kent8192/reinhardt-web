use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::component::{Head, LinkTag, MetaTag, ScriptTag, StyleTag};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeadSlotKind {
	Default,
	StaticPage,
	RetainedHook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct HeadSlotId(
	/// Monotonic registry-local identity retained across slot replacement.
	pub(crate) u64,
);

#[derive(Debug)]
struct HeadSlot {
	kind: HeadSlotKind,
	sequence: u64,
	head: Head,
}

#[derive(Debug)]
pub(crate) struct HeadRegistry {
	slots: BTreeMap<HeadSlotId, HeadSlot>,
	next_sequence: u64,
}

#[derive(Debug)]
struct OwnedDescriptor<T> {
	owner: HeadSlotId,
	descriptor: T,
}

#[derive(Debug, Default)]
struct ResolvedHead {
	base: Option<OwnedDescriptor<Cow<'static, str>>>,
	meta_tags: Vec<OwnedDescriptor<MetaTag>>,
	title: Option<OwnedDescriptor<Cow<'static, str>>>,
	links: Vec<OwnedDescriptor<LinkTag>>,
	styles: Vec<OwnedDescriptor<StyleTag>>,
	scripts: Vec<OwnedDescriptor<ScriptTag>>,
}

/// One effective head descriptor together with its deterministic representative slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedHeadEntry {
	/// The effective singleton base URL and the slot that last supplied it.
	Base {
		/// Stable slot that owns the effective descriptor.
		owner: HeadSlotId,
		/// Canonical base URL descriptor.
		descriptor: Cow<'static, str>,
	},
	/// An exact-deduplicated meta descriptor and its first resolved owner.
	Meta {
		/// Stable representative slot for this exact descriptor.
		owner: HeadSlotId,
		/// Canonical structured meta descriptor.
		descriptor: MetaTag,
	},
	/// The effective singleton title and the slot that last supplied it.
	Title {
		/// Stable slot that owns the effective descriptor.
		owner: HeadSlotId,
		/// Canonical title descriptor.
		descriptor: Cow<'static, str>,
	},
	/// An exact-deduplicated link descriptor and its first resolved owner.
	Link {
		/// Stable representative slot for this exact descriptor.
		owner: HeadSlotId,
		/// Canonical structured link descriptor.
		descriptor: LinkTag,
	},
	/// An exact-deduplicated style descriptor and its first resolved owner.
	Style {
		/// Stable representative slot for this exact descriptor.
		owner: HeadSlotId,
		/// Canonical structured style descriptor.
		descriptor: StyleTag,
	},
	/// An exact-deduplicated script descriptor and its first resolved owner.
	Script {
		/// Stable representative slot for this exact descriptor.
		owner: HeadSlotId,
		/// Canonical structured script descriptor.
		descriptor: ScriptTag,
	},
}

impl HeadRegistry {
	pub(crate) fn new(default_head: Head) -> Self {
		let default_id = HeadSlotId(0);
		let default_slot = HeadSlot {
			kind: HeadSlotKind::Default,
			sequence: 0,
			head: default_head,
		};
		let mut slots = BTreeMap::new();
		slots.insert(default_id, default_slot);

		Self {
			slots,
			next_sequence: 1,
		}
	}

	pub(crate) fn register(&mut self, kind: HeadSlotKind, head: Head) -> HeadSlotId {
		let sequence = self.next_sequence;
		self.next_sequence = self
			.next_sequence
			.checked_add(1)
			.expect("document-head slot sequence exhausted");
		let id = HeadSlotId(sequence);
		self.slots.insert(
			id,
			HeadSlot {
				kind,
				sequence,
				head,
			},
		);
		id
	}

	pub(crate) fn replace(&mut self, id: HeadSlotId, head: Head) -> bool {
		let Some(slot) = self.slots.get_mut(&id) else {
			return false;
		};
		slot.head = head;
		true
	}

	pub(crate) fn head(&self, id: HeadSlotId) -> Option<Head> {
		self.slots.get(&id).map(|slot| slot.head.clone())
	}

	pub(crate) fn remove(&mut self, id: HeadSlotId) -> bool {
		self.slots.remove(&id).is_some()
	}

	pub(crate) fn resolve(&self) -> Head {
		let resolved = self.resolve_with_owners();
		Head {
			base: resolved.base.map(|entry| entry.descriptor),
			meta_tags: resolved
				.meta_tags
				.into_iter()
				.map(|entry| entry.descriptor)
				.collect(),
			title: resolved.title.map(|entry| entry.descriptor),
			links: resolved
				.links
				.into_iter()
				.map(|entry| entry.descriptor)
				.collect(),
			styles: resolved
				.styles
				.into_iter()
				.map(|entry| entry.descriptor)
				.collect(),
			scripts: resolved
				.scripts
				.into_iter()
				.map(|entry| entry.descriptor)
				.collect(),
		}
	}

	pub(crate) fn resolved_entries(&self) -> Vec<ResolvedHeadEntry> {
		let resolved = self.resolve_with_owners();
		let mut entries = Vec::new();

		if let Some(base) = resolved.base {
			entries.push(ResolvedHeadEntry::Base {
				owner: base.owner,
				descriptor: base.descriptor,
			});
		}
		entries.extend(
			resolved
				.meta_tags
				.into_iter()
				.map(|entry| ResolvedHeadEntry::Meta {
					owner: entry.owner,
					descriptor: entry.descriptor,
				}),
		);
		if let Some(title) = resolved.title {
			entries.push(ResolvedHeadEntry::Title {
				owner: title.owner,
				descriptor: title.descriptor,
			});
		}
		entries.extend(
			resolved
				.links
				.into_iter()
				.map(|entry| ResolvedHeadEntry::Link {
					owner: entry.owner,
					descriptor: entry.descriptor,
				}),
		);
		entries.extend(
			resolved
				.styles
				.into_iter()
				.map(|entry| ResolvedHeadEntry::Style {
					owner: entry.owner,
					descriptor: entry.descriptor,
				}),
		);
		entries.extend(
			resolved
				.scripts
				.into_iter()
				.map(|entry| ResolvedHeadEntry::Script {
					owner: entry.owner,
					descriptor: entry.descriptor,
				}),
		);

		entries
	}

	fn resolve_with_owners(&self) -> ResolvedHead {
		let mut resolved = ResolvedHead::default();
		for (owner, slot) in self.ordered_slots() {
			if let Some(base) = &slot.head.base {
				resolved.base = Some(OwnedDescriptor {
					owner,
					descriptor: base.clone(),
				});
			}
			append_unique(&mut resolved.meta_tags, owner, &slot.head.meta_tags);
			if let Some(title) = &slot.head.title {
				resolved.title = Some(OwnedDescriptor {
					owner,
					descriptor: title.clone(),
				});
			}
			append_unique(&mut resolved.links, owner, &slot.head.links);
			append_unique(&mut resolved.styles, owner, &slot.head.styles);
			append_unique(&mut resolved.scripts, owner, &slot.head.scripts);
		}
		resolved
	}

	fn ordered_slots(&self) -> Vec<(HeadSlotId, &HeadSlot)> {
		let mut slots = self
			.slots
			.iter()
			.map(|(id, slot)| (*id, slot))
			.collect::<Vec<_>>();
		slots.sort_by_key(|(_, slot)| slot_sort_key(slot));
		slots
	}
}

impl ResolvedHeadEntry {
	/// Returns the slot currently representing this effective descriptor.
	pub(crate) fn owner(&self) -> HeadSlotId {
		match self {
			Self::Base { owner, .. }
			| Self::Meta { owner, .. }
			| Self::Title { owner, .. }
			| Self::Link { owner, .. }
			| Self::Style { owner, .. }
			| Self::Script { owner, .. } => *owner,
		}
	}

	/// Returns a stable owner-and-descriptor identity for managed DOM nodes.
	pub(crate) fn marker(&self) -> String {
		let owner = self.owner();
		let (kind, descriptor_hash) = self.descriptor_identity();
		format!("slot-{}-{kind}-{descriptor_hash:016x}", owner.0)
	}

	fn descriptor_identity(&self) -> (&'static str, u64) {
		let mut hasher = DescriptorHasher::new();
		let kind = match self {
			Self::Base { descriptor, .. } => {
				hasher.write_str(descriptor);
				"base"
			}
			Self::Meta { descriptor, .. } => {
				hasher.write_optional_str(descriptor.name.as_deref());
				hasher.write_optional_str(descriptor.property.as_deref());
				hasher.write_str(&descriptor.content);
				hasher.write_optional_str(descriptor.charset.as_deref());
				hasher.write_optional_str(descriptor.http_equiv.as_deref());
				"meta"
			}
			Self::Title { descriptor, .. } => {
				hasher.write_str(descriptor);
				"title"
			}
			Self::Link { descriptor, .. } => {
				hasher.write_str(&descriptor.rel);
				hasher.write_str(&descriptor.href);
				hasher.write_optional_str(descriptor.type_attr.as_deref());
				hasher.write_optional_str(descriptor.as_attr.as_deref());
				hasher.write_optional_str(descriptor.crossorigin.as_deref());
				hasher.write_optional_str(descriptor.integrity.as_deref());
				hasher.write_optional_str(descriptor.media.as_deref());
				hasher.write_optional_str(descriptor.sizes.as_deref());
				"link"
			}
			Self::Style { descriptor, .. } => {
				hasher.write_str(&descriptor.content);
				hasher.write_optional_str(descriptor.media.as_deref());
				hasher.write_optional_str(descriptor.nonce.as_deref());
				"style"
			}
			Self::Script { descriptor, .. } => {
				hasher.write_optional_str(descriptor.src.as_deref());
				hasher.write_optional_str(descriptor.content.as_deref());
				hasher.write_optional_str(descriptor.type_attr.as_deref());
				hasher.write_bool(descriptor.is_async);
				hasher.write_bool(descriptor.is_defer);
				hasher.write_optional_str(descriptor.crossorigin.as_deref());
				hasher.write_optional_str(descriptor.integrity.as_deref());
				hasher.write_optional_str(descriptor.nonce.as_deref());
				"script"
			}
		};
		(kind, hasher.finish())
	}
}

fn append_unique<T>(entries: &mut Vec<OwnedDescriptor<T>>, owner: HeadSlotId, descriptors: &[T])
where
	T: Clone + Eq,
{
	for descriptor in descriptors {
		if entries.iter().any(|entry| entry.descriptor == *descriptor) {
			continue;
		}
		entries.push(OwnedDescriptor {
			owner,
			descriptor: descriptor.clone(),
		});
	}
}

fn slot_sort_key(slot: &HeadSlot) -> (u8, u64) {
	let tier = match slot.kind {
		HeadSlotKind::Default => 0,
		HeadSlotKind::StaticPage => 1,
		HeadSlotKind::RetainedHook => 2,
	};
	(tier, slot.sequence)
}

struct DescriptorHasher(u64);

impl DescriptorHasher {
	const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
	const PRIME: u64 = 0x100000001b3;

	fn new() -> Self {
		Self(Self::OFFSET_BASIS)
	}

	fn write_optional_str(&mut self, value: Option<&str>) {
		match value {
			Some(value) => {
				self.write_byte(1);
				self.write_str(value);
			}
			None => self.write_byte(0),
		}
	}

	fn write_str(&mut self, value: &str) {
		for byte in u64::try_from(value.len())
			.expect("descriptor string length must fit in u64")
			.to_le_bytes()
		{
			self.write_byte(byte);
		}
		for byte in value.bytes() {
			self.write_byte(byte);
		}
	}

	fn write_bool(&mut self, value: bool) {
		self.write_byte(u8::from(value));
	}

	fn write_byte(&mut self, byte: u8) {
		self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(Self::PRIME);
	}

	fn finish(self) -> u64 {
		self.0
	}
}

#[cfg(test)]
mod tests {
	use super::{HeadRegistry, HeadSlotKind};
	use crate::component::Head;

	#[test]
	fn registry_restores_parent_and_preserves_static_preorder() {
		let mut registry = HeadRegistry::new(Head::new());
		let parent = registry.register(HeadSlotKind::StaticPage, Head::new().title("Project"));
		let child = registry.register(HeadSlotKind::StaticPage, Head::new().title("Outline"));

		assert_eq!(registry.resolve().title.as_deref(), Some("Outline"));
		assert!(registry.remove(child));
		assert_eq!(registry.resolve().title.as_deref(), Some("Project"));
		assert!(registry.remove(parent));
	}

	#[test]
	fn replacing_a_slot_does_not_move_it_after_later_static_slots() {
		let mut registry = HeadRegistry::new(Head::new());
		let first = registry.register(HeadSlotKind::StaticPage, Head::new().title("First"));
		registry.register(HeadSlotKind::StaticPage, Head::new().title("Later"));

		assert!(registry.replace(first, Head::new().title("Updated first")));
		assert_eq!(registry.resolve().title.as_deref(), Some("Later"));
	}

	#[test]
	fn retained_hooks_follow_all_static_pages_in_registration_order() {
		let mut registry = HeadRegistry::new(Head::new());
		registry.register(HeadSlotKind::RetainedHook, Head::new().title("First hook"));
		registry.register(HeadSlotKind::RetainedHook, Head::new().title("Later hook"));
		registry.register(HeadSlotKind::StaticPage, Head::new().title("Static page"));

		assert_eq!(registry.resolve().title.as_deref(), Some("Later hook"));
	}

	#[test]
	fn exact_duplicates_keep_the_first_resolved_owner_until_it_is_removed() {
		let duplicate = Head::new().meta_description("shared description");
		let mut registry = HeadRegistry::new(Head::new());
		let first = registry.register(HeadSlotKind::StaticPage, duplicate.clone());
		let second = registry.register(HeadSlotKind::StaticPage, duplicate);

		let first_entry = registry
			.resolved_entries()
			.into_iter()
			.next()
			.expect("the shared meta descriptor should resolve");
		assert_eq!(first_entry.owner(), first);
		let first_marker = first_entry.marker();

		assert!(registry.remove(first));
		let transferred_entry = registry
			.resolved_entries()
			.into_iter()
			.next()
			.expect("the duplicate owner should take over");
		assert_eq!(transferred_entry.owner(), second);
		assert_ne!(transferred_entry.marker(), first_marker);
	}

	#[test]
	fn marker_uses_a_platform_independent_descriptor_length() {
		let mut registry = HeadRegistry::new(Head::new());
		let owner = registry.register(
			HeadSlotKind::RetainedHook,
			Head::new().title("Stable title"),
		);

		let initial = registry
			.resolved_entries()
			.into_iter()
			.next()
			.expect("the title should resolve");
		assert_eq!(initial.owner(), owner);
		assert_eq!(initial.marker(), "slot-1-title-7bd04faf3cab580d");

		assert!(registry.replace(owner, Head::new().title("Stable title")));
		let replacement = registry
			.resolved_entries()
			.into_iter()
			.next()
			.expect("the replacement title should resolve");
		assert_eq!(replacement.owner(), owner);
		assert_eq!(replacement.marker(), "slot-1-title-7bd04faf3cab580d");
	}
}
