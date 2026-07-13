//! Scope-owned storage for Pages reactive handles.

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;

use reinhardt_core::reactive::{ReactiveScopeError, ScopeId, on_scope_dispose_after_nodes};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PageNodeKind {
	Callback,
	Action,
	Resource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageNodeKey {
	scope: ScopeId,
	index: usize,
	generation: u32,
	kind: PageNodeKind,
	owner_thread: std::thread::ThreadId,
}

impl PageNodeKey {
	pub(crate) fn scope(self) -> ScopeId {
		self.scope
	}
}

struct PageSlot {
	generation: u32,
	kind: PageNodeKind,
	value: Option<Box<dyn Any>>,
}

#[derive(Default)]
struct PageArena {
	slots: Vec<PageSlot>,
}

thread_local! {
	static PAGES_ARENAS: RefCell<BTreeMap<ScopeId, PageArena>> = const { RefCell::new(BTreeMap::new()) };
}

pub(crate) fn allocate_page_node<T: 'static>(
	operation: &'static str,
	kind: PageNodeKind,
	value: T,
) -> PageNodeKey {
	let scope = reinhardt_core::reactive::scope::current_scope_id().unwrap_or_else(|| {
		panic!("{}", ReactiveScopeError::NoActiveScope { operation });
	});
	let (key, register_cleanup) = PAGES_ARENAS.with(|arenas| {
		let mut arenas = arenas.borrow_mut();
		let register_cleanup = !arenas.contains_key(&scope);
		let arena = arenas.entry(scope).or_default();
		let index = arena.slots.len();
		let generation = 1;
		arena.slots.push(PageSlot {
			generation,
			kind,
			value: Some(Box::new(value)),
		});
		(
			PageNodeKey {
				scope,
				index,
				generation,
				kind,
				owner_thread: std::thread::current().id(),
			},
			register_cleanup,
		)
	});
	if register_cleanup {
		on_scope_dispose_after_nodes(scope, move || dispose_pages_scope(scope))
			.unwrap_or_else(|err| panic!("{err}"));
	}
	key
}

pub(crate) fn with_page_node<T: 'static, R>(
	key: PageNodeKey,
	f: impl FnOnce(&T) -> R,
) -> Result<R, String> {
	ensure_owner_thread(key)?;
	PAGES_ARENAS.with(|arenas| {
		let arenas = arenas.borrow();
		let arena = arenas
			.get(&key.scope)
			.ok_or_else(|| stale_error(key, None))?;
		let slot = arena
			.slots
			.get(key.index)
			.ok_or_else(|| stale_error(key, None))?;
		if slot.generation != key.generation {
			return Err(stale_error(key, Some(slot.generation)));
		}
		let value = slot
			.value
			.as_ref()
			.and_then(|value| value.downcast_ref::<T>())
			.ok_or_else(|| format!("pages reactive node type mismatch: kind={:?}", slot.kind))?;
		Ok(f(value))
	})
}

fn ensure_owner_thread(key: PageNodeKey) -> Result<(), String> {
	let current_thread = std::thread::current().id();
	if key.owner_thread == current_thread {
		Ok(())
	} else {
		Err(format!(
			"pages reactive node accessed from a different thread: scope={:?}, owner_thread={:?}, current_thread={current_thread:?}",
			key.scope, key.owner_thread
		))
	}
}

fn stale_error(key: PageNodeKey, actual_generation: Option<u32>) -> String {
	format!(
		"disposed pages reactive node access: kind={:?}, scope={:?}, index={}, expected_generation={}, actual_generation={actual_generation:?}",
		key.kind, key.scope, key.index, key.generation
	)
}

fn dispose_pages_scope(scope: ScopeId) {
	PAGES_ARENAS.with(|arenas| {
		arenas.borrow_mut().remove(&scope);
	});
}

#[cfg(all(test, native))]
mod tests {
	use super::*;
	use serial_test::serial;

	#[test]
	#[serial(reactive_runtime)]
	fn page_node_rejects_access_from_a_different_thread() {
		let scope = reinhardt_core::reactive::ReactiveScope::new();
		let key =
			scope.enter(|| allocate_page_node("test page node", PageNodeKind::Callback, 1_i32));

		let result = std::thread::spawn(move || with_page_node::<i32, _>(key, |value| *value))
			.join()
			.expect("worker thread should finish without panicking");

		assert!(
			result
				.expect_err("page node access from another thread must fail")
				.contains("different thread")
		);
	}
}
