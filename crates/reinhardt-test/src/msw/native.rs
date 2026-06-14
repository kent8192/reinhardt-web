//! Native `MockServiceWorker` runtime backed by a loopback HTTP server.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse, StatusCode};
use hyper_util::rt::TokioIo;
use reinhardt_pages::server_fn::{MockableServerFn, ServerFnError};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::context::TestContext;
use super::error::MswError;
use super::handler::{InterceptedRequest, RestHandler, ServerFnContextHandler, ServerFnHandler};
use super::matcher::UrlMatcher;
use super::recorder::{CallQuery, RecordedRequest, ServerFnCallQuery};
use super::response::MockResponse;
use super::state::{RecorderHandle, SharedHandlers};

/// Policy for requests that do not match a registered handler.
#[derive(Debug, Clone)]
pub enum UnhandledPolicy {
	/// Return a deterministic diagnostic error response.
	Error,
	/// Native passthrough is not supported in the first native runtime.
	Passthrough,
	/// Log a warning and return the same deterministic response as `Error`.
	Warn,
}

struct NativeRuntime {
	handle: JoinHandle<()>,
}

enum RuntimeState {
	Stopped,
	Starting,
	Started(NativeRuntime),
}

/// Native MSW-style mock server for HTTP clients that accept explicit endpoints.
pub struct MockServiceWorker {
	handlers: SharedHandlers,
	recorder: RecorderHandle,
	unhandled_policy: UnhandledPolicy,
	runtime: Mutex<RuntimeState>,
	url: Mutex<Option<String>>,
}

impl MockServiceWorker {
	/// Create a new worker with `UnhandledPolicy::Error`.
	pub fn new() -> Self {
		Self::with_policy(UnhandledPolicy::Error)
	}

	/// Create a new worker with a custom unhandled request policy.
	pub fn with_policy(policy: UnhandledPolicy) -> Self {
		Self {
			handlers: SharedHandlers::new(),
			recorder: RecorderHandle::new(),
			unhandled_policy: policy,
			runtime: Mutex::new(RuntimeState::Stopped),
			url: Mutex::new(None),
		}
	}

	/// Start the native loopback server or panic with a clear lifecycle error.
	pub async fn start(&self) {
		self.try_start()
			.await
			.expect("MockServiceWorker: failed to start native runtime");
	}

	/// Start the native loopback server.
	pub async fn try_start(&self) -> Result<(), MswError> {
		if matches!(self.unhandled_policy, UnhandledPolicy::Passthrough) {
			return Err(MswError::NativePassthroughUnsupported);
		}
		{
			let mut runtime = self.runtime.lock().expect("MSW runtime lock poisoned");
			match *runtime {
				RuntimeState::Stopped => {
					*runtime = RuntimeState::Starting;
				}
				RuntimeState::Starting | RuntimeState::Started(_) => {
					return Err(MswError::AlreadyStarted);
				}
			}
		}

		let listener = match TcpListener::bind("127.0.0.1:0").await {
			Ok(listener) => listener,
			Err(err) => {
				*self.runtime.lock().expect("MSW runtime lock poisoned") = RuntimeState::Stopped;
				return Err(MswError::Bind(err));
			}
		};
		let addr = match listener.local_addr() {
			Ok(addr) => addr,
			Err(err) => {
				*self.runtime.lock().expect("MSW runtime lock poisoned") = RuntimeState::Stopped;
				return Err(MswError::Bind(err));
			}
		};
		let url = format!("http://{addr}");
		let handlers = self.handlers.clone();
		let recorder = self.recorder.clone();
		let policy = self.unhandled_policy.clone();

		let handle = tokio::spawn(async move {
			serve(listener, handlers, recorder, policy).await;
		});

		*self.url.lock().expect("MSW URL lock poisoned") = Some(url);
		*self.runtime.lock().expect("MSW runtime lock poisoned") =
			RuntimeState::Started(NativeRuntime { handle });
		Ok(())
	}

	/// Stop the native loopback server.
	pub async fn stop(&self) {
		let runtime = {
			let mut runtime = self.runtime.lock().expect("MSW runtime lock poisoned");
			std::mem::replace(&mut *runtime, RuntimeState::Stopped)
		};
		if let RuntimeState::Started(runtime) = runtime {
			runtime.handle.abort();
			let _ = runtime.handle.await;
		}
		*self.url.lock().expect("MSW URL lock poisoned") = None;
	}

	/// Return the native server base URL.
	pub fn url(&self) -> String {
		self.url
			.lock()
			.expect("MSW URL lock poisoned")
			.as_ref()
			.expect("MockServiceWorker::url() called before start()")
			.clone()
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

	/// Register a REST handler.
	pub fn handle(&self, handler: RestHandler) {
		self.handlers.push(Box::new(handler));
	}

	/// Register a type-safe server function handler.
	pub fn handle_server_fn<S: MockableServerFn>(
		&self,
		handler: impl Fn(S::Args) -> Result<S::Response, ServerFnError> + Send + Sync + 'static,
	) {
		self.handlers.push(Box::new(ServerFnHandler::<S>::new(
			Box::new(handler),
			false,
			None,
		)));
	}

	/// Register a server function handler with a DI test context.
	pub fn handle_server_fn_with_context<S: MockableServerFn>(
		&self,
		context: TestContext,
		handler: impl Fn(S::Args, &TestContext) -> Result<S::Response, ServerFnError>
		+ Send
		+ Sync
		+ 'static,
	) {
		self.handlers
			.push(Box::new(ServerFnContextHandler::<S>::new(
				context,
				Box::new(handler),
				false,
				None,
			)));
	}

	/// Query recorded calls matching a URL pattern.
	pub fn calls_to(&self, pattern: impl Into<UrlMatcher>) -> CallQuery<'_> {
		CallQuery::new(&self.recorder, pattern)
	}

	/// Query recorded calls to a specific server function.
	pub fn calls_to_server_fn<S: MockableServerFn>(&self) -> ServerFnCallQuery<'_, S> {
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

impl Default for MockServiceWorker {
	fn default() -> Self {
		Self::new()
	}
}

impl Drop for MockServiceWorker {
	fn drop(&mut self) {
		let runtime = std::mem::replace(
			self.runtime.get_mut().expect("MSW runtime lock poisoned"),
			RuntimeState::Stopped,
		);
		if let RuntimeState::Started(runtime) = runtime {
			runtime.handle.abort();
		}
		self.url.get_mut().expect("MSW URL lock poisoned").take();
	}
}

async fn serve(
	listener: TcpListener,
	handlers: SharedHandlers,
	recorder: RecorderHandle,
	policy: UnhandledPolicy,
) {
	loop {
		let Ok((stream, _addr)) = listener.accept().await else {
			break;
		};
		let handlers = handlers.clone();
		let recorder = recorder.clone();
		let policy = policy.clone();

		tokio::spawn(async move {
			let io = TokioIo::new(stream);
			let service = service_fn(move |request| {
				handle_request(request, handlers.clone(), recorder.clone(), policy.clone())
			});
			let _ = http1::Builder::new().serve_connection(io, service).await;
		});
	}
}

async fn handle_request(
	request: HyperRequest<Incoming>,
	handlers: SharedHandlers,
	recorder: RecorderHandle,
	policy: UnhandledPolicy,
) -> Result<HyperResponse<Full<Bytes>>, NativeNetworkError> {
	let intercepted = intercepted_request(request).await;
	recorder.record(RecordedRequest {
		url: intercepted.url.clone(),
		method: intercepted.method.clone(),
		headers: intercepted.headers.clone(),
		body: intercepted.body.clone(),
		timestamp: timestamp_millis(),
	});

	let handler_result = {
		let handlers = handlers.lock();
		handlers
			.iter()
			.find(|handler| handler.matches(&intercepted))
			.map(|handler| {
				let delay = handler.delay();
				let response = handler.respond(&intercepted);
				let is_network_error = handler.is_network_error();
				(is_network_error, delay, response)
			})
	};

	match handler_result {
		Some((true, _, _)) => Err(NativeNetworkError),
		Some((false, delay, Some(response))) => {
			if let Some(duration) = delay {
				tokio::time::sleep(duration).await;
			}
			Ok(hyper_response(response))
		}
		Some((false, _, None)) => Ok(diagnostic_response(
			StatusCode::INTERNAL_SERVER_ERROR,
			format!(
				"MSW: Failed to process request for {} {}",
				intercepted.method, intercepted.url
			),
		)),
		None => {
			if matches!(policy, UnhandledPolicy::Warn) {
				eprintln!(
					"MSW: No handler for {} {}",
					intercepted.method, intercepted.url
				);
			}
			Ok(diagnostic_response(
				StatusCode::INTERNAL_SERVER_ERROR,
				format!(
					"MSW: No handler for {} {}",
					intercepted.method, intercepted.url
				),
			))
		}
	}
}

async fn intercepted_request(request: HyperRequest<Incoming>) -> InterceptedRequest {
	let method = request.method().as_str().to_string();
	let url = request
		.uri()
		.path_and_query()
		.map(|path| path.as_str().to_string())
		.unwrap_or_else(|| request.uri().path().to_string());
	let headers = request
		.headers()
		.iter()
		.filter_map(|(name, value)| {
			value
				.to_str()
				.ok()
				.map(|value| (name.as_str().to_ascii_lowercase(), value.to_string()))
		})
		.collect();
	let body = request
		.into_body()
		.collect()
		.await
		.ok()
		.map(|collected| collected.to_bytes())
		.and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

	InterceptedRequest {
		url,
		method,
		headers,
		body,
	}
}

fn hyper_response(mock: MockResponse) -> HyperResponse<Full<Bytes>> {
	let status = StatusCode::from_u16(mock.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
	let mut builder = HyperResponse::builder().status(status);
	for (name, value) in mock.headers {
		builder = builder.header(name, value);
	}
	builder
		.body(Full::new(Bytes::from(mock.body)))
		.expect("MSW response should be buildable")
}

fn diagnostic_response(status: StatusCode, body: String) -> HyperResponse<Full<Bytes>> {
	HyperResponse::builder()
		.status(status)
		.header("content-type", "text/plain; charset=utf-8")
		.body(Full::new(Bytes::from(body)))
		.expect("MSW diagnostic response should be buildable")
}

fn timestamp_millis() -> f64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or(Duration::ZERO)
		.as_secs_f64()
		* 1000.0
}

#[derive(Debug)]
struct NativeNetworkError;

impl std::fmt::Display for NativeNetworkError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "MSW simulated native network error")
	}
}

impl std::error::Error for NativeNetworkError {}
