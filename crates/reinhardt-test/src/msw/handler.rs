//! Handler types for intercepting and responding to requests.

use std::collections::HashMap;
use std::time::Duration;

use super::matcher::{UrlMatcher, extract_path};
use super::response::MockResponse;
use super::state::{HandlerThreadSafety, HitCounter, OnceFlag, PredicateFn, ResponseFn};

/// An intercepted HTTP request extracted from JS.
#[derive(Debug, Clone)]
pub struct InterceptedRequest {
	/// The request URL.
	pub url: String,
	/// The HTTP method.
	pub method: String,
	/// Request headers.
	pub headers: HashMap<String, String>,
	/// Request body, if present.
	pub body: Option<String>,
}

impl InterceptedRequest {
	/// Parses the request body as JSON.
	pub fn body_json<T: serde::de::DeserializeOwned>(&self) -> serde_json::Result<T> {
		serde_json::from_str(self.body.as_deref().unwrap_or_default())
	}
}

/// HTTP method for handler matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
	Get,
	Post,
	Put,
	Delete,
	Patch,
}

impl Method {
	pub(crate) fn as_str(&self) -> &'static str {
		match self {
			Method::Get => "GET",
			Method::Post => "POST",
			Method::Put => "PUT",
			Method::Delete => "DELETE",
			Method::Patch => "PATCH",
		}
	}
}

/// Type-erased handler interface for heterogeneous storage.
pub(crate) trait ErasedHandler: HandlerThreadSafety {
	fn matches(&self, req: &InterceptedRequest) -> bool;
	/// Build the response. Does NOT apply delay (caller handles that).
	fn respond(&self, req: &InterceptedRequest) -> Option<MockResponse>;
	fn is_consumed(&self) -> bool;
	/// Returns the delay duration if configured.
	fn delay(&self) -> Option<Duration>;
	/// Returns true if this is a network error handler.
	fn is_network_error(&self) -> bool;
	/// Describes a call expectation this handler has not met, if any.
	fn unmet_expectation(&self) -> Option<String> {
		None
	}
}

/// Request conditions beyond method and URL, plus a call expectation.
pub(crate) struct RequestRules {
	pub(crate) predicates: Vec<(String, Box<PredicateFn>)>,
	pub(crate) expected: Option<usize>,
}

impl RequestRules {
	pub(crate) fn new() -> Self {
		Self {
			predicates: Vec::new(),
			expected: None,
		}
	}
}

/// Handler for REST endpoints.
pub struct RestHandler {
	method: Method,
	matcher: UrlMatcher,
	response_fn: Option<Box<ResponseFn>>,
	once: bool,
	consumed: OnceFlag,
	delay: Option<Duration>,
	network_error: bool,
	rules: RequestRules,
	hits: HitCounter,
}

impl RestHandler {
	pub(crate) fn new(
		method: Method,
		matcher: UrlMatcher,
		response_fn: Box<ResponseFn>,
		once: bool,
		delay: Option<Duration>,
		rules: RequestRules,
	) -> Self {
		Self {
			method,
			matcher,
			response_fn: Some(response_fn),
			once,
			consumed: OnceFlag::new(false),
			delay,
			network_error: false,
			rules,
			hits: HitCounter::new(),
		}
	}

	pub(crate) fn network_error(
		method: Method,
		matcher: UrlMatcher,
		once: bool,
		rules: RequestRules,
	) -> Self {
		Self {
			method,
			matcher,
			response_fn: None,
			once,
			consumed: OnceFlag::new(false),
			delay: None,
			network_error: true,
			rules,
			hits: HitCounter::new(),
		}
	}

	fn describe(&self) -> String {
		let mut description = format!("{} {:?}", self.method.as_str(), self.matcher);
		for (condition, _) in &self.rules.predicates {
			description.push_str(" and ");
			description.push_str(condition);
		}
		description
	}
}

impl ErasedHandler for RestHandler {
	fn matches(&self, req: &InterceptedRequest) -> bool {
		if self.consumed.get() {
			return false;
		}
		req.method == self.method.as_str()
			&& self.matcher.matches(&req.url)
			&& self
				.rules
				.predicates
				.iter()
				.all(|(_, predicate)| predicate(req))
	}

	fn respond(&self, req: &InterceptedRequest) -> Option<MockResponse> {
		if self.once {
			self.consumed.set(true);
		}
		self.hits.increment();
		self.response_fn.as_ref().map(|f| f(req))
	}

	fn is_consumed(&self) -> bool {
		self.consumed.get()
	}

	fn delay(&self) -> Option<Duration> {
		self.delay
	}

	fn is_network_error(&self) -> bool {
		self.network_error
	}

	fn unmet_expectation(&self) -> Option<String> {
		let expected = self.rules.expected?;
		let received = self.hits.get();
		(received != expected).then(|| {
			format!(
				"{}: expected {expected} call(s), received {received}",
				self.describe()
			)
		})
	}
}

use std::marker::PhantomData;

use reinhardt_pages::server_fn::MockableServerFn;
use reinhardt_pages::server_fn::ServerFnError;

use super::context::TestContext;

#[cfg(native)]
type ServerFnResponseFn<S> = dyn Fn(<S as MockableServerFn>::Args) -> Result<<S as MockableServerFn>::Response, ServerFnError>
	+ Send
	+ Sync;

#[cfg(wasm)]
type ServerFnResponseFn<S> = dyn Fn(
	<S as MockableServerFn>::Args,
) -> Result<<S as MockableServerFn>::Response, ServerFnError>;

#[cfg(native)]
type ServerFnContextResponseFn<S> = dyn Fn(
		<S as MockableServerFn>::Args,
		&TestContext,
	) -> Result<<S as MockableServerFn>::Response, ServerFnError>
	+ Send
	+ Sync;

#[cfg(wasm)]
type ServerFnContextResponseFn<S> =
	dyn Fn(
		<S as MockableServerFn>::Args,
		&TestContext,
	) -> Result<<S as MockableServerFn>::Response, ServerFnError>;

/// Type-safe handler for server functions.
pub(crate) struct ServerFnHandler<S: MockableServerFn> {
	response_fn: Box<ServerFnResponseFn<S>>,
	once: bool,
	consumed: OnceFlag,
	delay: Option<Duration>,
	_marker: PhantomData<fn() -> S>,
}

impl<S: MockableServerFn> ServerFnHandler<S> {
	pub(crate) fn new(
		response_fn: Box<ServerFnResponseFn<S>>,
		once: bool,
		delay: Option<Duration>,
	) -> Self {
		Self {
			response_fn,
			once,
			consumed: OnceFlag::new(false),
			delay,
			_marker: PhantomData,
		}
	}
}

impl<S: MockableServerFn> ErasedHandler for ServerFnHandler<S> {
	fn matches(&self, req: &InterceptedRequest) -> bool {
		if self.consumed.get() {
			return false;
		}
		req.method == "POST" && extract_path(&req.url) == S::PATH
	}

	fn respond(&self, req: &InterceptedRequest) -> Option<MockResponse> {
		if self.once {
			self.consumed.set(true);
		}
		let body = req.body.as_deref().unwrap_or("{}");
		let args: S::Args = serde_json::from_str(body).ok()?;
		let result = (self.response_fn)(args);
		match result {
			Ok(response) => {
				let body = serde_json::to_string(&response).ok()?;
				Some(MockResponse {
					status: 200,
					headers: {
						let mut h = HashMap::new();
						h.insert("content-type".to_string(), "application/json".to_string());
						h
					},
					body,
				})
			}
			Err(err) => Some(server_fn_error_response(err)),
		}
	}

	fn is_consumed(&self) -> bool {
		self.consumed.get()
	}

	fn delay(&self) -> Option<Duration> {
		self.delay
	}

	fn is_network_error(&self) -> bool {
		false
	}
}

/// Type-safe handler for server functions with DI test context.
pub(crate) struct ServerFnContextHandler<S: MockableServerFn> {
	response_fn: Box<ServerFnContextResponseFn<S>>,
	context: TestContext,
	once: bool,
	consumed: OnceFlag,
	delay: Option<Duration>,
	_marker: PhantomData<fn() -> S>,
}

impl<S: MockableServerFn> ServerFnContextHandler<S> {
	pub(crate) fn new(
		context: TestContext,
		response_fn: Box<ServerFnContextResponseFn<S>>,
		once: bool,
		delay: Option<Duration>,
	) -> Self {
		Self {
			response_fn,
			context,
			once,
			consumed: OnceFlag::new(false),
			delay,
			_marker: PhantomData,
		}
	}
}

impl<S: MockableServerFn> ErasedHandler for ServerFnContextHandler<S> {
	fn matches(&self, req: &InterceptedRequest) -> bool {
		if self.consumed.get() {
			return false;
		}
		req.method == "POST" && extract_path(&req.url) == S::PATH
	}

	fn respond(&self, req: &InterceptedRequest) -> Option<MockResponse> {
		if self.once {
			self.consumed.set(true);
		}
		let body = req.body.as_deref().unwrap_or("{}");
		let args: S::Args = serde_json::from_str(body).ok()?;
		let result = (self.response_fn)(args, &self.context);
		match result {
			Ok(response) => {
				let body = serde_json::to_string(&response).ok()?;
				Some(MockResponse {
					status: 200,
					headers: {
						let mut h = HashMap::new();
						h.insert("content-type".to_string(), "application/json".to_string());
						h
					},
					body,
				})
			}
			Err(err) => Some(server_fn_error_response(err)),
		}
	}

	fn is_consumed(&self) -> bool {
		self.consumed.get()
	}

	fn delay(&self) -> Option<Duration> {
		self.delay
	}

	fn is_network_error(&self) -> bool {
		false
	}
}

fn server_fn_error_response(err: ServerFnError) -> MockResponse {
	let status = err.status().unwrap_or(500);
	let body = serde_json::to_string(&err)
		.expect("ServerFnError must serialize into its versioned error envelope");
	MockResponse {
		status,
		headers: {
			let mut h = HashMap::new();
			h.insert("content-type".to_string(), "application/json".to_string());
			h
		},
		body,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	fn make_intercepted(url: &str, method: &str) -> InterceptedRequest {
		InterceptedRequest {
			url: url.to_string(),
			method: method.to_string(),
			headers: HashMap::new(),
			body: None,
		}
	}

	#[rstest]
	fn rest_handler_matches_method_and_url() {
		let handler = RestHandler::new(
			Method::Get,
			"/api/users".into(),
			Box::new(|_| MockResponse::empty()),
			false,
			None,
			RequestRules::new(),
		);
		assert!(handler.matches(&make_intercepted("/api/users", "GET")));
		assert!(!handler.matches(&make_intercepted("/api/users", "POST")));
		assert!(!handler.matches(&make_intercepted("/api/posts", "GET")));
	}

	#[rstest]
	fn rest_handler_once_is_consumed_after_respond() {
		let handler = RestHandler::new(
			Method::Get,
			"/api/users".into(),
			Box::new(|_| MockResponse::empty()),
			true,
			None,
			RequestRules::new(),
		);
		assert!(!handler.is_consumed());
		let _ = handler.respond(&make_intercepted("/api/users", "GET"));
		assert!(handler.is_consumed());
	}

	#[rstest]
	fn rest_handler_reusable_not_consumed() {
		let handler = RestHandler::new(
			Method::Get,
			"/api/users".into(),
			Box::new(|_| MockResponse::empty()),
			false,
			None,
			RequestRules::new(),
		);
		let _ = handler.respond(&make_intercepted("/api/users", "GET"));
		let _ = handler.respond(&make_intercepted("/api/users", "GET"));
		assert!(!handler.is_consumed());
	}

	#[rstest]
	fn rest_handler_delay() {
		let handler = RestHandler::new(
			Method::Get,
			"/api/users".into(),
			Box::new(|_| MockResponse::empty()),
			false,
			Some(Duration::from_millis(100)),
			RequestRules::new(),
		);
		assert_eq!(handler.delay(), Some(Duration::from_millis(100)));
	}

	#[rstest]
	fn network_error_handler() {
		let handler =
			RestHandler::network_error(Method::Get, "/api/fail".into(), false, RequestRules::new());
		assert!(handler.matches(&make_intercepted("/api/fail", "GET")));
		assert!(handler.is_network_error());
		assert!(
			handler
				.respond(&make_intercepted("/api/fail", "GET"))
				.is_none()
		);
	}

	#[rstest]
	fn server_fn_error_response_uses_the_versioned_json_envelope() {
		// Arrange
		let error = ServerFnError::server(503, "Mock unavailable");

		// Act
		let response = server_fn_error_response(error);
		let decoded: ServerFnError =
			serde_json::from_str(&response.body).expect("response must contain a ServerFnError");

		// Assert
		assert_eq!(response.status, 503);
		assert_eq!(
			response.headers.get("content-type"),
			Some(&"application/json".to_string())
		);
		assert_eq!(
			decoded.kind(),
			reinhardt_pages::server_fn::ServerFnErrorKind::Server
		);
		assert_eq!(decoded.status(), Some(503));
		assert_eq!(decoded.user_message(), "Mock unavailable");
	}
}
