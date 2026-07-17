//! Keyed async query cache hooks.
//!
//! `use_query` adds an app-wide cache layer for async reads while preserving
//! the existing `ResourceState` loading/success/error model. Query keys are
//! typed by their result and error payloads, and `#[server_fn]` generates
//! key helpers that include the server function identity plus an opaque digest
//! of canonical JSON arguments.
//!
//! Route loaders acquire these same keyed entries through an imperative RAII
//! lease. Prefetch, navigation, mounted-route state, and `use_query` therefore
//! share in-flight work without introducing a second route-data cache.

mod canonical_json;

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
#[cfg(not(wasm))]
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::future::AbortHandle;
use futures_util::future::Abortable;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use super::Signal;
use super::hooks::async_action::{Action, use_action};
use super::resource::ResourceState;
use crate::cancellation::{AbortableTaskGuard, CancellationSource, scope_cancellation};
use reinhardt_core::reactive::{ReactiveScope, ScopeId, scope::enter_scope};

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
	ssr_prefetch: bool,
	_type: PhantomData<fn() -> Result<T, E>>,
}

impl<T, E> Clone for QueryKey<T, E> {
	fn clone(&self) -> Self {
		Self {
			id: self.id.clone(),
			fetcher: Rc::clone(&self.fetcher),
			stale_time: self.stale_time,
			gc_time: self.gc_time,
			ssr_prefetch: self.ssr_prefetch,
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
			ssr_prefetch: true,
			_type: PhantomData,
		}
	}

	/// Creates a typed key for a generated `#[server_fn]` marker.
	///
	/// JSON object keys are sorted recursively so logically equivalent argument
	/// maps produce the same cache and hydration ID. The canonical argument
	/// payload is SHA-256 hashed before it becomes part of the ID.
	pub fn from_server_fn<M, Args, F, Fut>(args: Args, fetcher: F) -> Self
	where
		M: crate::server_fn::ServerFnMetadata,
		Args: Serialize,
		F: Fn() -> Fut + 'static,
		Fut: Future<Output = Result<T, E>> + 'static,
	{
		let encoded_args = canonical_json::encode(&args)
			.expect("server function query arguments must serialize into a cache key");
		let args_digest = Sha256::digest(encoded_args.as_bytes());
		Self::new(
			format!("server_fn:{}:{}:sha256:{args_digest:x}", M::PATH, M::CODEC),
			fetcher,
		)
	}

	/// Returns the stable cache ID for this key.
	pub fn id(&self) -> &str {
		&self.id
	}

	/// Configures how long a resolved value is considered fresh.
	///
	/// SSR-replayed success and error states are both treated as freshly fetched
	/// so the initial replay preserves the server-rendered state before a retry.
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

	/// Configures whether SSR may prefetch this query in the native resource context.
	pub fn with_ssr_prefetch(mut self, enabled: bool) -> Self {
		self.ssr_prefetch = enabled;
		self
	}
}

/// Identifies the runtime consumer holding a query lease.
// These consumer variants are part of the internal loader contract; later
// navigation and prefetch phases construct the variants that are not used by
// the ordinary `use_query` hook yet.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryConsumer {
	Prefetch,
	Navigation(u64),
	MountedRoute(u64),
	MountedQuery,
	Maintenance,
}

/// Controls whether a failed fetch remains a reusable cache error.
// The discard policy is exercised by route loaders added in later tasks.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryErrorPolicy {
	Retain,
	Discard,
}

/// Options for an imperative query acquisition.
pub(crate) struct QueryAcquireOptions {
	pub consumer: QueryConsumer,
	pub error_policy: QueryErrorPolicy,
}

struct QueryRequest<T, E> {
	generation: u64,
	source: CancellationSource,
	_guard: AbortableTaskGuard,
	_marker: PhantomData<fn() -> Result<T, E>>,
}

struct QueryEntry<T: Clone + 'static, E: Clone + 'static> {
	_scope: Rc<ReactiveScope>,
	id: String,
	state: Signal<ResourceState<T, E>>,
	is_fetching: Signal<bool>,
	fetcher: RefCell<Rc<QueryFetcher<T, E>>>,
	request: RefCell<Option<QueryRequest<T, E>>>,
	next_generation: Cell<u64>,
	completed: RefCell<Option<(u64, Result<T, E>)>>,
	waiters: RefCell<Vec<Waker>>,
	lease_count: Cell<usize>,
	retain_lease_count: Cell<usize>,
	refetch_after_in_flight: Cell<bool>,
	last_fetched_ms: Cell<Option<u64>>,
	stale_time: Cell<Duration>,
	gc_time: Cell<Duration>,
}

struct QueryLeaseInner<T: Clone + 'static, E: Clone + 'static> {
	entry: Rc<QueryEntry<T, E>>,
	generation: Cell<Option<u64>>,
	retains_errors: bool,
}

/// RAII interest in one keyed query entry.
pub(crate) struct QueryLease<T: Clone + 'static, E: Clone + 'static> {
	inner: Rc<QueryLeaseInner<T, E>>,
}

impl<T: Clone + 'static, E: Clone + 'static> Clone for QueryLease<T, E> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Drop for QueryLeaseInner<T, E> {
	fn drop(&mut self) {
		let entry = &self.entry;
		let remaining = entry.lease_count.get().saturating_sub(1);
		entry.lease_count.set(remaining);
		if self.retains_errors {
			let retained = entry.retain_lease_count.get().saturating_sub(1);
			entry.retain_lease_count.set(retained);
		}
		if remaining == 0 {
			entry.cancel_request();
		}
	}
}

/// Polls cached query work in the scope that owns the query entry.
///
/// Cache entries outlive the component that first requested them, so their
/// fetchers cannot rely on that component's render scope remaining active.
struct ScopedQueryFuture<Fut> {
	scope: ScopeId,
	future: Pin<Box<Fut>>,
}

impl<Fut> Future for ScopedQueryFuture<Fut>
where
	Fut: Future<Output = ()>,
{
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.get_mut();
		let poll = || this.future.as_mut().poll(cx);
		enter_scope(this.scope, poll).unwrap_or(Poll::Ready(()))
	}
}

fn initial_query_state<T, E>(
	hydrated_state: Option<ResourceState<T, E>>,
) -> (ResourceState<T, E>, Option<u64>) {
	let initial_state = hydrated_state.unwrap_or(ResourceState::Loading);
	let last_fetched_ms = if matches!(
		&initial_state,
		ResourceState::Success(_) | ResourceState::Error(_)
	) {
		Some(now_ms())
	} else {
		None
	};
	(initial_state, last_fetched_ms)
}

impl<T: Clone + 'static, E: Clone + 'static> QueryEntry<T, E> {
	fn new(key: QueryKey<T, E>) -> Self
	where
		T: Serialize + DeserializeOwned,
		E: Serialize + DeserializeOwned,
	{
		let (initial_state, last_fetched_ms) = initial_query_state(hydrated_query_state(&key.id));
		let id = key.id;
		let scope = Rc::new(ReactiveScope::new());
		let (state, is_fetching) = scope.enter(|| (Signal::new(initial_state), Signal::new(false)));

		Self {
			_scope: scope,
			id,
			state,
			is_fetching,
			fetcher: RefCell::new(key.fetcher),
			request: RefCell::new(None),
			next_generation: Cell::new(0),
			completed: RefCell::new(None),
			waiters: RefCell::new(Vec::new()),
			lease_count: Cell::new(0),
			retain_lease_count: Cell::new(0),
			refetch_after_in_flight: Cell::new(false),
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
		self.state
			.with_untracked(|state| matches!(state, ResourceState::Loading) || self.is_stale())
	}

	fn has_request(&self) -> bool {
		self.request.borrow().is_some()
	}

	fn next_request_generation(&self) -> u64 {
		let generation = self.next_generation.get();
		self.next_generation.set(generation.wrapping_add(1));
		generation
	}

	fn cancel_request(&self) {
		if let Some(request) = self.request.borrow_mut().take() {
			request.source.cancel();
			self.refetch_after_in_flight.set(false);
			self.is_fetching.set(false);
			self.wake_waiters();
		}
	}

	fn wake_waiters(&self) {
		let waiters = std::mem::take(&mut *self.waiters.borrow_mut());
		for waiter in waiters {
			waiter.wake();
		}
	}

	// The lease result future registers here while a navigation waits for a
	// generation to settle; later loader tasks will exercise this path.
	#[allow(dead_code)]
	fn register_waiter(&self, waker: &Waker) {
		let mut waiters = self.waiters.borrow_mut();
		if !waiters.iter().any(|previous| previous.will_wake(waker)) {
			waiters.push(waker.clone());
		}
	}

	fn make_lease(
		self: &Rc<Self>,
		generation: Option<u64>,
		error_policy: QueryErrorPolicy,
	) -> QueryLease<T, E> {
		self.lease_count.set(self.lease_count.get() + 1);
		let retains_errors = error_policy == QueryErrorPolicy::Retain;
		if retains_errors {
			self.retain_lease_count
				.set(self.retain_lease_count.get() + 1);
		}
		QueryLease {
			inner: Rc::new(QueryLeaseInner {
				entry: Rc::clone(self),
				generation: Cell::new(generation),
				retains_errors,
			}),
		}
	}

	fn acquire(self: &Rc<Self>, options: QueryAcquireOptions) -> QueryLease<T, E>
	where
		T: Serialize + DeserializeOwned,
		E: Serialize + DeserializeOwned,
	{
		let _consumer = options.consumer;
		let should_fetch = if self.has_request() {
			false
		} else if options.error_policy == QueryErrorPolicy::Retain {
			self.should_fetch_on_mount()
		} else {
			match self.state.with_untracked(|state| state.clone()) {
				ResourceState::Success(_) => self.is_stale(),
				ResourceState::Error(_) => true,
				ResourceState::Loading => true,
			}
		};
		// Register interest before starting work. Native test execution may poll
		// a ready fetch synchronously, and completion must observe this lease when
		// deciding whether an error is retainable or whether invalidation queues a
		// follow-up request.
		let lease = self.make_lease(None, options.error_policy);
		let generation = if should_fetch {
			Some(self.start_fetch(false))
		} else {
			self.request
				.borrow()
				.as_ref()
				.map(|request| request.generation)
		};
		lease.inner.generation.set(generation);
		lease
	}

	#[cfg(native)]
	fn mark_resolved_fetched(&self) {
		if self.state.with_untracked(|state| {
			matches!(state, ResourceState::Success(_) | ResourceState::Error(_))
		}) {
			self.last_fetched_ms.set(Some(now_ms()));
		}
	}

	fn start_fetch(self: &Rc<Self>, force: bool) -> u64 {
		if self.has_request() {
			if force {
				self.refetch_after_in_flight.set(true);
			}
			return self
				.request
				.borrow()
				.as_ref()
				.map(|request| request.generation)
				.unwrap_or_default();
		}

		let had_success = self
			.state
			.with_untracked(|state| matches!(state, ResourceState::Success(_)));
		if !force && had_success && !self.is_stale() {
			return self.next_generation.get();
		}
		let generation = self.next_request_generation();
		let source = CancellationSource::new();
		let token = source.handle();
		let (abort_handle, abort_registration) = AbortHandle::new_pair();
		let guard = AbortableTaskGuard::new(abort_handle);
		*self.request.borrow_mut() = Some(QueryRequest {
			generation,
			source,
			_guard: guard,
			_marker: PhantomData,
		});
		self.is_fetching.set(true);
		if !had_success {
			self.state.set(ResourceState::Loading);
		}

		let entry = Rc::clone(self);
		let fetch_entry = Rc::clone(&entry);
		let scope = entry._scope.id();
		let scoped = ScopedQueryFuture {
			scope,
			future: Box::pin(async move {
				let result = scope_cancellation(token, async move {
					let fetcher = fetch_entry.fetcher.borrow().clone();
					fetcher().await
				})
				.await;
				entry.complete_fetch(generation, result);
			}),
		};
		spawn_query_task(async move {
			let _ = Abortable::new(scoped, abort_registration).await;
		});
		generation
	}

	fn complete_fetch(self: &Rc<Self>, generation: u64, result: Result<T, E>) {
		let cancelled = self
			.request
			.borrow()
			.as_ref()
			.map(|request| request.source.handle().is_cancelled())
			.unwrap_or(true);
		let matches_request = self
			.request
			.borrow()
			.as_ref()
			.is_some_and(|request| request.generation == generation);
		if cancelled || !matches_request {
			return;
		}
		self.request.borrow_mut().take();
		self.completed
			.borrow_mut()
			.replace((generation, result.clone()));
		match result {
			Ok(value) => {
				self.last_fetched_ms.set(Some(now_ms()));
				self.state.set(ResourceState::Success(value));
			}
			Err(error) => {
				if self.retain_lease_count.get() > 0 {
					self.last_fetched_ms.set(Some(now_ms()));
				} else {
					self.last_fetched_ms.set(None);
				}
				self.state.set(ResourceState::Error(error));
			}
		}
		self.is_fetching.set(false);
		self.wake_waiters();
		if self.refetch_after_in_flight.replace(false) && self.lease_count.get() > 0 {
			self.start_fetch(true);
		}
	}
}

// Route preparation consumes this future in later implementation tasks.
#[allow(dead_code)]
struct QueryResultFuture<T: Clone + 'static, E: Clone + 'static> {
	entry: Rc<QueryEntry<T, E>>,
	generation: Option<u64>,
}

impl<T: Clone + 'static, E: Clone + 'static> Future for QueryResultFuture<T, E> {
	type Output = Result<T, E>;

	fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.get_mut();
		if let Some(generation) = this.generation {
			if let Some((completed_generation, result)) = this.entry.completed.borrow().as_ref()
				&& *completed_generation == generation
			{
				return Poll::Ready(result.clone());
			}
		} else {
			match this.entry.state.with_untracked(|state| state.clone()) {
				ResourceState::Success(value) => return Poll::Ready(Ok(value)),
				ResourceState::Error(error) => return Poll::Ready(Err(error)),
				ResourceState::Loading => {}
			}
		}
		this.entry.register_waiter(context.waker());
		Poll::Pending
	}
}

impl<T: Clone + 'static, E: Clone + 'static> QueryLease<T, E> {
	// Route preparation consumes this result operation in later implementation
	// tasks; keep it available while the public hook remains synchronous.
	#[allow(dead_code)]
	pub(crate) async fn result(&self) -> Result<T, E> {
		QueryResultFuture {
			entry: Rc::clone(&self.inner.entry),
			generation: self.inner.generation.get(),
		}
		.await
	}

	// Route preparation reads the settled state when a loader joins cached work.
	#[allow(dead_code)]
	pub(crate) fn state(&self) -> ResourceState<T, E> {
		self.inner.entry.state.with_untracked(|state| state.clone())
	}
}

pub(crate) fn acquire_query<T, E>(
	key: QueryKey<T, E>,
	options: QueryAcquireOptions,
) -> QueryLease<T, E>
where
	T: Clone + Serialize + DeserializeOwned + 'static,
	E: Clone + Serialize + DeserializeOwned + 'static,
{
	query_entry(key).acquire(options)
}

/// Current phase of a query.
#[derive(Clone, Debug, PartialEq)]
pub enum QueryPhase<T, E> {
	/// The query has no successful value or error yet.
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
	lease: QueryLease<T, E>,
	guards: Rc<RefCell<Vec<QueryGuard>>>,
}

impl<T: Clone + 'static, E: Clone + 'static> Clone for QueryHandle<T, E> {
	fn clone(&self) -> Self {
		Self {
			entry: Rc::clone(&self.entry),
			lease: self.lease.clone(),
			guards: Rc::clone(&self.guards),
		}
	}
}

impl<T: Clone + 'static, E: Clone + 'static> QueryHandle<T, E> {
	fn mark_ssr_read(&self) {
		#[cfg(native)]
		crate::ssr::resource_context::mark_resource_read(&self.entry.id);
	}

	/// Returns this query's deterministic SSR hydration key.
	pub fn ssr_key(&self) -> &str {
		&self.entry.id
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
		self.phase().is_pending()
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

	let lease = acquire_query(
		key,
		QueryAcquireOptions {
			consumer: QueryConsumer::MountedQuery,
			error_policy: QueryErrorPolicy::Retain,
		},
	);
	let entry = Rc::clone(&lease.inner.entry);

	QueryHandle {
		entry,
		lease,
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
	#[cfg(any(wasm, test))]
	super::resource::reserve_client_resource_key(&id);
	let cache_id = scoped_query_cache_id(&id);
	QUERY_CACHE.with(|cache| {
		let mut cache = cache.borrow_mut();
		if let Some(cached) = cache.get(&cache_id) {
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
			cache_id,
			CachedQueryEntry {
				typed: entry.clone(),
				refetch: Rc::new({
					let entry = Rc::clone(&entry);
					move || {
						entry.start_fetch(true);
					}
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
		context.borrow_mut().reserve_call_order_key(key.id());
		let ssr_prefetch = key.ssr_prefetch;
		let entry = Rc::new(QueryEntry::new(key));
		if ssr_prefetch {
			let fetcher = entry.fetcher.borrow().clone();
			context.borrow_mut().register_resource_with_owner(
				entry.id.clone(),
				move || fetcher(),
				entry.state,
				Some(Rc::clone(&entry._scope)),
			);
			entry.mark_resolved_fetched();
		}
		let lease = entry.make_lease(None, QueryErrorPolicy::Retain);
		QueryHandle {
			entry: Rc::clone(&entry),
			lease,
			guards: Rc::new(RefCell::new(Vec::new())),
		}
	})
}

fn invalidate_query_id(id: &str) {
	let cache_id = scoped_query_cache_id(id);
	QUERY_CACHE.with(|cache| {
		if let Some(cached) = cache.borrow().get(&cache_id) {
			(cached.refetch)();
		}
	});
}

fn scoped_query_cache_id(id: &str) -> String {
	#[cfg(all(native, feature = "testing"))]
	if let Some(scope_id) = crate::testing::component::active_query_scope_id() {
		return format!("test-screen:{scope_id}:{id}");
	}

	id.to_string()
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

#[cfg(all(test, not(wasm)))]
thread_local! {
	static INLINE_QUERY_TASK_DEPTH: Cell<usize> = const { Cell::new(0) };
	static DEFERRED_QUERY_TASKS: RefCell<std::collections::VecDeque<Pin<Box<dyn Future<Output = ()> + 'static>>>> =
		const { RefCell::new(std::collections::VecDeque::new()) };
}

#[cfg(all(test, not(wasm)))]
struct InlineQueryTaskGuard;

#[cfg(all(test, not(wasm)))]
impl InlineQueryTaskGuard {
	fn new() -> Option<Self> {
		INLINE_QUERY_TASK_DEPTH.with(|depth| {
			let current = depth.get();
			if current == 0 {
				depth.set(1);
				Some(Self)
			} else {
				None
			}
		})
	}
}

#[cfg(all(test, not(wasm)))]
impl Drop for InlineQueryTaskGuard {
	fn drop(&mut self) {
		INLINE_QUERY_TASK_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
	}
}

#[cfg(all(test, not(wasm)))]
fn spawn_query_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	if crate::platform::has_task_sink() {
		schedule_query_task(fut);
	} else {
		let Some(_guard) = InlineQueryTaskGuard::new() else {
			DEFERRED_QUERY_TASKS.with(|tasks| tasks.borrow_mut().push_back(Box::pin(fut)));
			return;
		};
		tokio_test::block_on(async move {
			fut.await;
			loop {
				let task = DEFERRED_QUERY_TASKS.with(|tasks| tasks.borrow_mut().pop_front());
				let Some(task) = task else {
					break;
				};
				task.await;
			}
		});
	}
}

#[cfg(any(not(test), wasm))]
fn spawn_query_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	schedule_query_task(fut);
}

fn schedule_query_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	crate::platform::spawn_task(async move {
		crate::platform::defer_yield().await;
		fut.await;
	});
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

#[cfg(all(test, not(wasm)))]
pub(crate) fn clear_query_cache_for_test() {
	QUERY_CACHE.with(|cache| cache.borrow_mut().clear());
}

#[cfg(all(test, not(wasm)))]
mod tests {
	use std::cell::Cell;
	use std::collections::VecDeque;
	use std::future::Future;
	use std::pin::Pin;
	use std::task::{Context, Poll, Waker};

	use reinhardt_core::reactive::ReactiveScope;
	use rstest::rstest;
	use serde::Serializer;
	use serde::ser::SerializeMap;
	use serial_test::serial;

	use super::*;

	struct OrderedMapArgs(&'static [(&'static str, i64)]);

	impl Serialize for OrderedMapArgs {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			let mut map = serializer.serialize_map(Some(self.0.len()))?;
			for (key, value) in self.0 {
				map.serialize_entry(key, value)?;
			}
			map.end()
		}
	}

	struct OrderedLargeMapArgs(&'static [(&'static str, u128)]);

	impl Serialize for OrderedLargeMapArgs {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			let mut map = serializer.serialize_map(Some(self.0.len()))?;
			for (key, value) in self.0 {
				map.serialize_entry(key, value)?;
			}
			map.end()
		}
	}

	type TestTask = Pin<Box<dyn Future<Output = ()> + 'static>>;

	fn poll_one_task(tasks: &Rc<RefCell<VecDeque<TestTask>>>) -> Poll<()> {
		let mut task = tasks
			.borrow_mut()
			.pop_front()
			.expect("a query request should schedule one task");
		let mut context = Context::from_waker(Waker::noop());
		let result = task.as_mut().poll(&mut context);
		if result.is_pending() {
			tasks.borrow_mut().push_back(task);
		}
		result
	}

	struct TestGate {
		ready: Rc<Cell<bool>>,
		dropped: Rc<Cell<usize>>,
		result: Option<Result<String, String>>,
	}

	impl Future for TestGate {
		type Output = Result<String, String>;

		fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
			let this = self.get_mut();
			if this.ready.get() {
				Poll::Ready(
					this.result
						.take()
						.expect("test gate polled after completion"),
				)
			} else {
				Poll::Pending
			}
		}
	}

	impl Drop for TestGate {
		fn drop(&mut self) {
			self.dropped.set(self.dropped.get() + 1);
		}
	}

	#[test]
	#[serial(query_cache)]
	fn imperative_acquisition_deduplicates_in_flight_work() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let calls = Rc::new(Cell::new(0));
			let key = QueryKey::new("imperative-dedupe", {
				let calls = Rc::clone(&calls);
				move || {
					calls.set(calls.get() + 1);
					async { Ok::<_, String>("value".to_string()) }
				}
			});

			// Act
			let first = acquire_query(
				key.clone(),
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(1),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			let second = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(2),
					error_policy: QueryErrorPolicy::Discard,
				},
			);

			// Assert
			assert_eq!(calls.get(), 0, "acquisition must not run a second fetch");
			assert_eq!(tasks.borrow().len(), 1);
			assert_eq!(poll_one_task(&tasks), Poll::Ready(()));
			assert_eq!(calls.get(), 1);
			assert_eq!(
				tokio_test::block_on(first.result()),
				Ok("value".to_string())
			);
			assert_eq!(
				tokio_test::block_on(second.result()),
				Ok("value".to_string())
			);
		});
	}

	#[test]
	#[serial(query_cache)]
	fn dropping_one_of_two_leases_keeps_request_alive() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let ready = Rc::new(Cell::new(false));
			let dropped = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("two-leases", {
				let ready = Rc::clone(&ready);
				let dropped = Rc::clone(&dropped);
				move || TestGate {
					ready: Rc::clone(&ready),
					dropped: Rc::clone(&dropped),
					result: Some(Ok("shared".to_string())),
				}
			});
			let entry = query_entry(key.clone());
			let first = acquire_query(
				key.clone(),
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(1),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			let second = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::MountedRoute(2),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Pending);

			// Act
			drop(first);
			assert_eq!(entry.lease_count.get(), 1);
			assert!(entry.has_request(), "the remaining lease keeps work alive");
			ready.set(true);
			let completion = poll_one_task(&tasks);

			// Assert
			assert_eq!(completion, Poll::Ready(()));
			assert_eq!(
				entry.state.with_untracked(|state| state.clone()),
				ResourceState::Success("shared".to_string())
			);
			assert_eq!(
				tokio_test::block_on(second.result()),
				Ok("shared".to_string())
			);
			assert_eq!(dropped.get(), 1);
		});
	}

	#[test]
	#[serial(query_cache)]
	fn dropping_final_lease_cancels_request_once() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let ready = Rc::new(Cell::new(false));
			let dropped = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("final-lease-cancel", {
				let ready = Rc::clone(&ready);
				let dropped = Rc::clone(&dropped);
				move || TestGate {
					ready: Rc::clone(&ready),
					dropped: Rc::clone(&dropped),
					result: Some(Ok("never-published".to_string())),
				}
			});
			let entry = query_entry(key.clone());
			let lease = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(3),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Pending);
			let cancelled = Rc::new(Cell::new(0));
			let cancelled_for_callback = Rc::clone(&cancelled);
			let registration = entry
				.request
				.borrow()
				.as_ref()
				.expect("the pending request must be owned by the entry")
				.source
				.register(move || cancelled_for_callback.set(cancelled_for_callback.get() + 1));

			// Act
			drop(lease);
			let completion = poll_one_task(&tasks);

			// Assert
			assert_eq!(cancelled.get(), 1, "the source must cancel exactly once");
			assert!(entry.request.borrow().is_none());
			assert!(!entry.is_fetching.get());
			assert_eq!(completion, Poll::Ready(()));
			assert_eq!(dropped.get(), 1, "the aborted fetch future must be dropped");
			drop(registration);
			let _ = ready;
		});
	}

	#[test]
	#[serial(query_cache)]
	fn queued_refetch_keeps_completed_generation_for_existing_lease() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let ready = Rc::new(Cell::new(false));
			let dropped = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("queued-generation", {
				let ready = Rc::clone(&ready);
				let dropped = Rc::clone(&dropped);
				move || TestGate {
					ready: Rc::clone(&ready),
					dropped: Rc::clone(&dropped),
					result: Some(Ok("first".to_string())),
				}
			});
			let entry = query_entry(key.clone());
			let lease = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(11),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Pending);
			let _ = entry.start_fetch(true);

			// Act
			ready.set(true);
			assert_eq!(poll_one_task(&tasks), Poll::Ready(()));
			let mut result = Box::pin(lease.result());
			let mut context = Context::from_waker(Waker::noop());

			// Assert
			assert!(
				entry.has_request(),
				"the queued refetch must start after completion"
			);
			assert_eq!(
				result.as_mut().poll(&mut context),
				Poll::Ready(Ok("first".to_string()))
			);
			drop(result);
			drop(lease);
		});
	}

	#[test]
	#[serial(query_cache)]
	fn cancelling_request_discards_queued_refetch() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let ready = Rc::new(Cell::new(false));
			let dropped = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("cancel-queued-refetch", {
				let ready = Rc::clone(&ready);
				let dropped = Rc::clone(&dropped);
				move || TestGate {
					ready: Rc::clone(&ready),
					dropped: Rc::clone(&dropped),
					result: Some(Ok("replacement".to_string())),
				}
			});
			let entry = query_entry(key.clone());
			let lease = acquire_query(
				key.clone(),
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(12),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Pending);
			let _ = entry.start_fetch(true);
			assert!(entry.refetch_after_in_flight.get());

			// Act
			drop(lease);

			// Assert
			assert!(!entry.refetch_after_in_flight.get());
			assert_eq!(poll_one_task(&tasks), Poll::Ready(()));
			ready.set(true);
			let replacement = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(13),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Ready(()));
			assert!(
				!entry.has_request(),
				"a cancelled request must not schedule a stale follow-up fetch"
			);
			assert_eq!(
				tokio_test::block_on(replacement.result()),
				Ok("replacement".to_string())
			);
		});
	}

	#[test]
	#[serial(query_cache)]
	fn cancel_completion_race_does_not_publish_obsolete_value() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let ready = Rc::new(Cell::new(false));
			let dropped = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("cancel-race", {
				let ready = Rc::clone(&ready);
				let dropped = Rc::clone(&dropped);
				move || TestGate {
					ready: Rc::clone(&ready),
					dropped: Rc::clone(&dropped),
					result: Some(Ok("obsolete".to_string())),
				}
			});
			let entry = query_entry(key.clone());
			let lease = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(4),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Pending);
			let generation = entry
				.request
				.borrow()
				.as_ref()
				.expect("the request generation must be visible")
				.generation;

			// Act
			drop(lease);
			entry.complete_fetch(generation, Ok("obsolete".to_string()));
			let completion = poll_one_task(&tasks);

			// Assert
			assert_eq!(completion, Poll::Ready(()));
			assert_eq!(
				entry.state.with_untracked(|state| state.clone()),
				ResourceState::Loading
			);
			assert!(entry.completed.borrow().is_none());
			assert_eq!(dropped.get(), 1);
			let _ = ready;
		});
	}

	#[test]
	#[serial(query_cache)]
	fn cancelled_revalidation_preserves_previous_success() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let ready = Rc::new(Cell::new(false));
			let dropped = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("cancel-revalidation", {
				let ready = Rc::clone(&ready);
				let dropped = Rc::clone(&dropped);
				move || TestGate {
					ready: Rc::clone(&ready),
					dropped: Rc::clone(&dropped),
					result: Some(Ok("new".to_string())),
				}
			})
			.with_stale_time(Duration::ZERO);
			let entry = query_entry(key.clone());
			entry.state.set(ResourceState::Success("old".to_string()));
			entry.last_fetched_ms.set(Some(now_ms()));
			let lease = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(5),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Pending);

			// Act
			drop(lease);
			ready.set(true);
			let completion = poll_one_task(&tasks);

			// Assert
			assert_eq!(completion, Poll::Ready(()));
			assert_eq!(
				entry.state.with_untracked(|state| state.clone()),
				ResourceState::Success("old".to_string())
			);
			assert!(!entry.is_fetching.get());
			assert!(entry.last_fetched_ms.get().is_some());
			assert_eq!(dropped.get(), 1);
		});
	}

	#[test]
	#[serial(query_cache)]
	fn discarded_error_retries_on_next_acquisition() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let calls = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("discarded-error", {
				let calls = Rc::clone(&calls);
				move || {
					calls.set(calls.get() + 1);
					async { Err::<String, _>("route failed".to_string()) }
				}
			});

			// Act
			let first = acquire_query(
				key.clone(),
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(6),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(
				tokio_test::block_on(first.result()),
				Err("route failed".to_string())
			);
			let second = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(7),
					error_policy: QueryErrorPolicy::Discard,
				},
			);

			// Assert
			assert_eq!(calls.get(), 2);
			assert_eq!(
				tokio_test::block_on(second.result()),
				Err("route failed".to_string())
			);
		});
	}

	#[test]
	#[serial(query_cache)]
	fn invalidation_survives_cancelled_request() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let tasks = Rc::new(RefCell::new(VecDeque::new()));
			let tasks_for_sink = Rc::clone(&tasks);
			let _sink = crate::platform::install_task_sink(move |task| {
				tasks_for_sink.borrow_mut().push_back(task);
			});
			let ready = Rc::new(Cell::new(false));
			let dropped = Rc::new(Cell::new(0));
			let calls = Rc::new(Cell::new(0));
			let key: QueryKey<String, String> = QueryKey::new("cancel-then-invalidate", {
				let ready = Rc::clone(&ready);
				let dropped = Rc::clone(&dropped);
				let calls = Rc::clone(&calls);
				move || {
					calls.set(calls.get() + 1);
					TestGate {
						ready: Rc::clone(&ready),
						dropped: Rc::clone(&dropped),
						result: Some(Ok("refetched".to_string())),
					}
				}
			});
			let lease = acquire_query(
				key,
				QueryAcquireOptions {
					consumer: QueryConsumer::Navigation(8),
					error_policy: QueryErrorPolicy::Discard,
				},
			);
			assert_eq!(poll_one_task(&tasks), Poll::Pending);

			// Act
			drop(lease);
			ready.set(true);
			invalidate_query_id("cancel-then-invalidate");
			assert_eq!(
				tasks.borrow().len(),
				2,
				"the invalidation must queue a replacement request"
			);
			assert_eq!(poll_one_task(&tasks), Poll::Ready(()));
			assert_eq!(poll_one_task(&tasks), Poll::Ready(()));

			// Assert
			assert_eq!(calls.get(), 2);
			assert_eq!(dropped.get(), 2);
			let entry = query_entry(QueryKey::new("cancel-then-invalidate", || async {
				Ok::<_, String>("unused".to_string())
			}));
			assert_eq!(
				entry.state.with_untracked(|state| state.clone()),
				ResourceState::Success("refetched".to_string())
			);
		});
	}

	#[rstest]
	#[serial(query_cache)]
	fn use_query_deduplicates_shared_key() {
		ReactiveScope::run(|| {
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
		});
	}

	#[rstest]
	#[serial(query_cache)]
	fn cached_query_survives_the_scope_that_created_it() {
		// Arrange
		clear_query_cache_for_test();
		let key = QueryKey::new("retained-cache-entry", || async {
			Ok::<_, String>("cached".to_string())
		});
		let scope = ReactiveScope::new();
		let first = scope.enter(|| use_query(key.clone()));
		assert_eq!(first.data(), Some("cached".to_string()));
		drop(first);
		drop(scope);

		// Act
		let cached = ReactiveScope::run(|| use_query(key));

		// Assert
		assert_eq!(cached.data(), Some("cached".to_string()));
	}

	#[rstest]
	#[serial(query_cache)]
	fn refetch_runs_fetcher_again() {
		ReactiveScope::run(|| {
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
		});
	}

	#[rstest]
	#[serial(query_cache)]
	fn failed_query_respects_stale_time_before_retrying() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let calls = Rc::new(Cell::new(0));
			let key = QueryKey::new("failed-query", {
				let calls = Rc::clone(&calls);
				move || {
					calls.set(calls.get() + 1);
					async { Err::<String, _>("not found".to_string()) }
				}
			})
			.with_stale_time(Duration::from_secs(30));

			// Act
			let first = use_query(key.clone());
			let second = use_query(key);

			// Assert
			assert_eq!(calls.get(), 1);
			assert_eq!(first.error(), Some("not found".to_string()));
			assert_eq!(second.error(), Some("not found".to_string()));
		});
	}

	#[rstest]
	#[serial(query_cache)]
	fn successful_query_is_not_pending_during_background_fetch() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let entry = Rc::new(QueryEntry::new(QueryKey::new(
				"background-refetch",
				|| async { Ok::<_, String>("fresh".to_string()) },
			)));
			entry
				.state
				.set(ResourceState::Success("cached".to_string()));
			entry.is_fetching.set(true);
			let lease = entry.make_lease(None, QueryErrorPolicy::Retain);
			let query = QueryHandle {
				entry,
				lease,
				guards: Rc::new(RefCell::new(Vec::new())),
			};

			// Act
			let data = query.data();
			let is_fetching = query.is_fetching();
			let is_pending = query.is_pending();

			// Assert
			assert_eq!(data, Some("cached".to_string()));
			assert!(is_fetching);
			assert!(!is_pending);
		});
	}

	#[rstest]
	#[serial(query_cache)]
	fn mutation_success_invalidates_registered_query() {
		ReactiveScope::run(|| {
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
			let mutation = use_mutation(|_: ()| async { Ok::<_, String>("done".to_string()) })
				.invalidates(key);

			// Act
			mutation.force_success_for_test("done".to_string());

			// Assert
			assert_eq!(calls.get(), 2);
			assert_eq!(query.data(), Some(2));
		});
	}

	#[rstest]
	#[serial(query_cache)]
	fn invalidation_during_in_flight_fetch_runs_after_completion() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			let calls = Rc::new(Cell::new(0));

			// Act
			let query = use_query(QueryKey::new("queued-invalidation", {
				let calls = Rc::clone(&calls);
				move || {
					let calls = Rc::clone(&calls);
					async move {
						let value = calls.get() + 1;
						calls.set(value);
						if value == 1 {
							invalidate_query_id("queued-invalidation");
						}
						Ok::<_, String>(value)
					}
				}
			}));

			// Assert
			assert_eq!(calls.get(), 2);
			assert_eq!(query.data(), Some(2));
		});
	}

	#[test]
	#[serial(query_cache)]
	fn manual_query_call_order_key_reserves_client_resource_counter() {
		ReactiveScope::run(|| {
			// Arrange
			clear_query_cache_for_test();
			super::super::resource::set_client_resource_counter(0);

			// Act
			let _entry = query_entry(QueryKey::new("rh-res-0", || async {
				Ok::<_, String>("query".to_string())
			}));

			// Assert
			assert_eq!(super::super::resource::current_client_resource_counter(), 1);
			super::super::resource::set_client_resource_counter(0);
		});
	}

	#[test]
	#[serial(query_cache)]
	fn hydrated_query_error_is_fresh_on_first_mount() {
		ReactiveScope::run(|| {
			// Arrange
			let (hydrated_state, last_fetched_ms) =
				initial_query_state(Some(ResourceState::Error("not found".to_string())));
			let entry = QueryEntry::new(QueryKey::new("hydrated-query-error", || async {
				Err::<String, _>("not found".to_string())
			}));
			entry.state.set(hydrated_state);
			entry.last_fetched_ms.set(last_fetched_ms);

			// Assert
			assert!(
				!entry.should_fetch_on_mount(),
				"a freshly hydrated error must remain visible for the initial mount"
			);
		});
	}

	#[tokio::test]
	#[serial(query_cache)]
	async fn ssr_replayed_query_error_is_fresh_for_stale_time() {
		// Arrange
		let context = Rc::new(RefCell::new(
			crate::ssr::resource_context::SsrResourceContext::new(Duration::from_secs(1)),
		));

		let discovery_query =
			crate::ssr::resource_context::scope_context(Rc::clone(&context), async {
				ReactiveScope::run(|| {
					let query =
						try_create_ssr_query(QueryKey::new("ssr-replayed-query-error", || async {
							Err::<String, _>("not found".to_string())
						}))
						.expect("active SSR context should create the query");
					let _ = query.get();
					query
				})
			})
			.await;
		assert!(crate::ssr::resource_context::resolve_external_resources(&context).await);

		// Act
		let replayed_query =
			crate::ssr::resource_context::scope_context(Rc::clone(&context), async {
				ReactiveScope::run(|| {
					let query =
						try_create_ssr_query(QueryKey::new("ssr-replayed-query-error", || async {
							Err::<String, _>("must not refetch during replay".to_string())
						}))
						.expect("active SSR context should replay the query");
					let query = query.stale_time(Duration::from_secs(30));

					// Assert
					assert_eq!(query.get(), ResourceState::Error("not found".to_string()));
					assert!(
						!query.entry.is_stale(),
						"a replayed error must remain fresh when stale_time is applied"
					);
					query
				})
			})
			.await;
		drop(replayed_query);
		drop(discovery_query);
	}

	#[rstest]
	#[serial(query_cache)]
	fn server_fn_key_hashes_arguments_without_exposing_them() {
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
		assert_eq!(
			key.id(),
			"server_fn:/api/server_fn/list_jobs:json:sha256:b86b1ea11b28136fe5224b9d1e3017b7efb68d4fae0b90c4940e0c0f89b3907a"
		);
		assert!(!key.id().contains("[42]"));
	}

	#[rstest]
	#[serial(query_cache)]
	fn server_fn_key_preserves_large_integer_arguments() {
		// Arrange
		clear_query_cache_for_test();

		struct Marker;

		impl crate::server_fn::ServerFnMetadata for Marker {
			const PATH: &'static str = "/api/server_fn/load_job";
			const NAME: &'static str = "load_job";
			const CODEC: &'static str = "json";
			const IS_JSON_CODEC: bool = true;
		}

		// Act
		let key: QueryKey<(), crate::server_fn::ServerFnError> =
			QueryKey::from_server_fn::<Marker, _, _, _>((u128::MAX,), || async { Ok(()) });

		// Assert
		assert_eq!(
			key.id(),
			"server_fn:/api/server_fn/load_job:json:sha256:d80bcc323657a82faa939889d29892c9b53c3bb4f98ff3738140a27a3ac7b9df"
		);
		assert!(!key.id().contains(&u128::MAX.to_string()));
	}

	#[rstest]
	#[serial(query_cache)]
	fn server_fn_key_canonicalizes_object_arguments() {
		// Arrange
		clear_query_cache_for_test();

		struct Marker;

		impl crate::server_fn::ServerFnMetadata for Marker {
			const PATH: &'static str = "/api/server_fn/filter_jobs";
			const NAME: &'static str = "filter_jobs";
			const CODEC: &'static str = "json";
			const IS_JSON_CODEC: bool = true;
		}

		// Act
		let first: QueryKey<(), crate::server_fn::ServerFnError> =
			QueryKey::from_server_fn::<Marker, _, _, _>(
				(OrderedMapArgs(&[("status", 1), ("owner", 2)]),),
				|| async { Ok(()) },
			);
		let second: QueryKey<(), crate::server_fn::ServerFnError> =
			QueryKey::from_server_fn::<Marker, _, _, _>(
				(OrderedMapArgs(&[("owner", 2), ("status", 1)]),),
				|| async { Ok(()) },
			);

		// Assert
		assert_eq!(first.id(), second.id());
		assert_eq!(
			first.id(),
			"server_fn:/api/server_fn/filter_jobs:json:sha256:b2b2c11c6c2d2aacfabe8dba6102508d46a7690b66d0662adc332e4802f078d2"
		);
	}

	#[rstest]
	#[serial(query_cache)]
	fn server_fn_key_canonicalizes_large_integer_object_arguments() {
		// Arrange
		clear_query_cache_for_test();

		struct Marker;

		impl crate::server_fn::ServerFnMetadata for Marker {
			const PATH: &'static str = "/api/server_fn/filter_large_jobs";
			const NAME: &'static str = "filter_large_jobs";
			const CODEC: &'static str = "json";
			const IS_JSON_CODEC: bool = true;
		}

		// Act
		let first: QueryKey<(), crate::server_fn::ServerFnError> =
			QueryKey::from_server_fn::<Marker, _, _, _>(
				(OrderedLargeMapArgs(&[("status", u128::MAX), ("owner", 2)]),),
				|| async { Ok(()) },
			);
		let second: QueryKey<(), crate::server_fn::ServerFnError> =
			QueryKey::from_server_fn::<Marker, _, _, _>(
				(OrderedLargeMapArgs(&[("owner", 2), ("status", u128::MAX)]),),
				|| async { Ok(()) },
			);

		// Assert
		assert_eq!(first.id(), second.id());
	}

	#[rstest]
	#[serial(query_cache)]
	fn server_fn_key_does_not_expose_sensitive_arguments() {
		// Arrange
		clear_query_cache_for_test();

		struct Marker;

		impl crate::server_fn::ServerFnMetadata for Marker {
			const PATH: &'static str = "/api/server_fn/load_user";
			const NAME: &'static str = "load_user";
			const CODEC: &'static str = "json";
			const IS_JSON_CODEC: bool = true;
		}

		let email = "sensitive@example.com";

		// Act
		let key: QueryKey<(), crate::server_fn::ServerFnError> =
			QueryKey::from_server_fn::<Marker, _, _, _>((email,), || async { Ok(()) });

		// Assert
		assert_eq!(
			key.id(),
			"server_fn:/api/server_fn/load_user:json:sha256:5cb828e12cdd77b9af33cfac3c965b44acc673692df8ffb22bc6794506ea59bc"
		);
		assert!(!key.id().contains(email));
	}
}
