//! Keyed async query cache hooks.
//!
//! `use_query` adds an app-wide cache layer for async reads while preserving
//! the existing `ResourceState` loading/success/error model. Query keys are
//! typed by their result and error payloads, and `#[server_fn]` generates
//! key helpers that include the server function identity plus serialized
//! arguments.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;
#[cfg(not(wasm))]
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::Signal;
use super::hooks::async_action::{Action, use_action};
use super::resource::ResourceState;

type QueryFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + 'static>>;
type QueryFetcher<T, E> = dyn Fn() -> QueryFuture<T, E> + 'static;

const DEFAULT_STALE_TIME: Duration = Duration::from_secs(30);
const DEFAULT_GC_TIME: Duration = Duration::from_secs(5 * 60);

thread_local! {
	static QUERY_CACHE: RefCell<HashMap<String, CachedQueryEntry>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
struct CachedQueryEntry {
	typed: Rc<dyn Any>,
	refetch: Rc<dyn Fn()>,
}

/// Typed cache key and fetcher for a query.
///
/// Values are normally produced by the `#[server_fn]` generated `key(...)`
/// helper. Manual keys are also supported for non-server-function fetchers.
pub struct QueryKey<T, E> {
	id: String,
	fetcher: Rc<QueryFetcher<T, E>>,
	stale_time: Duration,
	gc_time: Duration,
	_type: PhantomData<fn() -> Result<T, E>>,
}

impl<T, E> Clone for QueryKey<T, E> {
	fn clone(&self) -> Self {
		Self {
			id: self.id.clone(),
			fetcher: Rc::clone(&self.fetcher),
			stale_time: self.stale_time,
			gc_time: self.gc_time,
			_type: PhantomData,
		}
	}
}

impl<T, E> QueryKey<T, E> {
	/// Creates a typed query key from an explicit cache ID and fetcher.
	pub fn new<Id, F, Fut>(id: Id, fetcher: F) -> Self
	where
		Id: Into<String>,
		F: Fn() -> Fut + 'static,
		Fut: Future<Output = Result<T, E>> + 'static,
	{
		Self {
			id: id.into(),
			fetcher: Rc::new(move || Box::pin(fetcher())),
			stale_time: DEFAULT_STALE_TIME,
			gc_time: DEFAULT_GC_TIME,
			_type: PhantomData,
		}
	}

	/// Creates a typed key for a generated `#[server_fn]` marker.
	pub fn from_server_fn<M, Args, F, Fut>(args: Args, fetcher: F) -> Self
	where
		M: crate::server_fn::ServerFnMetadata,
		Args: Serialize,
		F: Fn() -> Fut + 'static,
		Fut: Future<Output = Result<T, E>> + 'static,
	{
		let encoded_args = serde_json::to_string(&args)
			.expect("server function query arguments must serialize into a cache key");
		Self::new(
			format!("server_fn:{}:{}:{}", M::PATH, M::CODEC, encoded_args),
			fetcher,
		)
	}

	/// Returns the stable cache ID for this key.
	pub fn id(&self) -> &str {
		&self.id
	}

	/// Configures how long a successful value is considered fresh.
	pub fn with_stale_time(mut self, stale_time: Duration) -> Self {
		self.stale_time = stale_time;
		self
	}

	/// Configures the requested cache retention window after the last observer.
	///
	/// The current implementation stores this value for cache policy parity and
	/// future eviction; entries are retained for the app lifetime unless the
	/// cache is explicitly cleared.
	pub fn with_gc_time(mut self, gc_time: Duration) -> Self {
		self.gc_time = gc_time;
		self
	}
}

struct QueryEntry<T: Clone + 'static, E: Clone + 'static> {
	id: String,
	state: Signal<ResourceState<T, E>>,
	is_fetching: Signal<bool>,
	fetcher: RefCell<Rc<QueryFetcher<T, E>>>,
	in_flight: Cell<bool>,
	last_fetched_ms: Cell<Option<u64>>,
	stale_time: Cell<Duration>,
	gc_time: Cell<Duration>,
}

impl<T: Clone + 'static, E: Clone + 'static> QueryEntry<T, E> {
	fn new(key: QueryKey<T, E>) -> Self
	where
		T: Serialize + DeserializeOwned,
		E: Serialize + DeserializeOwned,
	{
		let initial_state = hydrated_query_state(&key.id).unwrap_or(ResourceState::Loading);
		let last_fetched_ms = if matches!(initial_state, ResourceState::Success(_)) {
			Some(now_ms())
		} else {
			None
		};
		let id = key.id;

		Self {
			id,
			state: Signal::new(initial_state),
			is_fetching: Signal::new(false),
			fetcher: RefCell::new(key.fetcher),
			in_flight: Cell::new(false),
			last_fetched_ms: Cell::new(last_fetched_ms),
			stale_time: Cell::new(key.stale_time),
			gc_time: Cell::new(key.gc_time),
		}
	}

	fn update_policy(&self, key: QueryKey<T, E>) {
		*self.fetcher.borrow_mut() = key.fetcher;
		self.stale_time.set(key.stale_time);
		self.gc_time.set(key.gc_time);
	}

	fn is_stale(&self) -> bool {
		let Some(last_fetched_ms) = self.last_fetched_ms.get() else {
			return true;
		};
		now_ms().saturating_sub(last_fetched_ms) >= duration_ms(self.stale_time.get())
	}

	fn should_fetch_on_mount(&self) -> bool {
		self.state.with_untracked(|state| {
			matches!(state, ResourceState::Loading | ResourceState::Error(_)) || self.is_stale()
		})
	}

	fn start_fetch(self: &Rc<Self>, force: bool) {
		if self.in_flight.replace(true) {
			return;
		}

		let had_success = self
			.state
			.with_untracked(|state| matches!(state, ResourceState::Success(_)));
		if !force && had_success && !self.is_stale() {
			self.in_flight.set(false);
			return;
		}
		self.is_fetching.set(true);
		if !had_success {
			self.state.set(ResourceState::Loading);
		}

		let entry = Rc::clone(self);
		let fetcher = self.fetcher.borrow().clone();
		spawn_query_task(async move {
			let result = fetcher().await;
			match result {
				Ok(value) => {
					entry.last_fetched_ms.set(Some(now_ms()));
					entry.state.set(ResourceState::Success(value));
				}
				Err(error) => {
					entry.state.set(ResourceState::Error(error));
				}
			}
			entry.is_fetching.set(false);
			entry.in_flight.set(false);
		});
	}
}

/// Current phase of a query.
#[derive(Clone, Debug, PartialEq)]
pub enum QueryPhase<T, E> {
	/// The query has no successful value yet, or is currently fetching.
	Pending,
	/// The query has loaded successfully.
	Success(T),
	/// The latest fetch failed.
	Error(E),
}

impl<T, E> QueryPhase<T, E> {
	/// Returns `true` if the query is pending.
	pub fn is_pending(&self) -> bool {
		matches!(self, Self::Pending)
	}

	/// Returns `true` if the query is successful.
	pub fn is_success(&self) -> bool {
		matches!(self, Self::Success(_))
	}

	/// Returns `true` if the query is in an error state.
	pub fn is_error(&self) -> bool {
		matches!(self, Self::Error(_))
	}

	/// Returns the success value if available.
	pub fn result(&self) -> Option<&T> {
		match self {
			Self::Success(value) => Some(value),
			_ => None,
		}
	}

	/// Returns the error value if available.
	pub fn error(&self) -> Option<&E> {
		match self {
			Self::Error(error) => Some(error),
			_ => None,
		}
	}
}

/// Reactive handle returned by [`use_query`].
pub struct QueryHandle<T: Clone + 'static, E: Clone + 'static> {
	entry: Rc<QueryEntry<T, E>>,
	guards: Rc<RefCell<Vec<QueryGuard>>>,
}

impl<T: Clone + 'static, E: Clone + 'static> Clone for QueryHandle<T, E> {
	fn clone(&self) -> Self {
		Self {
			entry: Rc::clone(&self.entry),
			guards: Rc::clone(&self.guards),
		}
	}
}

impl<T: Clone + 'static, E: Clone + 'static> QueryHandle<T, E> {
	fn mark_ssr_read(&self) {
		#[cfg(native)]
		crate::ssr::resource_context::mark_resource_read(&self.entry.id);
	}

	/// Returns the underlying resource-style state.
	pub fn get(&self) -> ResourceState<T, E> {
		self.mark_ssr_read();
		self.entry.state.get()
	}

	/// Returns the current query phase.
	pub fn phase(&self) -> QueryPhase<T, E> {
		self.mark_ssr_read();
		match self.entry.state.get() {
			ResourceState::Loading => QueryPhase::Pending,
			ResourceState::Success(value) => QueryPhase::Success(value),
			ResourceState::Error(error) => QueryPhase::Error(error),
		}
	}

	/// Returns `true` while a fetch is in progress.
	pub fn is_fetching(&self) -> bool {
		self.mark_ssr_read();
		self.entry.is_fetching.get()
	}

	/// Returns `true` until the query has a successful value or error.
	pub fn is_pending(&self) -> bool {
		self.phase().is_pending() || self.is_fetching()
	}

	/// Returns `true` if the query has a successful value.
	pub fn is_success(&self) -> bool {
		self.phase().is_success()
	}

	/// Returns `true` if the query is in an error state.
	pub fn is_error(&self) -> bool {
		self.phase().is_error()
	}

	/// Returns the current successful value, if present.
	pub fn data(&self) -> Option<T> {
		self.mark_ssr_read();
		match self.entry.state.get() {
			ResourceState::Success(value) => Some(value),
			_ => None,
		}
	}

	/// Returns the current error value, if present.
	pub fn error(&self) -> Option<E> {
		self.mark_ssr_read();
		match self.entry.state.get() {
			ResourceState::Error(error) => Some(error),
			_ => None,
		}
	}

	/// Manually refetches this query.
	pub fn refetch(&self) {
		self.entry.start_fetch(true);
	}

	/// Refetches this query at a fixed interval while the handle is alive.
	pub fn poll(self, interval: Duration) -> Self {
		if !interval.is_zero() {
			self.guards
				.borrow_mut()
				.push(QueryGuard::poll(interval, Rc::clone(&self.entry)));
		}
		self
	}

	/// Updates the stale-time policy for this mounted query.
	pub fn stale_time(self, stale_time: Duration) -> Self {
		self.entry.stale_time.set(stale_time);
		if self.entry.is_stale() {
			self.entry.start_fetch(false);
		}
		self
	}

	/// Updates the cache retention policy for this mounted query.
	pub fn gc_time(self, gc_time: Duration) -> Self {
		self.entry.gc_time.set(gc_time);
		self
	}

	/// Returns the current stale-time policy.
	pub fn stale_time_policy(&self) -> Duration {
		self.entry.stale_time.get()
	}

	/// Returns the current cache retention policy.
	pub fn gc_time_policy(&self) -> Duration {
		self.entry.gc_time.get()
	}
}

/// Creates or subscribes to an app-wide keyed query.
pub fn use_query<T, E>(key: QueryKey<T, E>) -> QueryHandle<T, E>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	#[cfg(native)]
	if let Some(query) = try_create_ssr_query(key.clone()) {
		return query;
	}

	let entry = query_entry(key);
	if entry.should_fetch_on_mount() {
		entry.start_fetch(false);
	}

	QueryHandle {
		entry,
		guards: Rc::new(RefCell::new(Vec::new())),
	}
}

/// Creates a mutation action that can invalidate queries on success.
pub fn use_mutation<P, T, E, F, Fut>(mutation_fn: F) -> Action<T, E>
where
	P: 'static,
	T: Clone + 'static,
	E: Clone + 'static,
	F: Fn(P) -> Fut + 'static,
	Fut: Future<Output = Result<T, E>> + 'static,
{
	use_action(mutation_fn)
}

impl<T, E> Action<T, E>
where
	T: Clone + 'static,
	E: Clone + 'static,
{
	/// Refetches `key` after this mutation succeeds.
	pub fn invalidates<QT, QE>(self, key: QueryKey<QT, QE>) -> Self
	where
		QT: Clone + 'static,
		QE: Clone + 'static,
	{
		let id = key.id().to_string();
		self.on_success(move |_| {
			invalidate_query_id(&id);
		})
	}
}

fn query_entry<T, E>(key: QueryKey<T, E>) -> Rc<QueryEntry<T, E>>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	let id = key.id.clone();
	QUERY_CACHE.with(|cache| {
		let mut cache = cache.borrow_mut();
		if let Some(cached) = cache.get(&id) {
			let entry = Rc::clone(&cached.typed)
				.downcast::<QueryEntry<T, E>>()
				.unwrap_or_else(|_| {
					panic!("query cache key `{id}` was reused with incompatible types")
				});
			entry.update_policy(key);
			return entry;
		}

		let entry = Rc::new(QueryEntry::new(key));
		cache.insert(
			id,
			CachedQueryEntry {
				typed: entry.clone(),
				refetch: Rc::new({
					let entry = Rc::clone(&entry);
					move || entry.start_fetch(true)
				}),
			},
		);
		entry
	})
}

#[cfg(native)]
fn try_create_ssr_query<T, E>(key: QueryKey<T, E>) -> Option<QueryHandle<T, E>>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	crate::ssr::resource_context::with_active_context(|context| {
		let entry = Rc::new(QueryEntry::new(key));
		let fetcher = entry.fetcher.borrow().clone();
		context.borrow_mut().register_resource(
			entry.id.clone(),
			move || fetcher(),
			entry.state.clone(),
		);
		QueryHandle {
			entry,
			guards: Rc::new(RefCell::new(Vec::new())),
		}
	})
}

fn invalidate_query_id(id: &str) {
	QUERY_CACHE.with(|cache| {
		if let Some(cached) = cache.borrow().get(id) {
			(cached.refetch)();
		}
	});
}

#[cfg(wasm)]
fn hydrated_query_state<T, E>(key: &str) -> Option<ResourceState<T, E>>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	let context = crate::hydration::HydrationContext::from_window().ok()?;
	let value = context.get_resource_state(key)?;
	serde_json::from_value(value.clone()).ok()
}

#[cfg(not(wasm))]
fn hydrated_query_state<T, E>(_key: &str) -> Option<ResourceState<T, E>>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	None
}

#[cfg(test)]
fn spawn_query_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	tokio_test::block_on(fut);
}

#[cfg(not(test))]
fn spawn_query_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	crate::platform::spawn_task(fut);
}

#[cfg(wasm)]
struct QueryGuard {
	interval_id: i32,
	_closure: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

#[cfg(wasm)]
impl QueryGuard {
	fn poll<T, E>(interval: Duration, entry: Rc<QueryEntry<T, E>>) -> Self
	where
		T: Clone + 'static,
		E: Clone + 'static,
	{
		use wasm_bindgen::JsCast;

		let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
			entry.start_fetch(true);
		}) as Box<dyn FnMut()>);

		let interval_ms = duration_ms(interval).min(i32::MAX as u64) as i32;
		let interval_id = web_sys::window()
			.and_then(|window| {
				window
					.set_interval_with_callback_and_timeout_and_arguments_0(
						closure.as_ref().unchecked_ref(),
						interval_ms,
					)
					.ok()
			})
			.unwrap_or(-1);

		Self {
			interval_id,
			_closure: closure,
		}
	}
}

#[cfg(wasm)]
impl Drop for QueryGuard {
	fn drop(&mut self) {
		if self.interval_id >= 0
			&& let Some(window) = web_sys::window()
		{
			window.clear_interval_with_handle(self.interval_id);
		}
	}
}

#[cfg(not(wasm))]
struct QueryGuard;

#[cfg(not(wasm))]
impl QueryGuard {
	fn poll<T, E>(_interval: Duration, _entry: Rc<QueryEntry<T, E>>) -> Self
	where
		T: Clone + 'static,
		E: Clone + 'static,
	{
		Self
	}
}

fn duration_ms(duration: Duration) -> u64 {
	duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(not(wasm))]
fn now_ms() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(duration_ms)
		.unwrap_or_default()
}

#[cfg(wasm)]
fn now_ms() -> u64 {
	js_sys::Date::now() as u64
}

#[cfg(test)]
pub(crate) fn clear_query_cache_for_test() {
	QUERY_CACHE.with(|cache| cache.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
	use std::cell::Cell;

	use rstest::rstest;
	use serial_test::serial;

	use super::*;

	#[rstest]
	#[serial(query_cache)]
	fn use_query_deduplicates_shared_key() {
		// Arrange
		clear_query_cache_for_test();
		let calls = Rc::new(Cell::new(0));

		// Act
		let first = use_query(QueryKey::new("shared", {
			let calls = Rc::clone(&calls);
			move || {
				calls.set(calls.get() + 1);
				async { Ok::<_, String>("value".to_string()) }
			}
		}));
		let second = use_query(QueryKey::new("shared", {
			let calls = Rc::clone(&calls);
			move || {
				calls.set(calls.get() + 1);
				async { Ok::<_, String>("value".to_string()) }
			}
		}));

		// Assert
		assert_eq!(calls.get(), 1);
		assert_eq!(first.data(), Some("value".to_string()));
		assert_eq!(second.data(), Some("value".to_string()));
	}

	#[rstest]
	#[serial(query_cache)]
	fn refetch_runs_fetcher_again() {
		// Arrange
		clear_query_cache_for_test();
		let calls = Rc::new(Cell::new(0));
		let query = use_query(QueryKey::new("manual-refetch", {
			let calls = Rc::clone(&calls);
			move || {
				let value = calls.get() + 1;
				calls.set(value);
				async move { Ok::<_, String>(value) }
			}
		}));

		// Act
		query.refetch();

		// Assert
		assert_eq!(calls.get(), 2);
		assert_eq!(query.data(), Some(2));
	}

	#[rstest]
	#[serial(query_cache)]
	fn mutation_success_invalidates_registered_query() {
		// Arrange
		clear_query_cache_for_test();
		let calls = Rc::new(Cell::new(0));
		let key = QueryKey::new("invalidated", {
			let calls = Rc::clone(&calls);
			move || {
				let value = calls.get() + 1;
				calls.set(value);
				async move { Ok::<_, String>(value) }
			}
		});
		let query = use_query(key.clone());
		let mutation =
			use_mutation(|_: ()| async { Ok::<_, String>("done".to_string()) }).invalidates(key);

		// Act
		mutation.force_success_for_test("done".to_string());

		// Assert
		assert_eq!(calls.get(), 2);
		assert_eq!(query.data(), Some(2));
	}

	#[rstest]
	#[serial(query_cache)]
	fn server_fn_key_encodes_identity_and_args() {
		// Arrange
		clear_query_cache_for_test();

		struct Marker;

		impl crate::server_fn::ServerFnMetadata for Marker {
			const PATH: &'static str = "/api/server_fn/list_jobs";
			const NAME: &'static str = "list_jobs";
			const CODEC: &'static str = "json";
			const IS_JSON_CODEC: bool = true;
		}

		// Act
		let key: QueryKey<Vec<i64>, crate::server_fn::ServerFnError> =
			QueryKey::from_server_fn::<Marker, _, _, _>((42_i64,), || async { Ok(vec![42]) });

		// Assert
		assert_eq!(key.id(), r#"server_fn:/api/server_fn/list_jobs:json:[42]"#);
	}
}
