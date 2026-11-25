//! REST handler builder helpers.

use std::time::Duration;

use super::handler::{InterceptedRequest, Method, RequestRules, RestHandler};
use super::matcher::UrlMatcher;
use super::response::MockResponse;

/// Create a GET handler builder.
pub fn get(pattern: impl Into<UrlMatcher>) -> RestHandlerBuilder {
	RestHandlerBuilder::new(Method::Get, pattern.into())
}

/// Create a POST handler builder.
pub fn post(pattern: impl Into<UrlMatcher>) -> RestHandlerBuilder {
	RestHandlerBuilder::new(Method::Post, pattern.into())
}

/// Create a PUT handler builder.
pub fn put(pattern: impl Into<UrlMatcher>) -> RestHandlerBuilder {
	RestHandlerBuilder::new(Method::Put, pattern.into())
}

/// Create a DELETE handler builder.
pub fn delete(pattern: impl Into<UrlMatcher>) -> RestHandlerBuilder {
	RestHandlerBuilder::new(Method::Delete, pattern.into())
}

/// Create a PATCH handler builder.
pub fn patch(pattern: impl Into<UrlMatcher>) -> RestHandlerBuilder {
	RestHandlerBuilder::new(Method::Patch, pattern.into())
}

/// Builder for configuring a REST handler.
pub struct RestHandlerBuilder {
	method: Method,
	matcher: UrlMatcher,
	once: bool,
	delay: Option<Duration>,
	rules: RequestRules,
}

impl RestHandlerBuilder {
	fn new(method: Method, matcher: UrlMatcher) -> Self {
		Self {
			method,
			matcher,
			once: false,
			delay: None,
			rules: RequestRules::new(),
		}
	}

	/// Make this handler respond only once, then be consumed.
	pub fn once(mut self) -> Self {
		self.once = true;
		self
	}

	/// Add a delay before responding.
	pub fn delay(mut self, duration: Duration) -> Self {
		self.delay = Some(duration);
		self
	}

	/// Match only requests that carry this header value. Header names are
	/// compared case-insensitively.
	pub fn header(self, name: impl Into<String>, value: impl Into<String>) -> Self {
		let name = name.into().to_ascii_lowercase();
		let value = value.into();
		let condition = format!("header {name}: {value}");
		self.condition(
			condition,
			Box::new(move |request: &InterceptedRequest| {
				request
					.headers
					.get(&name)
					.is_some_and(|actual| *actual == value)
			}),
		)
	}

	/// Match only requests whose body contains this text.
	pub fn body_contains(self, needle: impl Into<String>) -> Self {
		let needle = needle.into();
		let condition = format!("body contains {needle:?}");
		self.condition(
			condition,
			Box::new(move |request: &InterceptedRequest| {
				request
					.body
					.as_deref()
					.is_some_and(|body| body.contains(&needle))
			}),
		)
	}

	/// Match only requests for which `predicate` returns `true`.
	#[cfg(native)]
	pub fn matching(
		self,
		predicate: impl Fn(&InterceptedRequest) -> bool + Send + Sync + 'static,
	) -> Self {
		self.condition("custom predicate".to_string(), Box::new(predicate))
	}

	/// Match only requests for which `predicate` returns `true`.
	#[cfg(wasm)]
	pub fn matching(self, predicate: impl Fn(&InterceptedRequest) -> bool + 'static) -> Self {
		self.condition("custom predicate".to_string(), Box::new(predicate))
	}

	/// Expect this handler to answer exactly `calls` requests. The worker checks
	/// the expectation in `verify()` and when it is dropped.
	pub fn expect(mut self, calls: usize) -> Self {
		self.rules.expected = Some(calls);
		self
	}

	fn condition(mut self, description: String, predicate: Box<super::state::PredicateFn>) -> Self {
		self.rules.predicates.push((description, predicate));
		self
	}

	/// Respond with a fixed response.
	pub fn respond(self, response: impl Into<MockResponse>) -> RestHandler {
		let response = response.into();
		RestHandler::new(
			self.method,
			self.matcher,
			Box::new(move |_| response.clone()),
			self.once,
			self.delay,
			self.rules,
		)
	}

	/// Respond with a dynamic closure.
	#[cfg(native)]
	pub fn respond_with(
		self,
		f: impl Fn(&InterceptedRequest) -> MockResponse + Send + Sync + 'static,
	) -> RestHandler {
		RestHandler::new(
			self.method,
			self.matcher,
			Box::new(f),
			self.once,
			self.delay,
			self.rules,
		)
	}

	/// Respond with a dynamic closure.
	#[cfg(wasm)]
	pub fn respond_with(
		self,
		f: impl Fn(&InterceptedRequest) -> MockResponse + 'static,
	) -> RestHandler {
		RestHandler::new(
			self.method,
			self.matcher,
			Box::new(f),
			self.once,
			self.delay,
			self.rules,
		)
	}

	/// Simulate a network error (rejected Promise with TypeError).
	pub fn network_error(self) -> RestHandler {
		RestHandler::network_error(self.method, self.matcher, self.once, self.rules)
	}
}

#[cfg(test)]
mod tests {
	use super::super::handler::ErasedHandler;
	use super::*;
	use rstest::*;
	use serde_json::json;
	use std::collections::HashMap;

	fn make_req(url: &str, method: &str) -> InterceptedRequest {
		InterceptedRequest {
			url: url.to_string(),
			method: method.to_string(),
			headers: HashMap::new(),
			body: None,
		}
	}

	#[rstest]
	fn get_builder_creates_get_handler() {
		let handler = get("/api/data").respond(MockResponse::empty());
		assert!(handler.matches(&make_req("/api/data", "GET")));
		assert!(!handler.matches(&make_req("/api/data", "POST")));
	}

	#[rstest]
	fn post_builder() {
		let handler = post("/api/data").respond(MockResponse::empty());
		assert!(handler.matches(&make_req("/api/data", "POST")));
	}

	#[rstest]
	fn respond_with_closure() {
		let handler = get("/api/data").respond_with(|_| MockResponse::json(42));
		let resp = handler.respond(&make_req("/api/data", "GET")).unwrap();
		assert_eq!(resp.body, "42");
	}

	#[rstest]
	fn once_modifier() {
		let handler = get("/api/data").once().respond(MockResponse::empty());
		assert!(handler.matches(&make_req("/api/data", "GET")));
		let _ = handler.respond(&make_req("/api/data", "GET"));
		assert!(handler.is_consumed());
		assert!(!handler.matches(&make_req("/api/data", "GET")));
	}

	#[rstest]
	fn delay_modifier() {
		let handler = get("/api/data")
			.delay(Duration::from_millis(500))
			.respond(MockResponse::empty());
		assert_eq!(handler.delay(), Some(Duration::from_millis(500)));
	}

	#[rstest]
	fn network_error_handler() {
		let handler = get("/api/fail").network_error();
		assert!(handler.matches(&make_req("/api/fail", "GET")));
		assert!(handler.is_network_error());
		assert!(handler.respond(&make_req("/api/fail", "GET")).is_none());
	}

	#[rstest]
	fn from_json_value() {
		let handler = get("/api/data").respond(json!({"ok": true}));
		let resp = handler.respond(&make_req("/api/data", "GET")).unwrap();
		assert_eq!(resp.body, r#"{"ok":true}"#);
	}
}
