//! Platform implementation for browser-WASM targets.
//!
//! Compiled only under `#[cfg(wasm)]` via the dispatch in
//! `crate::platform`. The items here back the cross-target API
//! re-exported from `crate::platform` and `crate::prelude`, mapping each
//! abstract type onto its `web_sys` counterpart and implementing task
//! spawning on top of `wasm_bindgen_futures`.

use std::future::Future;

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

/// Spawns a task on the current runtime.
///
/// On WASM, this uses `wasm_bindgen_futures::spawn_local` to schedule
/// the task on the browser's event loop.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::prelude::spawn_task;
///
/// spawn_task(async move {
///     let data = fetch_data().await;
///     process(data);
/// });
/// ```
pub fn spawn_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut);
}

/// Yields to the event loop by queuing a microtask.
///
/// On WASM, this resolves a `JsFuture` wrapping a `Promise.resolve()`,
/// giving the browser event loop a chance to tick. This is necessary when
/// spawning async work during initialization (e.g. inside `main()`),
/// where `JsFuture`s from fetch would otherwise hang.
pub async fn defer_yield() {
	use wasm_bindgen::JsValue;
	let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
	let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
