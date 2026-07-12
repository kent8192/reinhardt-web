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
	owner_thread: std::thread::ThreadId,
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
	/// A key was accessed from a thread other than the one that owns its scope.
	WrongThread {
		/// Scope that owns the key.
		scope: ScopeId,
		/// Thread that created the key.
		owner_thread: std::thread::ThreadId,
		/// Thread that attempted to access the key.
		current_thread: std::thread::ThreadId,
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
			Self::WrongThread {
				scope,
				owner_thread,
				current_thread,
			} => write!(
				f,
				"reactive scope accessed from a different thread: scope={scope:?}, owner_thread={owner_thread:?}, current_thread={current_thread:?}"
			),
		}
	}
}

impl std::error::Error for ReactiveScopeError {}

struct CoreSlot {
	generation: u32,
	kind: NodeKind,
	node_id: NodeId,
	dirty: bool,
	disposed: bool,
	value: Option<Box<dyn Any>>,
}

struct ScopeState {
	slots: Vec<CoreSlot>,
	cleanup: Vec<Box<dyn FnOnce()>>,
}

struct TakenNodeValue<T: 'static> {
	key: NodeKey,
	value: Option<Box<T>>,
}

impl<T: 'static> Drop for TakenNodeValue<T> {
	fn drop(&mut self) {
		if let Some(value) = self.value.take() {
			restore_node_value(self.key, value);
		}
	}
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
		enter_scope(self.id, f).unwrap_or_else(|err| panic!("{err}"))
	}

	/// Dispose this scope and run registered cleanup callbacks.
	pub fn dispose(&self) {
		if self.disposed.replace(true) {
			return;
		}
		dispose_scope(self.id);
	}
}

impl Default for ReactiveScope {
	fn default() -> Self {
		Self::new()
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

pub(crate) fn enter_scope<R>(
	scope: ScopeId,
	f: impl FnOnce() -> R,
) -> Result<R, ReactiveScopeError> {
	let exists = SCOPES.with(|scopes| scopes.borrow().contains_key(&scope));
	if !exists {
		return Err(ReactiveScopeError::DisposedScope { scope });
	}

	ACTIVE_SCOPES.with(|active| active.borrow_mut().push(scope));
	struct PopScope;
	impl Drop for PopScope {
		fn drop(&mut self) {
			ACTIVE_SCOPES.with(|active| {
				active.borrow_mut().pop();
			});
		}
	}
	let _guard = PopScope;
	Ok(f())
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
			dirty: false,
			disposed: false,
			value: Some(Box::new(value)),
		});
		NodeKey {
			scope,
			index,
			generation,
			node_id,
			kind,
			owner_thread: std::thread::current().id(),
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
	ensure_owner_thread(key)?;
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
		if slot.disposed {
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
	ensure_owner_thread(key)?;
	let value = SCOPES.with(|scopes| {
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
		if slot.disposed {
			return Err(ReactiveScopeError::DisposedNode {
				kind: slot.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: Some(slot.generation),
			});
		}
		slot.value.take().ok_or(ReactiveScopeError::TypeMismatch {
			kind: slot.kind,
			type_name: core::any::type_name::<T>(),
		})
	})?;

	let value = match value.downcast::<T>() {
		Ok(value) => value,
		Err(value) => {
			restore_node_value(key, value);
			return Err(ReactiveScopeError::TypeMismatch {
				kind: key.kind,
				type_name: core::any::type_name::<T>(),
			});
		}
	};
	let mut value = TakenNodeValue {
		key,
		value: Some(value),
	};
	Ok(f(value.value.as_deref_mut().expect(
		"taken node value must remain available during mutation",
	)))
}

fn restore_node_value(key: NodeKey, value: Box<dyn Any>) {
	if ensure_owner_thread(key).is_err() {
		return;
	}
	SCOPES.with(|scopes| {
		let Ok(mut scopes) = scopes.try_borrow_mut() else {
			return;
		};
		let Some(state) = scopes.get_mut(&key.scope) else {
			return;
		};
		let Some(slot) = state.slots.get_mut(key.index) else {
			return;
		};
		if slot.generation == key.generation && !slot.disposed && slot.value.is_none() {
			slot.value = Some(value);
		}
	});
}

pub(crate) fn find_node_key(node_id: NodeId, kind: NodeKind) -> Option<NodeKey> {
	SCOPES.with(|scopes| {
		let scopes = scopes.borrow();
		for (&scope, state) in scopes.iter() {
			for (index, slot) in state.slots.iter().enumerate() {
				if slot.node_id == node_id
					&& slot.kind == kind
					&& !slot.disposed
					&& slot.value.is_some()
				{
					return Some(NodeKey {
						scope,
						index,
						generation: slot.generation,
						node_id,
						kind,
						owner_thread: std::thread::current().id(),
					});
				}
			}
		}
		None
	})
}

pub(crate) fn node_is_dirty(key: NodeKey) -> Result<bool, ReactiveScopeError> {
	ensure_owner_thread(key)?;
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
		if slot.generation != key.generation || slot.disposed {
			return Err(ReactiveScopeError::DisposedNode {
				kind: slot.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: Some(slot.generation),
			});
		}
		Ok(slot.dirty)
	})
}

pub(crate) fn set_node_dirty(key: NodeKey, dirty: bool) -> Result<(), ReactiveScopeError> {
	ensure_owner_thread(key)?;
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
		if slot.generation != key.generation || slot.disposed {
			return Err(ReactiveScopeError::DisposedNode {
				kind: slot.kind,
				scope: key.scope,
				index: key.index,
				expected_generation: key.generation,
				actual_generation: Some(slot.generation),
			});
		}
		slot.dirty = dirty;
		Ok(())
	})
}

pub(crate) fn mark_node_disposed(key: NodeKey) -> Result<(), ReactiveScopeError> {
	ensure_owner_thread(key)?;
	SCOPES.with(|scopes| {
		let mut scopes = scopes.borrow_mut();
		let state = scopes
			.get_mut(&key.scope)
			.ok_or(ReactiveScopeError::DisposedScope { scope: key.scope })?;
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
		slot.disposed = true;
		Ok(())
	})
}

fn ensure_owner_thread(key: NodeKey) -> Result<(), ReactiveScopeError> {
	let current_thread = std::thread::current().id();
	if key.owner_thread == current_thread {
		Ok(())
	} else {
		Err(ReactiveScopeError::WrongThread {
			scope: key.scope,
			owner_thread: key.owner_thread,
			current_thread,
		})
	}
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
	loop {
		let cleanup = SCOPES.with(|scopes| {
			scopes
				.borrow_mut()
				.get_mut(&scope)
				.map(|state| core::mem::take(&mut state.cleanup))
		});
		let Some(cleanup) = cleanup else {
			return;
		};
		if cleanup.is_empty() {
			break;
		}
		for cleanup in cleanup.into_iter().rev() {
			cleanup();
		}
	}

	let node_ids = SCOPES.with(|scopes| {
		scopes
			.borrow()
			.get(&scope)
			.map(|state| {
				state
					.slots
					.iter()
					.map(|slot| slot.node_id)
					.collect::<Vec<_>>()
			})
			.unwrap_or_default()
	});
	for node_id in node_ids {
		let _ = try_with_runtime(|runtime| runtime.remove_node(node_id));
	}

	loop {
		let value = SCOPES.with(|scopes| {
			scopes
				.borrow_mut()
				.get_mut(&scope)
				.and_then(|state| state.slots.iter_mut().find_map(|slot| slot.value.take()))
		});
		let Some(value) = value else {
			break;
		};
		drop(value);
	}

	SCOPES.with(|scopes| {
		scopes.borrow_mut().remove(&scope);
	});
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

	#[test]
	#[serial(reactive_runtime)]
	fn scope_cleanup_can_read_live_nodes() {
		let scope = ReactiveScope::new();
		let observed = alloc::rc::Rc::new(Cell::new(None));
		let observed_for_cleanup = alloc::rc::Rc::clone(&observed);
		let scope_id = scope.id();

		scope.enter(|| {
			let signal = super::super::signal::Signal::new(42_i32);
			on_scope_dispose(scope_id, move || {
				observed_for_cleanup.set(Some(signal.get()));
			})
			.expect("live scope should accept cleanup callbacks");
		});

		scope.dispose();

		assert_eq!(observed.get(), Some(42));
	}
}
