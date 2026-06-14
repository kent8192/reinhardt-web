//! WASM entry point for Reinhardt Admin Panel

use crate::pages::router;
use reinhardt_pages::component::PageExt;
use reinhardt_pages::{Element, cleanup_reactive_nodes};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, HtmlElement, window};

fn render_current_route(app_element: &web_sys::Element) -> Result<(), JsValue> {
	cleanup_reactive_nodes();
	let view = router::with_router(|r| r.render_current());
	app_element.set_inner_html("");
	let wrapper = Element::new(app_element.clone());
	view.mount(&wrapper)
		.map_err(|e| JsValue::from_str(&format!("Mount failed: {:?}", e)))
}

/// WASM entry point
///
/// This function is called when the WASM module is loaded.
/// It initializes the application and mounts it to the DOM.
#[allow(clippy::main_recursion)]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	// Set up panic hook for better error messages in console
	#[cfg(feature = "console_error_panic_hook")]
	console_error_panic_hook::set_once();

	// Get DOM elements
	let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
	let document = window
		.document()
		.ok_or_else(|| JsValue::from_str("No document object"))?;
	let app_element = document
		.get_element_by_id("app")
		.ok_or_else(|| JsValue::from_str("No #app element found"))?;

	// Initialize reactive scheduler for WASM (required for Effect re-execution
	// when signals change from async contexts like spawn_task).
	reinhardt_pages::reactive::runtime::set_scheduler(|task| {
		wasm_bindgen_futures::spawn_local(async move { task() });
	});

	// Initialize global router
	router::init_global_router();

	// Initial render — mount to DOM so event handlers (form submit, etc.) are attached.
	render_current_route(&app_element)?;

	// Re-render only on actual router navigation. Tracking router signals in a
	// reactive effect remounts the whole app during form input updates and clears
	// uncontrolled field values.
	let app_clone = app_element.clone();
	let render_subscription = router::with_router(|r| {
		r.on_navigate(move |_path, _params| {
			if let Err(e) = render_current_route(&app_clone) {
				web_sys::console::error_1(&format!("Re-mount failed: {:?}", e).into());
			}
		})
	});
	// WASM entry points run for the whole application lifetime, so this
	// navigation subscription must stay alive for the same duration.
	std::mem::forget(render_subscription);

	// Set up navigation event listeners

	// 1. Link click handler (event delegation)
	// Intercept all link clicks and use client-side routing
	let link_handler = Closure::wrap(Box::new(move |event: Event| {
		// Check if the clicked element is a link
		if let Some(target) = event.target()
			&& let Ok(element) = target.dyn_into::<HtmlElement>()
		{
			// Find the closest <a> tag (in case user clicked inside the link)
			let mut current = Some(element);
			while let Some(el) = current {
				if el.tag_name().to_lowercase() == "a" {
					// Get href attribute
					if let Some(href) = el.get_attribute("href") {
						// Only handle internal links (starting with /)
						if href.starts_with('/') {
							// Prevent default browser navigation
							event.prevent_default();

							// Navigate using router
							router::with_router(|r| {
								let _ = r.push(&href);
							});
							return;
						}
					}
					break;
				}
				// Move up to parent
				current = el
					.parent_element()
					.and_then(|p| p.dyn_into::<HtmlElement>().ok());
			}
		}
	}) as Box<dyn FnMut(_)>);

	document.add_event_listener_with_callback("click", link_handler.as_ref().unchecked_ref())?;
	link_handler.forget();

	// 2. Browser back/forward button handler (popstate)
	let popstate_handler = Closure::wrap(Box::new(move |_event: Event| {
		// Re-render when user navigates with browser buttons
		// Use replace() since browser has already changed the history
		router::with_router(|r| {
			let current_path = web_sys::window()
				.and_then(|w| {
					let location = w.location();
					let pathname = location.pathname().ok()?;
					let search = location.search().ok().unwrap_or_default();
					Some(format!("{}{}", pathname, search))
				})
				.unwrap_or_else(|| "/".to_string());
			let _ = r.replace(&current_path);
		});
	}) as Box<dyn FnMut(_)>);

	window
		.add_event_listener_with_callback("popstate", popstate_handler.as_ref().unchecked_ref())?;
	popstate_handler.forget();

	Ok(())
}
