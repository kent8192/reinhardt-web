//! Request-scoped resource registration for server-side rendering.

use crate::reactive::{ResourceState, Signal};
use futures_util::future::join_all;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;
use tokio::time::timeout;

tokio::task_local! {
	static ACTIVE_CONTEXT: Rc<RefCell<SsrResourceContext>>;
}

type PendingResourceFuture = Pin<Box<dyn Future<Output = (String, Value)> + 'static>>;
type PendingResourceSubscriber = Box<dyn Fn(Value) + 'static>;

struct PendingResource {
	id: String,
	boundary_id: Option<String>,
	future: PendingResourceFuture,
	subscribers: Vec<PendingResourceSubscriber>,
}

/// Request-scoped SSR resource registry.
pub(crate) struct SsrResourceContext {
	next_resource_index: usize,
	pending: Vec<PendingResource>,
	resolved: HashMap<String, Value>,
	timed_out: HashSet<String>,
	boundary_stack: Vec<String>,
	timeout: Duration,
}

impl SsrResourceContext {
	/// Creates a resource registry for one SSR render.
	pub(crate) fn new(timeout: Duration) -> Self {
		Self {
			next_resource_index: 0,
			pending: Vec::new(),
			resolved: HashMap::new(),
			timed_out: HashSet::new(),
			boundary_stack: Vec::new(),
			timeout,
		}
	}

	/// Resets deterministic call-order resource IDs before a replay render.
	pub(crate) fn reset_call_order_keys(&mut self) {
		self.next_resource_index = 0;
	}

	/// Returns the current call-order resource index.
	pub(crate) fn call_order_index(&self) -> usize {
		self.next_resource_index
	}

	/// Restores the call-order resource index.
	pub(crate) fn set_call_order_index(&mut self, index: usize) {
		self.next_resource_index = index;
	}

	/// Allocates the next call-order resource key.
	pub(crate) fn next_call_order_key(&mut self) -> String {
		let id = self.next_resource_index;
		self.next_resource_index += 1;
		format!("rh-res-{id}")
	}

	fn push_boundary(&mut self, boundary_id: String) {
		self.boundary_stack.push(boundary_id);
	}

	fn pop_boundary(&mut self) {
		self.boundary_stack.pop();
	}

	fn current_boundary_id(&self) -> Option<String> {
		self.boundary_stack.last().cloned()
	}

	/// Registers a resource future unless the key is already known.
	pub(crate) fn register_resource<T, E, Fut>(
		&mut self,
		key: String,
		future: Fut,
		state: Signal<ResourceState<T, E>>,
	) where
		T: Clone + Serialize + DeserializeOwned + 'static,
		E: Clone + Serialize + DeserializeOwned + 'static,
		Fut: Future<Output = Result<T, E>> + 'static,
	{
		if let Some(value) = self.resolved.get(&key) {
			if let Ok(resource_state) = serde_json::from_value(value.clone()) {
				state.set(resource_state);
			}
			return;
		}
		if self.timed_out.contains(&key) {
			return;
		}

		let subscriber = Box::new(move |value: Value| {
			if let Ok(resource_state) = serde_json::from_value(value) {
				state.set(resource_state);
			}
		});

		if let Some(pending) = self.pending.iter_mut().find(|pending| pending.id == key) {
			pending.subscribers.push(subscriber);
			return;
		}

		let id = key.clone();
		let future = Box::pin(async move {
			let resource_state = match future.await {
				Ok(value) => ResourceState::Success(value),
				Err(error) => ResourceState::Error(error),
			};
			let value = serde_json::to_value(resource_state).unwrap_or(Value::Null);
			(id, value)
		});

		self.pending.push(PendingResource {
			id: key,
			boundary_id: self.current_boundary_id(),
			future,
			subscribers: vec![subscriber],
		});
	}

	/// Assigns already registered resource IDs to a Suspense boundary.
	pub(crate) fn assign_resources_to_boundary(&mut self, ids: &[String], boundary_id: &str) {
		if ids.is_empty() {
			return;
		}

		for pending in &mut self.pending {
			if pending.boundary_id.is_none() && ids.iter().any(|id| id == &pending.id) {
				pending.boundary_id = Some(boundary_id.to_string());
			}
		}
	}

	/// Returns a resolved resource state by key.
	pub(crate) fn resolved_resource_state<T, E>(&self, key: &str) -> Option<ResourceState<T, E>>
	where
		T: Clone + DeserializeOwned + 'static,
		E: Clone + DeserializeOwned + 'static,
	{
		self.resolved
			.get(key)
			.and_then(|value| serde_json::from_value(value.clone()).ok())
	}

	/// Returns resolved resource payloads.
	pub(crate) fn resolved_resources(&self) -> impl Iterator<Item = (&str, &Value)> {
		self.resolved
			.iter()
			.map(|(key, value)| (key.as_str(), value))
	}

	/// Returns whether any resource is pending for a Suspense boundary.
	pub(crate) fn has_pending_for_boundary(&self, boundary_id: &str) -> bool {
		self.pending
			.iter()
			.any(|pending| pending.boundary_id.as_deref() == Some(boundary_id))
	}

	fn take_matching(&mut self, matches: impl Fn(Option<&str>) -> bool) -> Vec<PendingResource> {
		let mut selected = Vec::new();
		let mut remaining = Vec::new();
		for pending in std::mem::take(&mut self.pending) {
			if matches(pending.boundary_id.as_deref()) {
				selected.push(pending);
			} else {
				remaining.push(pending);
			}
		}
		self.pending = remaining;
		selected
	}

	fn record_resolved(
		&mut self,
		id: String,
		subscribers: Vec<PendingResourceSubscriber>,
		value: Value,
	) {
		for subscriber in subscribers {
			subscriber(value.clone());
		}
		self.resolved.insert(id, value);
	}

	fn record_timeout(&mut self, id: String) {
		self.timed_out.insert(id);
	}
}

/// Guard that restores the active Suspense boundary stack on early exit.
pub(crate) struct SsrBoundaryGuard {
	context: Rc<RefCell<SsrResourceContext>>,
	active: bool,
}

impl Drop for SsrBoundaryGuard {
	fn drop(&mut self) {
		if self.active {
			self.context.borrow_mut().pop_boundary();
		}
	}
}

/// Runs an async render operation with a request-scoped SSR resource context.
pub(crate) async fn scope_context<R>(
	context: Rc<RefCell<SsrResourceContext>>,
	future: impl Future<Output = R>,
) -> R {
	ACTIVE_CONTEXT.scope(context, future).await
}

/// Pushes an active Suspense boundary and restores it on drop.
pub(crate) fn enter_boundary(
	context: &Rc<RefCell<SsrResourceContext>>,
	boundary_id: String,
) -> SsrBoundaryGuard {
	context.borrow_mut().push_boundary(boundary_id);
	SsrBoundaryGuard {
		context: Rc::clone(context),
		active: true,
	}
}

/// Executes a closure with the active SSR resource context.
pub(crate) fn with_active_context<R>(
	f: impl FnOnce(&Rc<RefCell<SsrResourceContext>>) -> R,
) -> Option<R> {
	ACTIVE_CONTEXT.try_with(f).ok()
}

/// Resolves resources outside Suspense boundaries.
pub(crate) async fn resolve_external_resources(context: &Rc<RefCell<SsrResourceContext>>) -> bool {
	resolve_matching(context, |boundary_id| boundary_id.is_none()).await
}

/// Resolves resources inside a Suspense boundary.
pub(crate) async fn resolve_boundary_resources(
	context: &Rc<RefCell<SsrResourceContext>>,
	boundary_id: &str,
) -> bool {
	resolve_matching(context, |candidate| candidate == Some(boundary_id)).await
}

/// Resolves every still-pending resource for buffered SSR.
pub(crate) async fn resolve_pending_resources(context: &Rc<RefCell<SsrResourceContext>>) -> bool {
	resolve_matching(context, |_| true).await
}

async fn resolve_matching(
	context: &Rc<RefCell<SsrResourceContext>>,
	matches: impl Fn(Option<&str>) -> bool,
) -> bool {
	let (timeout_duration, pending) = {
		let mut context = context.borrow_mut();
		let timeout_duration = context.timeout;
		let pending = context.take_matching(matches);
		(timeout_duration, pending)
	};

	let results = join_all(pending.into_iter().map(|pending| async move {
		let PendingResource {
			id,
			future,
			subscribers,
			..
		} = pending;
		match timeout(timeout_duration, future).await {
			Ok((_id, value)) => Ok((id, subscribers, value)),
			Err(_) => Err(id),
		}
	}))
	.await;

	let mut all_resolved = true;
	for result in results {
		match result {
			Ok((id, subscribers, value)) => {
				context.borrow_mut().record_resolved(id, subscribers, value)
			}
			Err(id) => {
				context.borrow_mut().record_timeout(id);
				all_resolved = false;
			}
		}
	}
	all_resolved
}
