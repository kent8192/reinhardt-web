//! Testing-only helpers for the dependency registry.
//!
//! This module is compiled only when the `testing` feature is enabled.
//! Everything exported here is intended for use by `reinhardt-testkit` and
//! downstream test code; it is **not** part of the public production API.

use std::any::TypeId;
use std::sync::Weak;

use crate::registry::{DependencyRegistry, DependencyScope, FactoryTrait};

/// Captures the previous registration for a type so it can be restored on drop.
///
/// Returned by [`DependencyRegistry::register_override`]. While the guard is
/// alive the override is active; when it is dropped the previous factory and
/// scope are restored (or the entry is removed entirely if there was none).
///
/// When used with a per-context registry (via
/// `InjectionContextBuilder::with_registry`), `#[serial(di_registry)]` is
/// not required. When used with the global registry,
/// `#[serial(di_registry)]` is still required.
pub struct OverrideGuard {
	pub(crate) type_id: TypeId,
	pub(crate) previous: Option<(Box<dyn FactoryTrait>, DependencyScope)>,
	pub(crate) registry: Weak<DependencyRegistry>,
}

impl OverrideGuard {
	/// Returns the `TypeId` whose factory this guard is restoring.
	pub fn type_id(&self) -> TypeId {
		self.type_id
	}
}

impl Drop for OverrideGuard {
	fn drop(&mut self) {
		// If the registry has already been dropped (only possible once
		// per-context registries land — see Approach C), silently bail.
		let Some(registry) = self.registry.upgrade() else {
			return;
		};

		// Take the previous entry out of `self` so we don't try to use it
		// after this Drop run completes.
		match self.previous.take() {
			Some((factory, scope)) => {
				registry.restore_override(self.type_id, factory, scope);
			}
			None => {
				registry.remove_override(self.type_id);
			}
		}
	}
}

// Compile-time check that `OverrideGuard` stays `Send + Sync`.
// `tokio::test` requires guards held across `.await` points to be `Send`,
// and parallel-test infrastructure expects `Sync`. Adding a non-`Send`
// field in the future will break this assertion and prompt a re-evaluation.
const _: fn() = || {
	fn _assert<T: Send + Sync>() {}
	_assert::<OverrideGuard>();
};
