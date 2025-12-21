//! Client-side routing
//!
//! This module handles client-side routing using the browser's History API.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, Event, Window};

/// Route definition
pub enum AppRoute {
	Home,
	About,
	NotFound,
}

impl AppRoute {
	/// Get the path for this route
	pub fn path(&self) -> &'static str {
		match self {
			AppRoute::Home => "/",
			AppRoute::About => "/about",
			AppRoute::NotFound => "/404",
		}
	}

	/// Parse a path into a route
	pub fn from_path(path: &str) -> Self {
		match path {
			"/" => AppRoute::Home,
			"/about" => AppRoute::About,
			_ => AppRoute::NotFound,
		}
	}

	/// Render this route
	pub fn render(&self, container: &Element) {
		match self {
			AppRoute::Home => {
				container.set_inner_html("<div class=\"container mt-5\"><h1>Welcome to {{ project_name }}!</h1><p class=\"lead\">A modern WASM-based application built with Reinhardt Pages.</p></div>");
			}
			AppRoute::About => {
				container.set_inner_html("<div class=\"container mt-5\"><h1>About {{ project_name }}</h1><p>This is a Reinhardt Pages application.</p></div>");
			}
			AppRoute::NotFound => {
				container.set_inner_html("<div class=\"container mt-5\"><h1>404 - Page Not Found</h1></div>");
			}
		}
	}
}

/// Global router instance
thread_local! {
	static ROUTER: RefCell<Option<Router>> = RefCell::new(None);
}

/// Initialize the global router
pub fn init_global_router() {
	ROUTER.with(|r| {
		*r.borrow_mut() = Some(Router::new());
	});
}

/// Access the global router
pub fn with_router<F, R>(f: F) -> R
where
	F: FnOnce(&Router) -> R,
{
	ROUTER.with(|r| {
		let router = r.borrow();
		f(router.as_ref().expect("Router not initialized"))
	})
}

/// Router implementation
pub struct Router {
	window: Window,
	root: Option<Element>,
}

impl Router {
	/// Create a new router
	fn new() -> Self {
		let window = web_sys::window().expect("no global `window` exists");

		// Set up popstate listener
		let closure = Closure::<dyn Fn(Event)>::new(move |_event: Event| {
			with_router(|router| {
				router.navigate_to_current_url();
			});
		});

		window
			.add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref())
			.expect("Failed to add popstate listener");

		// Leak the closure to keep it alive
		closure.forget();

		Router { window, root: None }
	}

	/// Mount the router to a DOM element
	pub fn mount(&mut self, element: &Element) {
		self.root = Some(element.clone());
		self.navigate_to_current_url();
	}

	/// Navigate to the current URL
	fn navigate_to_current_url(&self) {
		if let Some(root) = &self.root {
			let location = self.window.location();
			let pathname = location.pathname().unwrap_or_else(|_| "/".to_string());
			let route = AppRoute::from_path(&pathname);
			route.render(root);
		}
	}

	/// Navigate to a new route
	pub fn navigate(&self, route: AppRoute) {
		let path = route.path();
		let history = self.window.history().expect("Failed to get history");
		history
			.push_state_with_url(&JsValue::NULL, "", Some(path))
			.expect("Failed to push state");

		if let Some(root) = &self.root {
			route.render(root);
		}
	}
}
