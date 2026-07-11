//! Scope-owned storage for Pages reactive handles.

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;

use reinhardt_core::reactive::{ReactiveScopeError, ScopeId, on_scope_dispose};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
// Action and Resource slots are consumed by the next migration layer.
#[allow(dead_code)]
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
			},
			register_cleanup,
		)
	});
	if register_cleanup {
		on_scope_dispose(scope, move || dispose_pages_scope(scope))
			.unwrap_or_else(|err| panic!("{err}"));
	}
	key
}

pub(crate) fn with_page_node<T: 'static, R>(
	key: PageNodeKey,
	f: impl FnOnce(&T) -> R,
) -> Result<R, String> {
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

// Mutation becomes live when Action and Resource move into the arena.
#[allow(dead_code)]
pub(crate) fn with_page_node_mut<T: 'static, R>(
	key: PageNodeKey,
	f: impl FnOnce(&mut T) -> R,
) -> Result<R, String> {
	PAGES_ARENAS.with(|arenas| {
		let mut arenas = arenas.borrow_mut();
		let arena = arenas
			.get_mut(&key.scope)
			.ok_or_else(|| stale_error(key, None))?;
		let slot = arena
			.slots
			.get_mut(key.index)
			.ok_or_else(|| stale_error(key, None))?;
		if slot.generation != key.generation {
			return Err(stale_error(key, Some(slot.generation)));
		}
		let value = slot
			.value
			.as_mut()
			.and_then(|value| value.downcast_mut::<T>())
			.ok_or_else(|| format!("pages reactive node type mismatch: kind={:?}", slot.kind))?;
		Ok(f(value))
	})
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
