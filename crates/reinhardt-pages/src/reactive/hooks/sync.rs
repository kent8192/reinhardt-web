//! External store hook: use_sync_external_store
//!
//! This hook provides a way to subscribe to external data stores.

use std::rc::Rc;

use crate::reactive::Signal;

/// Subscribes to an external store.
///
/// This is the React-like equivalent of `useSyncExternalStore`. It allows you
/// to subscribe to external data sources like Redux stores, browser APIs, or
/// any other mutable state that exists outside the reactive system.
///
/// # Type Parameters
///
/// * `T` - The type of value in the store
/// * `S` - The subscribe function type
/// * `G` - The get snapshot function type
///
/// # Arguments
///
/// * `subscribe` - A function that subscribes to the store and returns an unsubscribe function
/// * `get_snapshot` - A function that returns the current value from the store
///
/// # Returns
///
/// A `Signal<T>` that stays in sync with the external store
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::use_sync_external_store;
///
/// // Subscribe to browser online status
/// let is_online = use_sync_external_store(
///     // Subscribe function
///     |on_change| {
///         window().add_event_listener("online", on_change.clone());
///         window().add_event_listener("offline", on_change.clone());
///
///         // Return unsubscribe
///         Box::new(move || {
///             window().remove_event_listener("online", on_change.clone());
///             window().remove_event_listener("offline", on_change);
///         })
///     },
///     // Get current value
///     || navigator().on_line(),
/// );
///
/// if is_online.get() {
///     // ... online content
/// } else {
///     // ... offline content
/// }
/// ```
///
/// # When to Use
///
/// Use this hook when you need to:
/// - Subscribe to browser APIs (window size, online status, etc.)
/// - Integrate with external state management libraries
/// - Read from mutable sources that update outside React
///
/// # Note
///
/// The `get_snapshot` function should return consistent results when called
/// multiple times in a row. If the store's data changes, it should return
/// a different reference (for objects) or value (for primitives).
pub fn use_sync_external_store<T, S, G>(subscribe: S, get_snapshot: G) -> Signal<T>
where
	T: Clone + PartialEq + 'static,
	S: FnOnce(Rc<dyn Fn()>) -> Box<dyn FnOnce()> + 'static,
	G: Fn() -> T + 'static,
{
	let state = Signal::new(get_snapshot());

	// Set up the subscription
	let state_clone = state.clone();
	let get_snapshot = Rc::new(get_snapshot);
	let get_snapshot_clone = Rc::clone(&get_snapshot);

	let on_change: Rc<dyn Fn()> = Rc::new({
		let state = state_clone.clone();
		move || {
			let new_value = get_snapshot_clone();
			if state.get() != new_value {
				state.set(new_value);
			}
		}
	});

	// Subscribe and store the unsubscribe function
	let _unsubscribe = subscribe(on_change);

	// In a real implementation, we would call unsubscribe on cleanup
	// For now, the subscription lives for the lifetime of the component

	state
}

/// Subscribes to an external store with server snapshot support.
///
/// This variant is useful for SSR where you need different snapshots
/// for server and client environments.
///
/// # Type Parameters
///
/// * `T` - The type of value in the store
/// * `S` - The subscribe function type
/// * `G` - The get snapshot function type
/// * `GS` - The get server snapshot function type
///
/// # Arguments
///
/// * `subscribe` - A function that subscribes to the store and returns an unsubscribe function
/// * `get_snapshot` - A function that returns the current value from the store (client)
/// * `get_server_snapshot` - A function that returns the value during SSR
///
/// # Returns
///
/// A `Signal<T>` that stays in sync with the external store
pub fn use_sync_external_store_with_server<T, S, G, GS>(
	subscribe: S,
	get_snapshot: G,
	get_server_snapshot: GS,
) -> Signal<T>
where
	T: Clone + PartialEq + 'static,
	S: FnOnce(Rc<dyn Fn()>) -> Box<dyn FnOnce()> + 'static,
	G: Fn() -> T + 'static,
	GS: Fn() -> T + 'static,
{
	// Use server snapshot during SSR
	#[cfg(not(target_arch = "wasm32"))]
	{
		let _ = subscribe;
		let _ = get_snapshot;
		Signal::new(get_server_snapshot())
	}

	// Use client snapshot in browser
	#[cfg(target_arch = "wasm32")]
	{
		let _ = get_server_snapshot;
		use_sync_external_store(subscribe, get_snapshot)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::cell::RefCell;

	#[test]
	fn test_use_sync_external_store_basic() {
		let store_value = Rc::new(RefCell::new(42));

		let signal = use_sync_external_store(
			|_on_change| {
				// No-op subscribe for test
				Box::new(|| {})
			},
			{
				let store_value = Rc::clone(&store_value);
				move || *store_value.borrow()
			},
		);

		assert_eq!(signal.get(), 42);
	}

	#[test]
	fn test_use_sync_external_store_with_update() {
		let store_value = Rc::new(RefCell::new(0));
		let on_change_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

		let signal = use_sync_external_store(
			{
				let on_change_fn = Rc::clone(&on_change_fn);
				move |on_change| {
					*on_change_fn.borrow_mut() = Some(on_change);
					Box::new(|| {})
				}
			},
			{
				let store_value = Rc::clone(&store_value);
				move || *store_value.borrow()
			},
		);

		assert_eq!(signal.get(), 0);

		// Simulate store update
		*store_value.borrow_mut() = 100;

		// Trigger the on_change callback
		if let Some(on_change) = on_change_fn.borrow().as_ref() {
			on_change();
		}

		assert_eq!(signal.get(), 100);
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[test]
	fn test_use_sync_external_store_with_server() {
		let signal = use_sync_external_store_with_server(
			|_| Box::new(|| {}),
			|| 42, // Client snapshot
			|| 0,  // Server snapshot
		);

		// Should use server snapshot in non-WASM environment
		assert_eq!(signal.get(), 0);
	}
}
