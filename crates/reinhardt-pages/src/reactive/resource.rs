//! Resource - Async data fetching primitive
//!
//! This module provides `Resource<T>`, a reactive primitive for handling async operations.
//! It manages Loading/Success/Error states and integrates with the Signal-based
//! reactivity system.

use super::{Effect, Signal};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::platform::{defer_yield, spawn_task};
use reinhardt_core::reactive::deps::IntoDeps;

/// Type alias for the refetch callback function
///
/// This reduces type complexity for the `Resource` struct's `refetch_fn` field.
type RefetchCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

/// State of a Resource
///
/// A Resource can be in one of three states:
/// - `Loading`: Initial state or during refetch
/// - `Success(T)`: Successfully fetched data
/// - `Error(E)`: Failed to fetch with error
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResourceState<T, E> {
	/// Resource is currently loading
	Loading,
	/// Resource loaded successfully with data
	Success(T),
	/// Resource failed to load with error
	Error(E),
}

impl<T, E> ResourceState<T, E> {
	/// Returns `true` if the state is `Loading`
	pub fn is_loading(&self) -> bool {
		matches!(self, ResourceState::Loading)
	}

	/// Returns `true` if the state is `Success`
	pub fn is_success(&self) -> bool {
		matches!(self, ResourceState::Success(_))
	}

	/// Returns `true` if the state is `Error`
	pub fn is_error(&self) -> bool {
		matches!(self, ResourceState::Error(_))
	}

	/// Returns the success value if available
	pub fn as_ref(&self) -> Option<&T> {
		match self {
			ResourceState::Success(data) => Some(data),
			_ => None,
		}
	}

	/// Returns the error value if available
	pub fn error(&self) -> Option<&E> {
		match self {
			ResourceState::Error(err) => Some(err),
			_ => None,
		}
	}
}

impl<T: fmt::Display, E: fmt::Display> fmt::Display for ResourceState<T, E> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ResourceState::Loading => write!(f, "Loading..."),
			ResourceState::Success(data) => write!(f, "{}", data),
			ResourceState::Error(err) => write!(f, "Error: {}", err),
		}
	}
}

/// Resource - Reactive async data container
///
/// A Resource manages async data fetching with automatic state management.
/// It tracks Loading/Success/Error states and integrates with the Signal system
/// for fine-grained reactivity.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::{Resource, use_resource};
///
/// async fn fetch_user(id: u32) -> Result<User, String> {
///     // Fetch from API...
/// }
///
/// let resource = use_resource(|| fetch_user(42), ());
///
/// match resource.get() {
///     ResourceState::Loading => println!("Loading..."),
///     ResourceState::Success(user) => println!("User: {:?}", user),
///     ResourceState::Error(err) => println!("Error: {}", err),
/// }
/// ```
pub struct Resource<T: Clone + 'static, E: Clone + 'static = String> {
	state: Signal<ResourceState<T, E>>,
	refetch_fn: RefetchCallback,
	/// RAII anchor that keeps the dependency-tracking `Effect` alive for the
	/// lifetime of the `Resource`. An `Effect` disposes itself on drop (removing
	/// its node from the runtime graph), so without holding the handle here the
	/// effect would be torn down immediately after creation and dependency-change
	/// refetch would never fire.
	effect_guard: Rc<Effect>,
}

impl<T: Clone + 'static, E: Clone + 'static> Clone for Resource<T, E> {
	fn clone(&self) -> Self {
		Resource {
			state: self.state.clone(),
			refetch_fn: Rc::clone(&self.refetch_fn),
			effect_guard: Rc::clone(&self.effect_guard),
		}
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Resource<T, E> {
	/// Get the current state of the resource
	///
	/// This method tracks the access for reactivity - any Effect or Memo
	/// that calls this will automatically re-run when the state changes.
	pub fn get(&self) -> ResourceState<T, E> {
		self.state.get()
	}

	/// Update the resource state
	///
	/// This is typically used internally by the fetcher function.
	pub fn set(&self, new_state: ResourceState<T, E>) {
		self.state.set(new_state);
	}

	/// Trigger a refetch of the resource
	///
	/// This sets the state to Loading and re-executes the fetcher function.
	pub fn refetch(&self) {
		if let Some(ref refetch) = *self.refetch_fn.borrow() {
			refetch();
		}
	}

	/// Returns `true` if the resource is currently loading
	pub fn is_loading(&self) -> bool {
		self.state.with_untracked(|s| s.is_loading())
	}

	/// Returns `true` if the resource has successfully loaded
	pub fn is_success(&self) -> bool {
		self.state.with_untracked(|s| s.is_success())
	}

	/// Returns `true` if the resource failed to load
	pub fn is_error(&self) -> bool {
		self.state.with_untracked(|s| s.is_error())
	}
}

impl<T: Clone + 'static, E: Clone + 'static> reinhardt_core::reactive::deps::Trackable
	for Resource<T, E>
{
	/// Returns the underlying state `Signal`'s `NodeId`, allowing this
	/// `Resource` to participate in hook deps tuples alongside `Signal`
	/// and `Memo` (Refs #4195).
	fn node_id(&self) -> reinhardt_core::reactive::runtime::NodeId {
		self.state.id()
	}
}

/// Reactive async data hook — the resource counterpart of `use_effect`.
///
/// `use_resource(fetcher, deps)` runs `fetcher` and tracks its result as a
/// [`Resource`] (`Loading → Success/Error`). The `deps` argument follows the
/// same [`IntoDeps`] convention as [`use_effect`](super::hooks::use_effect):
///
/// - `()` → fetch once on mount (never automatically refetches).
/// - `(signal,)` / `(a, b, ..)` → refetch whenever any listed dependency
///   changes. Dependencies are the explicitly listed [`Trackable`]s
///   (`Signal`/`Memo`/`Resource`); signals merely *read* inside the async
///   `fetcher` do not subscribe (they cross an `await` boundary), so list
///   everything that should drive a refetch — the same stale-deps rule as
///   `use_effect`.
///
/// The initial fetch and every dependency-driven refetch are deferred one
/// microtask (`defer_yield`) so they cannot hang when created during WASM
/// initialization before the event loop is running (#3316).
///
/// [`Trackable`]: reinhardt_core::reactive::deps::Trackable
/// [`IntoDeps`]: reinhardt_core::reactive::deps::IntoDeps
///
/// # Dual-target behavior
///
/// Like [`use_action`](super::hooks::use_action), this hook is available on all
/// targets:
///
/// - **WASM**: the fetcher runs via `spawn_task` on the browser event loop.
/// - **Non-WASM (SSR)**: `spawn_task` drops the future, so the fetcher never
///   runs and the `Resource` stays `Loading`. The server renders the loading
///   state and the client performs the real fetch after hydration.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::{Signal, use_resource};
///
/// // Fetch once on mount:
/// let user = use_resource(|| async { fetch_user_from_api(42).await }, ());
///
/// // Refetch whenever `user_id` changes:
/// let user_id = Signal::new(42u32);
/// let user = use_resource(
///     {
///         let user_id = user_id.clone();
///         move || {
///             let id = user_id.get();
///             async move { fetch_user_from_api(id).await }
///         }
///     },
///     (user_id.clone(),),
/// );
/// user_id.set(100); // triggers a refetch
/// ```
pub fn use_resource<T, E, F, Fut, D>(fetcher: F, deps: D) -> Resource<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
	F: Fn() -> Fut + 'static,
	Fut: std::future::Future<Output = Result<T, E>> + 'static,
	D: IntoDeps,
{
	let state = Signal::new(ResourceState::Loading);
	let fetcher = Rc::new(fetcher);

	// Single fetch routine shared by the dependency-driven Effect and manual
	// refetch. `defer_yield` runs on every path (initial, dependency change, and
	// manual refetch) so the fetch cannot hang when spawned during WASM
	// initialization before the event loop ticks (#3316).
	let run: Rc<dyn Fn()> = {
		let state = state.clone();
		let fetcher = Rc::clone(&fetcher);
		Rc::new(move || {
			state.set(ResourceState::Loading);
			let state = state.clone();
			let fetcher = Rc::clone(&fetcher);
			spawn_task(async move {
				defer_yield().await;
				match fetcher().await {
					Ok(data) => state.set(ResourceState::Success(data)),
					Err(err) => state.set(ResourceState::Error(err)),
				}
			});
		})
	};

	// Drive the initial fetch and dependency-change refetches. `new_with_deps`
	// (not `new`) means only the explicitly listed `deps` trigger re-runs; the
	// Effect is stored in the returned `Resource` (see `effect_guard`) so it
	// stays alive for the Resource's lifetime instead of being disposed on drop.
	let effect = {
		let run = Rc::clone(&run);
		Effect::new_with_deps(
			move || {
				run();
				None::<fn()>
			},
			deps.into_deps(),
		)
	};

	let refetch_fn: RefetchCallback = Rc::new(RefCell::new(Some(Box::new({
		let run = Rc::clone(&run);
		move || run()
	}))));

	Resource {
		state,
		refetch_fn,
		effect_guard: Rc::new(effect),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_resource_state_constructors() {
		let loading: ResourceState<String, String> = ResourceState::Loading;
		assert!(loading.is_loading());
		assert!(!loading.is_success());
		assert!(!loading.is_error());

		let success: ResourceState<String, String> = ResourceState::Success("data".to_string());
		assert!(!success.is_loading());
		assert!(success.is_success());
		assert!(!success.is_error());
		assert_eq!(success.as_ref(), Some(&"data".to_string()));

		let error: ResourceState<String, String> = ResourceState::Error("failed".to_string());
		assert!(!error.is_loading());
		assert!(!error.is_success());
		assert!(error.is_error());
		assert_eq!(error.error(), Some(&"failed".to_string()));
	}

	#[test]
	fn test_resource_state_display() {
		let loading: ResourceState<String, String> = ResourceState::Loading;
		assert_eq!(format!("{}", loading), "Loading...");

		let success: ResourceState<String, String> = ResourceState::Success("Hello".to_string());
		assert_eq!(format!("{}", success), "Hello");

		let error: ResourceState<String, String> =
			ResourceState::Error("Connection failed".to_string());
		assert_eq!(format!("{}", error), "Error: Connection failed");
	}
}
