//! Browser-WASM platform types and task spawning.
//!
//! Backed by `web_sys` / `wasm_bindgen_futures`. See [`super`] for the
//! cross-target dispatch point.

use std::future::Future;

// --- DOM type aliases -----------------------------------------------------

/// DOM Event type (`web_sys::Event` on WASM).
pub type Event = web_sys::Event;

/// Window object type (`web_sys::Window` on WASM).
pub type Window = web_sys::Window;

/// HTML input element type (`web_sys::HtmlInputElement` on WASM).
pub type HtmlInputElement = web_sys::HtmlInputElement;

/// HTML textarea element type (`web_sys::HtmlTextAreaElement` on WASM).
pub type HtmlTextAreaElement = web_sys::HtmlTextAreaElement;

/// HTML select element type (`web_sys::HtmlSelectElement` on WASM).
pub type HtmlSelectElement = web_sys::HtmlSelectElement;

/// HTML form element type (`web_sys::HtmlFormElement` on WASM).
pub type HtmlFormElement = web_sys::HtmlFormElement;

/// HTML button element type (`web_sys::HtmlButtonElement` on WASM).
pub type HtmlButtonElement = web_sys::HtmlButtonElement;

// --- Task spawning --------------------------------------------------------

/// Spawn a task on the browser's event loop.
///
/// Uses `wasm_bindgen_futures::spawn_local` to schedule the future. The
/// native counterpart is a no-op stub, so this function is cross-target
/// safe for use in component code that does not care about target.
pub fn spawn_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut);
}

/// Yield to the event loop by queuing a microtask.
///
/// Resolves a `JsFuture` wrapping `Promise.resolve()`, giving the browser
/// event loop a chance to tick. This is necessary when spawning async work
/// during initialization (e.g. inside `main()`), where `JsFuture`s from
/// fetch would otherwise hang.
pub async fn defer_yield() {
	use wasm_bindgen::JsValue;
	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
