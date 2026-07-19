//! WASM-side template descriptor, overlay, and mounted-instance registry.

use std::{
	cell::RefCell,
	collections::BTreeMap,
	rc::{Rc, Weak},
};

use super::{
	protocol::{
		ClientHello, CompiledBuildId, DynamicAbiHash, PatchGeneration, StaticTemplateNode,
		TemplateDescriptor, TemplateKey, TemplatePatch,
	},
	template_instance::TemplateInstance,
};

#[cfg(wasm)]
struct RegistryInner {
	descriptors: BTreeMap<TemplateKey, TemplateDescriptor>,
	overlays: BTreeMap<TemplateKey, (StaticTemplateNode, PatchGeneration)>,
	deferred_patches: BTreeMap<TemplateKey, DeferredPatch>,
	instances: BTreeMap<u64, (TemplateKey, TemplateInstance)>,
	next_instance_id: u64,
	latest_generation: PatchGeneration,
	build_id: CompiledBuildId,
	manifest_digest: [u8; 32],
	identity_is_explicit: bool,
}

#[cfg(wasm)]
struct DeferredPatch {
	patch: TemplatePatch,
	generation: PatchGeneration,
}

/// Result of registering a descriptor that may have a deferred patch.
#[cfg(wasm)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferredPatchOutcome {
	/// No compatible deferred patch was waiting for this descriptor.
	None,
	/// A compatible static overlay was installed before the instance mounted.
	Applied,
	/// A stale client exposed a different dynamic ABI for the deferred patch.
	AbiMismatch,
}

/// Runtime registry for compiled descriptors and mounted instances.
#[cfg(wasm)]
#[derive(Clone)]
pub struct TemplateRegistry {
	inner: Rc<RefCell<RegistryInner>>,
}

#[cfg(wasm)]
impl Default for TemplateRegistry {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(wasm)]
impl TemplateRegistry {
	/// Creates an empty registry for one loaded WASM build.
	pub fn new() -> Self {
		Self {
			inner: Rc::new(RefCell::new(RegistryInner {
				descriptors: BTreeMap::new(),
				overlays: BTreeMap::new(),
				deferred_patches: BTreeMap::new(),
				instances: BTreeMap::new(),
				next_instance_id: 0,
				latest_generation: PatchGeneration(0),
				build_id: CompiledBuildId([0; 32]),
				manifest_digest: [0; 32],
				identity_is_explicit: false,
			})),
		}
	}

	/// Registers or replaces a descriptor for the current compiled build.
	pub fn register_descriptor(&self, descriptor: TemplateDescriptor) -> DeferredPatchOutcome {
		let mut inner = self.inner.borrow_mut();
		let key = descriptor.key.clone();
		inner.descriptors.insert(key.clone(), descriptor.clone());
		let outcome = match inner.deferred_patches.remove(&key) {
			Some(deferred) if deferred.generation < inner.latest_generation => {
				DeferredPatchOutcome::None
			}
			Some(deferred) if deferred.patch.abi_hash != descriptor.abi_hash => {
				DeferredPatchOutcome::AbiMismatch
			}
			Some(deferred) => {
				if descriptor.static_tree == deferred.patch.static_tree {
					inner.overlays.remove(&key);
				} else {
					inner
						.overlays
						.insert(key, (deferred.patch.static_tree, deferred.generation));
				}
				inner.latest_generation = inner.latest_generation.max(deferred.generation);
				DeferredPatchOutcome::Applied
			}
			None => DeferredPatchOutcome::None,
		};
		#[cfg(feature = "hmr")]
		Self::refresh_derived_identity(&mut inner);
		drop(inner);
		#[cfg(feature = "hmr")]
		crate::hmr::bridge::notify_client_hello_changed();
		outcome
	}

	/// Registers the descriptor that owns one mounted `page!` invocation.
	///
	/// Nested descriptors remain part of the owner's serialized descriptor and
	/// therefore its manifest identity. They are not registered independently
	/// until nested-template instance mounting is available.
	pub fn register_descriptor_tree(&self, descriptor: TemplateDescriptor) -> DeferredPatchOutcome {
		self.register_descriptor(descriptor)
	}

	/// Defers a patch whose template has not been mounted in this browser yet.
	///
	/// The descriptor is unavailable until a route, conditional branch, or list
	/// iteration first mounts it. The patch is retained only by template key and
	/// is checked against that descriptor's ABI before it can become an overlay.
	pub(crate) fn defer_patch(&self, patch: TemplatePatch, generation: PatchGeneration) {
		let mut inner = self.inner.borrow_mut();
		if generation < inner.latest_generation || inner.descriptors.contains_key(&patch.key) {
			return;
		}
		let should_replace = inner
			.deferred_patches
			.get(&patch.key)
			.is_none_or(|current| current.generation <= generation);
		if should_replace {
			inner
				.deferred_patches
				.insert(patch.key.clone(), DeferredPatch { patch, generation });
		}
	}

	/// Returns whether this browser has mounted or registered a template key.
	pub(crate) fn has_descriptor(&self, key: &TemplateKey) -> bool {
		self.inner.borrow().descriptors.contains_key(key)
	}

	/// Sets the build identity advertised by the browser handshake.
	pub fn set_build_identity(&self, build_id: CompiledBuildId, manifest_digest: [u8; 32]) {
		let mut inner = self.inner.borrow_mut();
		inner.build_id = build_id;
		inner.manifest_digest = manifest_digest;
		inner.identity_is_explicit = true;
	}

	#[cfg(feature = "hmr")]
	fn refresh_derived_identity(inner: &mut RegistryInner) {
		if inner.identity_is_explicit {
			return;
		}
		let (build_id, manifest_digest) =
			super::protocol::template_manifest_identity(inner.descriptors.values().cloned());
		inner.build_id = build_id;
		inner.manifest_digest = manifest_digest;
	}

	/// Returns the number of mounted instances currently retained by the registry.
	pub fn instance_count(&self) -> usize {
		self.inner.borrow().instances.len()
	}

	/// Returns the current build identity and accepted patch generation.
	pub(crate) fn identity(&self) -> (CompiledBuildId, [u8; 32], PatchGeneration) {
		let inner = self.inner.borrow();
		(
			inner.build_id,
			inner.manifest_digest,
			inner.latest_generation,
		)
	}

	/// Returns a cloned descriptor for detached patch planning.
	pub(crate) fn descriptor_for(&self, key: &TemplateKey) -> Option<TemplateDescriptor> {
		self.inner.borrow().descriptors.get(key).cloned()
	}

	/// Visits mounted instances without exposing the registry's storage.
	pub(crate) fn for_each_instance(
		&self,
		key: &TemplateKey,
		mut visit: impl FnMut(&TemplateInstance),
	) {
		let inner = self.inner.borrow();
		for (_, (instance_key, instance)) in &inner.instances {
			if instance_key == key {
				visit(instance);
			}
		}
	}

	/// Publishes overlays after a complete DOM transaction succeeds.
	pub(crate) fn publish_overlays(
		&self,
		overlays: &[(TemplateKey, StaticTemplateNode, PatchGeneration)],
		generation: PatchGeneration,
	) {
		let mut inner = self.inner.borrow_mut();
		for (key, overlay, overlay_generation) in overlays {
			if let Some(descriptor) = inner.descriptors.get(key) {
				if descriptor.static_tree == *overlay {
					inner.overlays.remove(key);
				} else {
					inner
						.overlays
						.insert(key.clone(), (overlay.clone(), *overlay_generation));
				}
			}
		}
		inner.latest_generation = generation;
	}

	/// Registers a mounted instance and returns its RAII removal guard.
	pub fn mount_instance(
		&self,
		key: TemplateKey,
		instance: TemplateInstance,
	) -> RegistrationGuard {
		let mut inner = self.inner.borrow_mut();
		let id = inner.next_instance_id;
		inner.next_instance_id = inner.next_instance_id.saturating_add(1);
		inner.instances.insert(id, (key.clone(), instance));
		RegistrationGuard {
			state: Rc::downgrade(&self.inner),
			id,
		}
	}

	/// Applies a validated static overlay for all future and current mounts.
	pub fn apply_overlay(
		&self,
		key: &TemplateKey,
		overlay: StaticTemplateNode,
		generation: PatchGeneration,
	) -> Result<(), RegistryError> {
		let mut inner = self.inner.borrow_mut();
		let Some(descriptor) = inner.descriptors.get(key) else {
			return Err(RegistryError::UnknownTemplate(key.clone()));
		};
		if generation < inner.latest_generation {
			return Err(RegistryError::GenerationBehind(generation));
		}
		if descriptor.static_tree == overlay {
			inner.overlays.remove(key);
		} else {
			inner.overlays.insert(key.clone(), (overlay, generation));
		}
		inner.latest_generation = generation;
		Ok(())
	}

	/// Returns the loaded build identity and all registered dynamic ABIs.
	pub fn client_hello(&self) -> ClientHello {
		let inner = self.inner.borrow();
		ClientHello {
			build_id: inner.build_id,
			manifest_digest: inner.manifest_digest,
			abi_hashes: inner
				.descriptors
				.iter()
				.map(|(key, descriptor)| (key.clone(), descriptor.abi_hash))
				.collect(),
		}
	}

	/// Returns a current overlay for a future mount, if one is installed.
	pub fn overlay_for(&self, key: &TemplateKey) -> Option<StaticTemplateNode> {
		self.inner
			.borrow()
			.overlays
			.get(key)
			.map(|(overlay, _)| overlay.clone())
	}

	/// Returns the overlay tree for a future mount, or the compiled tree when no
	/// overlay has been installed for the template.
	pub fn static_tree_for(&self, key: &TemplateKey) -> Option<StaticTemplateNode> {
		let inner = self.inner.borrow();
		inner
			.overlays
			.get(key)
			.map(|(overlay, _)| overlay.clone())
			.or_else(|| {
				inner
					.descriptors
					.get(key)
					.map(|descriptor| descriptor.static_tree.clone())
			})
	}
}

/// RAII registration token that removes a mounted instance on drop.
#[cfg(wasm)]
pub struct RegistrationGuard {
	state: Weak<RefCell<RegistryInner>>,
	id: u64,
}

#[cfg(wasm)]
impl Drop for RegistrationGuard {
	fn drop(&mut self) {
		if let Some(state) = self.state.upgrade() {
			state.borrow_mut().instances.remove(&self.id);
		}
	}
}

/// Registry validation failures.
#[cfg(wasm)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
	/// The descriptor is not known by this build.
	UnknownTemplate(TemplateKey),
	/// The patch generation is older than the mounted registry state.
	GenerationBehind(PatchGeneration),
	/// The supplied dynamic ABI does not match the descriptor.
	AbiMismatch(DynamicAbiHash),
}
