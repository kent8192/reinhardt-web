//! Scope-owned storage for arena-backed reactive handles.

use core::any::Any;
use core::cell::{Cell, RefCell};
use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;

use super::runtime::{NodeId, try_with_runtime};

/// Identifier for a live reactive scope.
///
/// The identifier records its owner thread so deferred work cannot enter an
/// unrelated worker-local scope that happens to have the same local counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeId {
	value: u64,
	owner_thread: std::thread::ThreadId,
	owner_thread_index: u32,
}

impl PartialOrd for ScopeId {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for ScopeId {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		(self.owner_thread_index, self.value).cmp(&(other.owner_thread_index, other.value))
	}
}

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
	/// A node was accessed while it was mutably borrowed.
	BorrowConflict {
		/// Kind of node being accessed.
		kind: NodeKind,
		/// Scope that owns the node.
		scope: ScopeId,
		/// Arena slot index.
		index: usize,
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
			Self::BorrowConflict { kind, scope, index } => write!(
				f,
				"reactive node borrow conflict: kind={kind:?}, scope={scope:?}, index={index}"
			),
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
	dirty: Cell<bool>,
	disposed: Cell<bool>,
	value: RefCell<Option<Box<dyn Any>>>,
}

struct ScopeState {
	slots: Vec<Rc<CoreSlot>>,
	cleanup: Vec<Box<dyn FnOnce()>>,
	after_node_cleanup: Vec<Box<dyn FnOnce()>>,
}

impl ScopeState {
	fn new() -> Self {
		Self {
			slots: Vec::new(),
			cleanup: Vec::new(),
			after_node_cleanup: Vec::new(),
		}
	}
}

static NEXT_SCOPE_OWNER_INDEX: AtomicU32 = AtomicU32::new(1);

thread_local! {
	static NEXT_SCOPE_ID: Cell<u64> = const { Cell::new(1) };
	static SCOPE_OWNER_INDEX: u32 = NEXT_SCOPE_OWNER_INDEX.fetch_add(1, AtomicOrdering::Relaxed);
	static ACTIVE_SCOPES: RefCell<Vec<ScopeId>> = const { RefCell::new(Vec::new()) };
	static SCOPES: RefCell<BTreeMap<ScopeId, ScopeState>> = const { RefCell::new(BTreeMap::new()) };
}

/// Owner for reactive nodes allocated during a scoped execution.
///
/// A scope is thread-affine and cannot be sent or shared across threads.
pub struct ReactiveScope {
	id: ScopeId,
	disposed: Cell<bool>,
	_thread_bound: PhantomData<Rc<()>>,
}

impl ReactiveScope {
	/// Create an empty reactive scope.
	pub fn new() -> Self {
		let id = NEXT_SCOPE_ID.with(|next| {
			let id = ScopeId {
				value: next.get(),
				owner_thread: std::thread::current().id(),
				owner_thread_index: SCOPE_OWNER_INDEX.with(|index| *index),
			};
			next.set(next.get() + 1);
			id
		});
		SCOPES.with(|scopes| {
			scopes.borrow_mut().insert(id, ScopeState::new());
		});
		Self {
			id,
			disposed: Cell::new(false),
			_thread_bound: PhantomData,
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
		if self.disposed.replace(true) {
			return;
		}

		// A scope retained in thread-local storage can outlive the `SCOPES`
		// thread-local during process or worker teardown.
		let _ = SCOPES.try_with(|_| dispose_scope(self.id));
	}
}

/// Return the current active scope identifier, if any.
pub fn current_scope_id() -> Option<ScopeId> {
	ACTIVE_SCOPES
		.try_with(|active| active.borrow().last().copied())
		.unwrap_or(None)
}

/// Run a closure with an existing live scope active.
///
/// This is intended for deferred callbacks that retain a [`ScopeId`] after
/// their creating render turn has completed. The caller must handle a disposed
/// scope, because the scope can be torn down before a deferred callback runs.
pub fn enter_scope<R>(scope: ScopeId, f: impl FnOnce() -> R) -> Result<R, ReactiveScopeError> {
	let current_thread = std::thread::current().id();
	if scope.owner_thread != current_thread {
		return Err(ReactiveScopeError::WrongThread {
			scope,
			owner_thread: scope.owner_thread,
			current_thread,
		});
	}

	let exists = SCOPES
		.try_with(|scopes| scopes.borrow().contains_key(&scope))
		.unwrap_or(false);
	if !exists {
		return Err(ReactiveScopeError::DisposedScope { scope });
	}

	ACTIVE_SCOPES
		.try_with(|active| active.borrow_mut().push(scope))
		.map_err(|_| ReactiveScopeError::DisposedScope { scope })?;
	struct PopScope;
	impl Drop for PopScope {
		fn drop(&mut self) {
			let _ = ACTIVE_SCOPES.try_with(|active| {
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
		state.slots.push(Rc::new(CoreSlot {
			generation,
			kind,
			node_id,
			dirty: Cell::new(false),
			disposed: Cell::new(false),
			value: RefCell::new(Some(Box::new(value))),
		}));
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
	let slot = lookup_node_slot(key)?;
	ensure_node_is_live(key, &slot)?;
	let value = slot
		.value
		.try_borrow()
		.map_err(|_| ReactiveScopeError::BorrowConflict {
			kind: slot.kind,
			scope: key.scope,
			index: key.index,
		})?;
	let value = value
		.as_ref()
		.and_then(|value| value.downcast_ref::<T>())
		.ok_or(ReactiveScopeError::TypeMismatch {
			kind: slot.kind,
			type_name: core::any::type_name::<T>(),
		})?;
	Ok(f(value))
}

pub(crate) fn with_node_mut<T: 'static, R>(
	key: NodeKey,
	f: impl FnOnce(&mut T) -> R,
) -> Result<R, ReactiveScopeError> {
	let slot = lookup_node_slot(key)?;
	ensure_node_is_live(key, &slot)?;
	let mut value =
		slot.value
			.try_borrow_mut()
			.map_err(|_| ReactiveScopeError::BorrowConflict {
				kind: slot.kind,
				scope: key.scope,
				index: key.index,
			})?;
	let value = value
		.as_mut()
		.and_then(|value| value.downcast_mut::<T>())
		.ok_or(ReactiveScopeError::TypeMismatch {
			kind: slot.kind,
			type_name: core::any::type_name::<T>(),
		})?;
	Ok(f(value))
}

fn lookup_node_slot(key: NodeKey) -> Result<Rc<CoreSlot>, ReactiveScopeError> {
	ensure_owner_thread(key)?;
	SCOPES.with(|scopes| {
		let scopes = scopes.borrow();
		let state = scopes
			.get(&key.scope)
			.ok_or_else(|| disposed_node_error(key, key.kind, None))?;
		let slot = state
			.slots
			.get(key.index)
			.map(Rc::clone)
			.ok_or_else(|| disposed_node_error(key, key.kind, None))?;
		if slot.generation != key.generation {
			return Err(disposed_node_error(key, slot.kind, Some(slot.generation)));
		}
		Ok(slot)
	})
}

fn ensure_node_is_live(key: NodeKey, slot: &CoreSlot) -> Result<(), ReactiveScopeError> {
	if slot.disposed.get() {
		return Err(disposed_node_error(key, slot.kind, Some(slot.generation)));
	}
	Ok(())
}

fn disposed_node_error(
	key: NodeKey,
	kind: NodeKind,
	actual_generation: Option<u32>,
) -> ReactiveScopeError {
	ReactiveScopeError::DisposedNode {
		kind,
		scope: key.scope,
		index: key.index,
		expected_generation: key.generation,
		actual_generation,
	}
}

pub(crate) fn find_node_key(node_id: NodeId, kind: NodeKind) -> Option<NodeKey> {
	SCOPES.with(|scopes| {
		let scopes = scopes.borrow();
		for (&scope, state) in scopes.iter() {
			for (index, slot) in state.slots.iter().enumerate() {
				let Ok(value) = slot.value.try_borrow() else {
					continue;
				};
				if slot.node_id == node_id
					&& slot.kind == kind
					&& !slot.disposed.get()
					&& value.is_some()
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
	let slot = lookup_node_slot(key)?;
	ensure_node_is_live(key, &slot)?;
	Ok(slot.dirty.get())
}

pub(crate) fn set_node_dirty(key: NodeKey, dirty: bool) -> Result<(), ReactiveScopeError> {
	let slot = lookup_node_slot(key)?;
	ensure_node_is_live(key, &slot)?;
	slot.dirty.set(dirty);
	Ok(())
}

pub(crate) fn mark_node_disposed(key: NodeKey) -> Result<(), ReactiveScopeError> {
	ensure_owner_thread(key)?;
	SCOPES.with(|scopes| {
		let scopes = scopes.borrow();
		let state = scopes
			.get(&key.scope)
			.ok_or(ReactiveScopeError::DisposedScope { scope: key.scope })?;
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
		slot.disposed.set(true);
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

/// Register a callback to run after the scope's stored nodes are dropped.
///
/// This is intended for teardown that may be observed by effect cleanup
/// callbacks while the scope is disposing.
pub fn on_scope_dispose_after_nodes(
	scope: ScopeId,
	cleanup: impl FnOnce() + 'static,
) -> Result<(), ReactiveScopeError> {
	SCOPES.with(|scopes| {
		let mut scopes = scopes.borrow_mut();
		let state = scopes
			.get_mut(&scope)
			.ok_or(ReactiveScopeError::DisposedScope { scope })?;
		state.after_node_cleanup.push(Box::new(cleanup));
		Ok(())
	})
}

fn take_scope_node_value(scope: ScopeId, kind: Option<NodeKind>) -> Option<Box<dyn Any>> {
	let slot = SCOPES.with(|scopes| {
		let scopes = scopes.borrow();
		let state = scopes.get(&scope)?;
		state.slots.iter().find_map(|slot| {
			let matches_kind = kind.is_none_or(|kind| slot.kind == kind);
			(matches_kind && slot.value.borrow().is_some()).then(|| Rc::clone(slot))
		})
	});
	slot.and_then(|slot| slot.value.borrow_mut().take())
}

pub(crate) fn dispose_scope(scope: ScopeId) {
	let is_live = SCOPES
		.try_with(|scopes| scopes.borrow().contains_key(&scope))
		.unwrap_or(false);
	if !is_live {
		return;
	}

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

	// Effect cleanup callbacks may read signals captured by the effect. Drop those
	// slots while every non-effect node remains available in the scope arena.
	while let Some(value) = take_scope_node_value(scope, Some(NodeKind::Effect)) {
		drop(value);
	}

	loop {
		let value = take_scope_node_value(scope, None);
		let Some(value) = value else {
			break;
		};
		drop(value);
	}

	loop {
		let cleanup = SCOPES.with(|scopes| {
			scopes
				.borrow_mut()
				.get_mut(&scope)
				.map(|state| core::mem::take(&mut state.after_node_cleanup))
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
	use super::super::{deps::Deps, effect::Effect, signal::Signal};
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

	#[cfg(not(target_arch = "wasm32"))]
	#[test]
	#[serial(reactive_runtime)]
	fn entering_a_same_numbered_worker_scope_is_rejected() {
		let scope = ReactiveScope::new();
		let scope_id = scope.id();

		let result = std::thread::spawn(move || {
			let _worker_scope = ReactiveScope::new();
			enter_scope(scope_id, || ())
		})
		.join()
		.expect("worker thread should finish without panicking");

		assert!(matches!(
			result,
			Err(ReactiveScopeError::WrongThread { .. })
		));
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

	#[test]
	#[serial(reactive_runtime)]
	fn scope_effect_cleanup_can_read_captured_signal_before_node_drop() {
		let scope = ReactiveScope::new();
		let observed = alloc::rc::Rc::new(Cell::new(None));
		let observed_for_effect = alloc::rc::Rc::clone(&observed);

		scope.enter(|| {
			let signal = Signal::new(42_i32);
			let signal_for_cleanup = signal;
			let _effect = Effect::new_with_deps(
				move || {
					let observed_for_cleanup = alloc::rc::Rc::clone(&observed_for_effect);
					Some(move || {
						observed_for_cleanup.set(Some(signal_for_cleanup.get_untracked()));
					})
				},
				Deps::from_signals(&[]),
			);
		});

		scope.dispose();

		assert_eq!(observed.get(), Some(42));
	}

	#[test]
	#[serial(reactive_runtime)]
	fn scope_node_reads_can_reenter_the_same_signal() {
		ReactiveScope::run(|| {
			let signal = Signal::new(42_i32);

			let observed = signal.with_untracked(|_| signal.get_untracked());

			assert_eq!(observed, 42);
		});
	}

	#[test]
	#[serial(reactive_runtime)]
	fn scope_post_node_cleanup_runs_after_node_values_drop() {
		struct DropMarker(alloc::rc::Rc<Cell<bool>>);

		impl Drop for DropMarker {
			fn drop(&mut self) {
				self.0.set(true);
			}
		}

		let scope = ReactiveScope::new();
		let dropped = alloc::rc::Rc::new(Cell::new(false));
		let observed = alloc::rc::Rc::new(Cell::new(false));
		let dropped_for_node = alloc::rc::Rc::clone(&dropped);
		let dropped_for_cleanup = alloc::rc::Rc::clone(&dropped);
		let observed_for_cleanup = alloc::rc::Rc::clone(&observed);
		let scope_id = scope.id();

		scope.enter(|| {
			allocate_node(NodeKind::Signal, DropMarker(dropped_for_node));
			on_scope_dispose_after_nodes(scope_id, move || {
				observed_for_cleanup.set(dropped_for_cleanup.get());
			})
			.expect("live scope should accept post-node cleanup callbacks");
		});

		scope.dispose();

		assert!(observed.get());
	}
}
