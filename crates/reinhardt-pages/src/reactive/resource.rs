//! Resource - Async data fetching primitive
//!
//! This module provides `Resource<T>`, a reactive primitive for handling async operations.
//! It manages Loading/Success/Error states and integrates with the Signal-based
//! reactivity system.

use super::{Effect, Signal};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

use crate::platform::{defer_yield, spawn_task};
use reinhardt_core::reactive::deps::IntoDeps;

/// Type alias for the refetch callback function
///
/// This reduces type complexity for the `Resource` struct's `refetch_fn` field.
type RefetchCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

#[cfg(wasm)]
thread_local! {
	static CLIENT_RESOURCE_COUNTER: Cell<usize> = const { Cell::new(0) };
}

/// Returns the current client call-order resource ID offset.
#[cfg(wasm)]
pub(crate) fn current_client_resource_counter() -> usize {
	CLIENT_RESOURCE_COUNTER.with(Cell::get)
}

/// Restores the client call-order resource ID offset.
#[cfg(wasm)]
pub(crate) fn set_client_resource_counter(value: usize) {
	CLIENT_RESOURCE_COUNTER.with(|counter| counter.set(value));
}

#[cfg(wasm)]
fn next_client_resource_key() -> String {
	CLIENT_RESOURCE_COUNTER.with(|counter| {
		let id = counter.get();
		counter.set(id + 1);
		format!("rh-res-{id}")
	})
}

#[cfg(wasm)]
fn reserve_client_resource_key(key: &str) {
	if let Some(id) = key.strip_prefix("rh-res-")
		&& let Ok(index) = id.parse::<usize>()
	{
		CLIENT_RESOURCE_COUNTER
			.with(|counter| counter.set(counter.get().max(index.saturating_add(1))));
	}
}

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
	ssr_key: Option<String>,
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
			ssr_key: self.ssr_key.clone(),
			effect_guard: Rc::clone(&self.effect_guard),
		}
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Resource<T, E> {
	fn mark_ssr_read(&self) {
		#[cfg(native)]
		if let Some(key) = &self.ssr_key {
			crate::ssr::resource_context::mark_resource_read(key);
		}
	}

	/// Get the current state of the resource
	///
	/// This method tracks the access for reactivity - any Effect or Memo
	/// that calls this will automatically re-run when the state changes.
	/// During SSR, a read outside an active Suspense boundary also keeps
	/// the resource in the external resolution set for the shell render.
	pub fn get(&self) -> ResourceState<T, E> {
		self.mark_ssr_read();
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
		self.mark_ssr_read();
		self.state.with_untracked(|s| s.is_loading())
	}

	/// Returns `true` if the resource has successfully loaded
	pub fn is_success(&self) -> bool {
		self.mark_ssr_read();
		self.state.with_untracked(|s| s.is_success())
	}

	/// Returns `true` if the resource failed to load
	pub fn is_error(&self) -> bool {
		self.mark_ssr_read();
		self.state.with_untracked(|s| s.is_error())
	}

	/// Returns this resource's deterministic SSR hydration key, if known.
	pub fn ssr_key(&self) -> Option<&str> {
		self.ssr_key.as_deref()
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
/// - **Non-WASM (SSR)**: when called under [`SsrRenderer`](crate::ssr::SsrRenderer),
///   the fetcher is registered in the request-scoped SSR resource context. The
///   renderer awaits it up to [`SsrOptions::resource_timeout`](crate::ssr::SsrOptions::resource_timeout),
///   serializes resolved `Success`/`Error` state into the hydration payload,
///   and replays the render so the client can adopt the server value instead of
///   refetching on mount. Outside an SSR context, native `spawn_task` remains a
///   no-op and the resource stays `Loading`.
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
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
	F: Fn() -> Fut + 'static,
	Fut: std::future::Future<Output = Result<T, E>> + 'static,
	D: IntoDeps,
{
	use_resource_with_optional_key(None, fetcher, deps)
}

/// Reactive async data hook with an explicit SSR hydration key.
///
/// Prefer this when call-order keys would be unstable across server and client
/// renders, such as conditionally rendered resource hooks.
pub fn use_resource_with_key<K, T, E, F, Fut, D>(key: K, fetcher: F, deps: D) -> Resource<T, E>
where
	K: Into<String>,
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
	F: Fn() -> Fut + 'static,
	Fut: std::future::Future<Output = Result<T, E>> + 'static,
	D: IntoDeps,
{
	use_resource_with_optional_key(Some(key.into()), fetcher, deps)
}

fn use_resource_with_optional_key<T, E, F, Fut, D>(
	key: Option<String>,
	fetcher: F,
	deps: D,
) -> Resource<T, E>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
	F: Fn() -> Fut + 'static,
	Fut: std::future::Future<Output = Result<T, E>> + 'static,
	D: IntoDeps,
{
	let fetcher = Rc::new(fetcher);

	#[cfg(native)]
	if let Some(resource) = try_create_ssr_resource(key.clone(), Rc::clone(&fetcher)) {
		return resource;
	}

	create_client_resource(key, fetcher, deps)
}

fn create_client_resource<T, E, F, Fut, D>(
	resource_key: Option<String>,
	fetcher: Rc<F>,
	deps: D,
) -> Resource<T, E>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
	F: Fn() -> Fut + 'static,
	Fut: std::future::Future<Output = Result<T, E>> + 'static,
	D: IntoDeps,
{
	let ssr_key = resource_key.clone();

	#[cfg(wasm)]
	let initial_state = {
		let key = if let Some(key) = resource_key.clone() {
			reserve_client_resource_key(&key);
			key
		} else {
			next_client_resource_key()
		};
		hydrated_resource_state(&key)
	};

	#[cfg(native)]
	let initial_state = {
		let _ = resource_key;
		None
	};

	let run_initial_fetch = initial_state.is_none();
	let state = Signal::new(initial_state.unwrap_or(ResourceState::Loading));

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
	build_resource_from_run(state, run, deps, run_initial_fetch, ssr_key)
}

fn build_resource_from_run<T, E, D>(
	state: Signal<ResourceState<T, E>>,
	run: Rc<dyn Fn()>,
	deps: D,
	run_initial_fetch: bool,
	ssr_key: Option<String>,
) -> Resource<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
	D: IntoDeps,
{
	let first_run = Rc::new(Cell::new(true));
	let effect = {
		let run = Rc::clone(&run);
		let first_run = Rc::clone(&first_run);
		Effect::new_with_deps(
			move || {
				let is_first = first_run.replace(false);
				if run_initial_fetch || !is_first {
					run();
				}
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
		ssr_key,
		effect_guard: Rc::new(effect),
	}
}

#[cfg(native)]
fn try_create_ssr_resource<T, E, F, Fut>(
	explicit_key: Option<String>,
	fetcher: Rc<F>,
) -> Option<Resource<T, E>>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
	F: Fn() -> Fut + 'static,
	Fut: std::future::Future<Output = Result<T, E>> + 'static,
{
	crate::ssr::resource_context::with_active_context(|context| {
		let mut context = context.borrow_mut();
		let key = if let Some(key) = explicit_key {
			context.reserve_call_order_key(&key);
			key
		} else {
			context.next_call_order_key()
		};
		if let Some(resolved_state) = context.resolved_resource_state::<T, E>(&key) {
			let state = Signal::new(resolved_state);
			let run: Rc<dyn Fn()> = Rc::new({
				let state = state.clone();
				move || state.set(ResourceState::Loading)
			});
			build_resource_from_run(state, run, (), false, Some(key))
		} else {
			let state = Signal::new(ResourceState::Loading);
			context.register_resource::<T, E, _, Fut>(
				key.clone(),
				{
					let fetcher = Rc::clone(&fetcher);
					move || fetcher()
				},
				state.clone(),
			);

			let run: Rc<dyn Fn()> = Rc::new({
				let state = state.clone();
				move || state.set(ResourceState::Loading)
			});

			build_resource_from_run(state, run, (), false, Some(key))
		}
	})
}

#[cfg(any(wasm, test))]
fn deserialize_resource_state<T, E>(value: &serde_json::Value) -> Option<ResourceState<T, E>>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	serde_json::from_value(value.clone()).ok()
}

#[cfg(wasm)]
fn hydrated_resource_state<T, E>(key: &str) -> Option<ResourceState<T, E>>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	let context = crate::hydration::HydrationContext::from_window().ok()?;
	let value = context.get_resource_state(key)?;
	deserialize_resource_state(value)
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

	#[test]
	fn test_deserialize_resource_state() {
		let value = serde_json::json!({"Success": "server"});
		let state: ResourceState<String, String> = deserialize_resource_state(&value).unwrap();
		assert_eq!(state, ResourceState::Success("server".to_string()));
	}
}
