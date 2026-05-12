//! Native (non-WASM) platform stubs.
//!
//! Provides type stubs that preserve API compatibility with WASM code
//! without requiring `web_sys` dependencies on native targets, plus a
//! no-op `spawn_task` so cross-target component code compiles unmodified.

use std::future::Future;

// --- DOM type stubs -------------------------------------------------------

pub use crate::component::DummyEvent as Event;

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

// --- Task spawning (no-op) ------------------------------------------------

/// No-op task spawner for native targets.
///
/// Mirrors the WASM `spawn_task` signature so cross-target component code
/// compiles unmodified. The future is dropped; native runtime integration
/// belongs to the caller (e.g. a tokio context).
pub fn spawn_task<F>(_fut: F)
where
	F: Future<Output = ()> + 'static,
{
	// Native: drop the future without execution.
}

/// No-op yield for native targets. Returns immediately.
pub async fn defer_yield() {}
