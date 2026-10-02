use reinhardt::urls::prelude::UnifiedRouter;
use std::cell::RefCell;

thread_local! {
	static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn record(event: &'static str) {
	EVENTS.with(|events| events.borrow_mut().push(event));
}

pub(crate) struct TraceGuard;

impl TraceGuard {
	pub(crate) fn new() -> Self {
		EVENTS.with(|events| events.borrow_mut().clear());
		Self
	}

	pub(crate) fn take(&self) -> Vec<&'static str> {
		EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
	}
}

impl Drop for TraceGuard {
	fn drop(&mut self) {
		EVENTS.with(|events| events.borrow_mut().clear());
	}
}

#[cfg(all(server, not(all(target_family = "wasm", target_os = "unknown"))))]
struct Captured(String);

#[cfg(all(server, not(all(target_family = "wasm", target_os = "unknown"))))]
impl Drop for Captured {
	fn drop(&mut self) {
		record("drop-capture");
	}
}

struct TemporaryName(String);

impl TemporaryName {
	fn new() -> Self {
		record("namespace");
		Self(String::from("trace"))
	}

	fn as_str(&self) -> &str {
		&self.0
	}
}

impl Drop for TemporaryName {
	fn drop(&mut self) {
		record("drop-borrow");
	}
}

#[reinhardt::url_patterns]
pub(crate) fn traced_urls() -> UnifiedRouter {
	UnifiedRouter::new()
		.with_prefix({
			record("prefix");
			"/api/"
		})
		.server({
			record("server-argument-1");
			let captured = Captured(String::from("native"));
			move |server| {
				record("server-body-1");
				assert_eq!(captured.0, "native");
				drop(captured);
				server
			}
		})
		.with_namespace(TemporaryName::new().as_str())
		.merge({
			record("merge-argument");
			UnifiedRouter::new()
		})
		.server({
			record("server-argument-2");
			|server| {
				record("server-body-2");
				server
			}
		})
}

#[cfg(all(server, not(all(target_family = "wasm", target_os = "unknown"))))]
pub(crate) fn handwritten_traced_urls() -> UnifiedRouter {
	UnifiedRouter::new()
		.with_prefix({
			record("prefix");
			"/api/"
		})
		.server({
			record("server-argument-1");
			let captured = Captured(String::from("native"));
			move |server| {
				record("server-body-1");
				assert_eq!(captured.0, "native");
				drop(captured);
				server
			}
		})
		.with_namespace(TemporaryName::new().as_str())
		.merge({
			record("merge-argument");
			UnifiedRouter::new()
		})
		.server({
			record("server-argument-2");
			|server| {
				record("server-body-2");
				server
			}
		})
}
