//! Platform implementation for native (non-WASM) targets.
//!
//! Compiled only under `#[cfg(native)]` via the dispatch in
//! `crate::platform`. These stub types maintain API compatibility with
//! WASM code without requiring `web-sys` dependencies on native targets,
//! and task spawning degrades to a no-op since there is no browser event
//! loop.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;

pub use crate::component::DummyEvent as Event;

type BoxedTask = Pin<Box<dyn Future<Output = ()> + 'static>>;
type TaskSink = Box<dyn Fn(BoxedTask) + 'static>;

thread_local! {
	static TASK_SINK: RefCell<Option<TaskSink>> = const { RefCell::new(None) };
}

#[cfg(feature = "testing")]
pub(crate) struct NativeTaskSinkGuard {
	previous: Option<TaskSink>,
}

#[cfg(feature = "testing")]
impl Drop for NativeTaskSinkGuard {
	fn drop(&mut self) {
		let previous = self.previous.take();
		TASK_SINK.with(|slot| {
			*slot.borrow_mut() = previous;
		});
	}
}

#[cfg(feature = "testing")]
pub(crate) fn install_task_sink(sink: impl Fn(BoxedTask) + 'static) -> NativeTaskSinkGuard {
	let previous = TASK_SINK.with(|slot| slot.borrow_mut().replace(Box::new(sink)));
	NativeTaskSinkGuard { previous }
}

pub(crate) fn try_spawn_task<F>(fut: F) -> bool
where
	F: Future<Output = ()> + 'static,
{
	TASK_SINK.with(|slot| {
		if let Some(sink) = slot.borrow().as_ref() {
			sink(Box::pin(fut));
			true
		} else {
			false
		}
	})
}

/// Stub Window type for SSR compatibility.
#[derive(Debug, Clone, Default)]
pub struct Window;

/// Stub HtmlInputElement for SSR compatibility.
#[derive(Debug, Clone, Default)]
pub struct HtmlInputElement {
	value: String,
}

impl HtmlInputElement {
	/// Get the input value.
	pub fn value(&self) -> String {
		self.value.clone()
	}

	/// Set the input value.
	pub fn set_value(&mut self, value: &str) {
		self.value = value.to_string();
	}
}

/// Stub HtmlTextAreaElement for SSR compatibility.
#[derive(Debug, Clone, Default)]
pub struct HtmlTextAreaElement {
	value: String,
}

impl HtmlTextAreaElement {
	/// Get the textarea value.
	pub fn value(&self) -> String {
		self.value.clone()
	}

	/// Set the textarea value.
	pub fn set_value(&mut self, value: &str) {
		self.value = value.to_string();
	}
}

/// Stub HtmlSelectElement for SSR compatibility.
#[derive(Debug, Clone, Default)]
pub struct HtmlSelectElement {
	value: String,
}

impl HtmlSelectElement {
	/// Get the selected value.
	pub fn value(&self) -> String {
		self.value.clone()
	}

	/// Set the selected value.
	pub fn set_value(&mut self, value: &str) {
		self.value = value.to_string();
	}
}

/// Stub HtmlFormElement for SSR compatibility.
#[derive(Debug, Clone, Default)]
pub struct HtmlFormElement;

/// Stub HtmlButtonElement for SSR compatibility.
#[derive(Debug, Clone, Default)]
pub struct HtmlButtonElement;

/// No-op task spawn for native targets.
///
/// On native (SSR) targets there is no browser event loop, so the future
/// is dropped. Keeping `spawn_task` cross-target lets `form!`-generated
/// submission handlers and reactive hooks compile identically on both
/// targets.
pub fn spawn_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	let _ = try_spawn_task(fut);
}

/// No-op microtask yield for native targets.
pub async fn defer_yield() {}
