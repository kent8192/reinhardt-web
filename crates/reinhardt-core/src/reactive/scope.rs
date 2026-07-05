//! Scope-owned storage for arena-backed reactive handles.

use core::any::Any;
use core::cell::{Cell, RefCell};
use core::fmt;

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::runtime::{NodeId, try_with_runtime};

/// Identifier for a live reactive scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(u64);

/// Generational key for a node stored inside a reactive scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey {
	scope: ScopeId,
	index: usize,
	generation: u32,
	node_id: NodeId,
	kind: NodeKind,
}

/// Kind of reactive node stored in a scope arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
	/// Signal source node.
	Signal,
	/// Memoized derived node.
	Memo,
	/// Effect observer node.
	Effect,
}

/// Errors produced by scope-owned reactive node access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactiveScopeError {
	/// An operation required an active scope but none was present.
	NoActiveScope {
		/// Operation that attempted to access the active scope.
		operation: &'static str,
	},
	/// A disposed scope was accessed.
	DisposedScope {
		/// Disposed scope identifier.
		scope: ScopeId,
	},
	/// A node key points at disposed or stale storage.
	DisposedNode {
		/// Kind of node being accessed.
		kind: NodeKind,
		/// Scope that owned the node.
		scope: ScopeId,
		/// Arena slot index.
		index: usize,
		/// Generation captured in the key.
		expected_generation: u32,
		/// Current slot generation, if the slot still exists.
		actual_generation: Option<u32>,
	},
	/// A node key was read as the wrong concrete value type.
	TypeMismatch {
		/// Kind of node being accessed.
		kind: NodeKind,
		/// Requested Rust type name.
		type_name: &'static str,
	},
}

impl fmt::Display for ReactiveScopeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NoActiveScope { operation } => write!(
				f,
				"{operation} requires an active ReactiveScope; wrap low-level code in ReactiveScope::run(...) or use a Reinhardt Pages entrypoint"
			),
			Self::DisposedScope { scope } => {
				write!(f, "disposed reactive scope access: scope={scope:?}")
			}
			Self::DisposedNode {
				kind,
				scope,
				index,
				expected_generation,
				actual_generation,
			} => write!(
				f,
				"disposed reactive node access: kind={kind:?}, scope={scope:?}, index={index}, expected_generation={expected_generation}, actual_generation={actual_generation:?}"
			),
			Self::TypeMismatch { kind, type_name } => {
				write!(
					f,
					"reactive node type mismatch: kind={kind:?}, requested={type_name}"
				)
			}
		}
	}
}

impl std::error::Error for ReactiveScopeError {}

struct CoreSlot {
	generation: u32,
	kind: NodeKind,
	node_id: NodeId,
	value: Option<Box<dyn Any>>,
}

struct ScopeState {
	slots: Vec<CoreSlot>,
	cleanup: Vec<Box<dyn FnOnce()>>,
}

impl ScopeState {
	fn new() -> Self {
		Self {
			slots: Vec::new(),
			cleanup: Vec::new(),
		}
	}
}

thread_local! {
	static NEXT_SCOPE_ID: Cell<u64> = const { Cell::new(1) };
	static ACTIVE_SCOPES: RefCell<Vec<ScopeId>> = const { RefCell::new(Vec::new()) };
	static SCOPES: RefCell<BTreeMap<ScopeId, ScopeState>> = const { RefCell::new(BTreeMap::new()) };
}

/// Owner for reactive nodes allocated during a scoped execution.
pub struct ReactiveScope {
	id: ScopeId,
	disposed: Cell<bool>,
}

impl ReactiveScope {
	/// Create an empty reactive scope.
	pub fn new() -> Self {
		let id = NEXT_SCOPE_ID.with(|next| {
			let id = ScopeId(next.get());
			next.set(next.get() + 1);
			id
		});
		SCOPES.with(|scopes| {
			scopes.borrow_mut().insert(id, ScopeState::new());
		});
		Self {
			id,
			disposed: Cell::new(false),
		}
	}

	/// Return this scope's identifier.
	pub fn id(&self) -> ScopeId {
		self.id
	}

	/// Run a closure inside a fresh reactive scope.
	pub fn run<R>(f: impl FnOnce() -> R) -> R {
		let scope = Self::new();
		scope.enter(f)
	}

	/// Run a closure with this scope as the current active scope.
	pub fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
		ACTIVE_SCOPES.with(|active| active.borrow_mut().push(self.id));
		struct PopScope;
		impl Drop for PopScope {
			fn drop(&mut self) {
				ACTIVE_SCOPES.with(|active| {
					active.borrow_mut().pop();
				});
			}
		}
		let _guard = PopScope;
		f()
	}

	/// Dispose this scope and run registered cleanup callbacks.
	pub fn dispose(&self) {
		if self.disposed.replace(true) {
			return;
		}
		dispose_scope(self.id);
	}
}

impl Drop for ReactiveScope {
	fn drop(&mut self) {
		self.dispose();
	}
}

/// Return the current active scope identifier, if any.
pub fn current_scope_id() -> Option<ScopeId> {
	ACTIVE_SCOPES.with(|active| active.borrow().last().copied())
}

/// Return the current active scope identifier or panic with diagnostics.
pub fn active_scope_id() -> ScopeId {
	require_active_scope("active_scope_id")
}

/// Return the current active scope identifier or panic for the given operation.
pub fn require_active_scope(operation: &'static str) -> ScopeId {
	current_scope_id().unwrap_or_else(|| {
		panic!("{}", ReactiveScopeError::NoActiveScope { operation });
	})
}

pub(crate) fn allocate_node<T: 'static>(kind: NodeKind, value: T) -> NodeKey {
	let scope = require_active_scope("reactive node creation");
	SCOPES.with(|scopes| {
		let mut scopes = scopes.borrow_mut();
		let state = scopes
			.get_mut(&scope)
			.expect("active ReactiveScope must have scope state");
		let index = state.slots.len();
		let node_id = NodeId::new();
		let generation = 1;
		state.slots.push(CoreSlot {
			generation,
			kind,
			node_id,
			value: Some(Box::new(value)),
		});
		NodeKey {
			scope,
			index,
			generation,
			node_id,
			kind,
		}
	})
}

#[cfg(test)]
pub(crate) fn allocate_empty_node(kind: NodeKind) -> NodeKey {
	allocate_node(kind, ())
}

#[cfg(test)]
pub(crate) fn lookup_empty_node(key: NodeKey) -> Result<(), ReactiveScopeError> {
	with_node::<(), _>(key, |_| ())
}

pub(crate) fn with_node<T: 'static, R>(
	key: NodeKey,
	f: impl FnOnce(&T) -> R,
) -> Result<R, ReactiveScopeError> {
	SCOPES.with(|scopes| {
		let scopes = scopes.borrow();
		let state = scopes
			.get(&key.scope)
			.ok_or(ReactiveScopeError::DisposedNode {
				kind: key.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: None,
			})?;
		let slot = state
			.slots
			.get(key.index)
			.ok_or(ReactiveScopeError::DisposedNode {
				kind: key.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: None,
			})?;
		if slot.generation != key.generation {
			return Err(ReactiveScopeError::DisposedNode {
				kind: slot.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: Some(slot.generation),
			});
		}
		let value = slot
			.value
			.as_ref()
			.and_then(|value| value.downcast_ref::<T>())
			.ok_or(ReactiveScopeError::TypeMismatch {
				kind: slot.kind,
				type_name: core::any::type_name::<T>(),
			})?;
		Ok(f(value))
	})
}

pub(crate) fn with_node_mut<T: 'static, R>(
	key: NodeKey,
	f: impl FnOnce(&mut T) -> R,
) -> Result<R, ReactiveScopeError> {
	SCOPES.with(|scopes| {
		let mut scopes = scopes.borrow_mut();
		let state = scopes
			.get_mut(&key.scope)
			.ok_or(ReactiveScopeError::DisposedNode {
				kind: key.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: None,
			})?;
		let slot = state
			.slots
			.get_mut(key.index)
			.ok_or(ReactiveScopeError::DisposedNode {
				kind: key.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: None,
			})?;
		if slot.generation != key.generation {
			return Err(ReactiveScopeError::DisposedNode {
				kind: slot.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: Some(slot.generation),
			});
		}
		let value = slot
			.value
			.as_mut()
			.and_then(|value| value.downcast_mut::<T>())
			.ok_or(ReactiveScopeError::TypeMismatch {
				kind: slot.kind,
				type_name: core::any::type_name::<T>(),
			})?;
		Ok(f(value))
	})
}

/// Register a callback to run when the given scope is disposed.
pub fn on_scope_dispose(
	scope: ScopeId,
	cleanup: impl FnOnce() + 'static,
) -> Result<(), ReactiveScopeError> {
	SCOPES.with(|scopes| {
		let mut scopes = scopes.borrow_mut();
		let state = scopes
			.get_mut(&scope)
			.ok_or(ReactiveScopeError::DisposedScope { scope })?;
		state.cleanup.push(Box::new(cleanup));
		Ok(())
	})
}

pub(crate) fn dispose_scope(scope: ScopeId) {
	let state = SCOPES.with(|scopes| scopes.borrow_mut().remove(&scope));
	if let Some(mut state) = state {
		for cleanup in state.cleanup.drain(..).rev() {
			cleanup();
		}
		let slots = core::mem::take(&mut state.slots);
		for slot in slots {
			let _ = try_with_runtime(|runtime| runtime.remove_node(slot.node_id));
			drop(slot.value);
		}
	}
}

impl NodeKey {
	/// Return the runtime node identifier linked to this key.
	pub fn node_id(self) -> NodeId {
		self.node_id
	}

	/// Return the scope that owns this key.
	pub fn scope(self) -> ScopeId {
		self.scope
	}

	/// Return the generation captured by this key.
	pub fn generation(self) -> u32 {
		self.generation
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serial_test::serial;

	#[test]
	#[serial(reactive_runtime)]
	fn reactive_scope_run_sets_current_scope() {
		ReactiveScope::run(|| {
			let id = current_scope_id().expect("scope must be active inside run");
			assert_eq!(active_scope_id(), id);
		});
		assert!(current_scope_id().is_none());
	}

	#[test]
	#[serial(reactive_runtime)]
	#[should_panic(expected = "requires an active ReactiveScope")]
	fn require_active_scope_panics_without_scope() {
		let _ = require_active_scope("Signal::new");
	}

	#[test]
	#[serial(reactive_runtime)]
	fn disposed_generation_is_detected() {
		let key = ReactiveScope::run(|| allocate_empty_node(NodeKind::Signal));
		let err = lookup_empty_node(key).expect_err("scope was disposed after run");
		assert!(err.to_string().contains("disposed reactive node"));
	}

	#[test]
	#[serial(reactive_runtime)]
	fn scope_dispose_runs_registered_cleanup() {
		let cleaned = alloc::rc::Rc::new(RefCell::new(false));
		let cleaned_for_scope = alloc::rc::Rc::clone(&cleaned);
		ReactiveScope::run(|| {
			let scope = current_scope_id().expect("scope must be active");
			on_scope_dispose(scope, move || {
				*cleaned_for_scope.borrow_mut() = true;
			})
			.expect("active scope should accept cleanup callback");
		});
		assert!(
			*cleaned.borrow(),
			"scope disposal must run cleanup callbacks"
		);
	}
}
