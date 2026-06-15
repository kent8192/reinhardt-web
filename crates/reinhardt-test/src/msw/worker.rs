//! `MockServiceWorker` — the core orchestrator for MSW-style fetch interception.

use std::cell::{Cell, RefCell};

use js_sys::Promise;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

use reinhardt_pages::server_fn::{MockableServerFn, ServerFnError};

use super::context::TestContext;
use super::handler::RestHandler;
use super::handler::{ServerFnContextHandler, ServerFnHandler};
use super::interceptor;
use super::matcher::UrlMatcher;
use super::recorder::{CallQuery, RecordedRequest, ServerFnCallQuery};
use super::state::{RecorderHandle, SharedHandlers};

struct ActiveInterceptor {
	owner_id: u64,
	original_fetch: JsValue,
	_closure: Closure<dyn FnMut(JsValue, JsValue) -> Promise>,
}

thread_local! {
	static NEXT_WORKER_ID: Cell<u64> = const { Cell::new(1) };
	static ACTIVE_INTERCEPTOR: RefCell<Option<ActiveInterceptor>> = const { RefCell::new(None) };
}

fn next_worker_id() -> u64 {
	NEXT_WORKER_ID.with(|next| {
		let id = next.get();
		next.set(id.wrapping_add(1).max(1));
		id
	})
}

fn restore_active_interceptor() {
	ACTIVE_INTERCEPTOR.with(|active| {
		if let Some(active) = active.borrow_mut().take() {
			interceptor::restore_fetch(&active.original_fetch);
		}
	});
}

fn restore_interceptor_if_owned(owner_id: u64) {
	ACTIVE_INTERCEPTOR.with(|active| {
		let should_restore = active
			.borrow()
			.as_ref()
			.is_some_and(|active| active.owner_id == owner_id);
		if should_restore && let Some(active) = active.borrow_mut().take() {
			interceptor::restore_fetch(&active.original_fetch);
		}
	});
}

/// Policy for requests that don't match any registered handler.
#[derive(Debug, Clone)]
pub enum UnhandledPolicy {
	/// Return a network error with a descriptive message (default).
	/// Ensures test determinism by preventing accidental real network calls.
	Error,
	/// Pass through to the original `window.fetch`.
	Passthrough,
	/// Log a warning to `console.warn` and pass through.
	Warn,
}

impl From<&UnhandledPolicy> for interceptor::UnhandledPolicy {
	fn from(p: &UnhandledPolicy) -> Self {
		match p {
			UnhandledPolicy::Error => interceptor::UnhandledPolicy::Error,
			UnhandledPolicy::Passthrough => interceptor::UnhandledPolicy::Passthrough,
			UnhandledPolicy::Warn => interceptor::UnhandledPolicy::Warn,
		}
	}
}

/// MSW-style network-level request interceptor for WASM testing.
///
/// Overrides `window.fetch` to intercept HTTP requests and return mock
/// responses. Supports type-safe `server_fn` mocking, REST endpoint
/// mocking, request recording, and assertion helpers.
///
/// # Example
///
/// ```rust,ignore
/// let worker = MockServiceWorker::new();
/// worker.handle(rest::get("/api/data").respond(MockResponse::json(42)));
/// worker.start().await;
/// // ... test ...
/// worker.calls_to("/api/data").assert_called();
/// // worker.stop() called automatically on drop
/// ```
pub struct MockServiceWorker {
	worker_id: u64,
	handlers: SharedHandlers,
	recorder: RecorderHandle,
	unhandled_policy: UnhandledPolicy,
	active: Cell<bool>,
}

impl MockServiceWorker {
	/// Create a new worker with `UnhandledPolicy::Error` (default).
	pub fn new() -> Self {
		Self::with_policy(UnhandledPolicy::Error)
	}

	/// Create a new worker with a custom unhandled request policy.
	pub fn with_policy(policy: UnhandledPolicy) -> Self {
		Self {
			worker_id: next_worker_id(),
			handlers: SharedHandlers::new(),
			recorder: RecorderHandle::new(),
			unhandled_policy: policy,
			active: Cell::new(false),
		}
	}

	/// Install the fetch override. Must be called before component rendering.
	///
	/// # Panics
	///
	/// Panics if already started or if `window` is unavailable.
	pub async fn start(&self) {
		assert!(
			!self.active.get(),
			"MockServiceWorker: already started. Call stop() before starting again."
		);

		restore_active_interceptor();

		let original = interceptor::save_original_fetch();
		let closure = interceptor::install_fetch_override(
			self.handlers.clone(),
			self.recorder.clone(),
			(&self.unhandled_policy).into(),
			original.clone(),
		);

		ACTIVE_INTERCEPTOR.with(|active| {
			*active.borrow_mut() = Some(ActiveInterceptor {
				owner_id: self.worker_id,
				original_fetch: original,
				_closure: closure,
			});
		});
		self.active.set(true);
	}

	/// Restore original `window.fetch` and clean up.
	pub async fn stop(&self) {
		if self.active.get() {
			restore_interceptor_if_owned(self.worker_id);
			self.active.set(false);
		}
	}

	/// Remove all handlers and recorded requests.
	pub fn reset(&self) {
		self.handlers.clear();
		self.recorder.clear();
	}

	/// Remove all handlers but keep recorded requests.
	pub fn reset_handlers(&self) {
		self.handlers.clear();
	}

	// --- REST handler registration ---

	/// Register a REST handler.
	pub fn handle(&self, handler: RestHandler) {
		self.handlers.push(Box::new(handler));
	}

	// --- server_fn handler registration ---

	/// Register a type-safe server_fn handler.
	pub fn handle_server_fn<S: MockableServerFn>(
		&self,
		handler: impl Fn(S::Args) -> Result<S::Response, ServerFnError> + 'static,
	) {
		self.handlers.push(Box::new(ServerFnHandler::<S>::new(
			Box::new(handler),
			false,
			None,
		)));
	}

	/// Register a server_fn handler with a DI test context.
	pub fn handle_server_fn_with_context<S: MockableServerFn>(
		&self,
		context: TestContext,
		handler: impl Fn(S::Args, &TestContext) -> Result<S::Response, ServerFnError> + 'static,
	) {
		self.handlers
			.push(Box::new(ServerFnContextHandler::<S>::new(
				context,
				Box::new(handler),
				false,
				None,
			)));
	}

	// --- Query API ---

	/// Query recorded calls matching a URL pattern.
	pub fn calls_to(&self, pattern: impl Into<UrlMatcher>) -> CallQuery {
		CallQuery::new(&self.recorder, pattern)
	}

	/// Query recorded calls to a specific server function (type-safe).
	pub fn calls_to_server_fn<S: MockableServerFn>(&self) -> ServerFnCallQuery<S> {
		ServerFnCallQuery {
			inner: CallQuery::new(&self.recorder, S::PATH),
			_marker: std::marker::PhantomData,
		}
	}

	/// All recorded calls.
	pub fn all_calls(&self) -> Vec<RecordedRequest> {
		self.recorder.all()
	}
}

impl Drop for MockServiceWorker {
	fn drop(&mut self) {
		if self.active.get() {
			restore_interceptor_if_owned(self.worker_id);
		}
	}
}
