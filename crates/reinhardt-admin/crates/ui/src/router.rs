//! Client-side routing using hash-based navigation

use futures_signals::signal::Mutable;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::HashChangeEvent;

/// Application routes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
	/// Dashboard (homepage)
	Dashboard,
	/// Model list view
	ModelList { model_name: String },
	/// Model detail view
	ModelDetail { model_name: String, id: String },
	/// Model create view
	ModelCreate { model_name: String },
	/// Model edit view
	ModelEdit { model_name: String, id: String },
	/// Not found (404)
	NotFound,
}

/// Router for managing client-side navigation
pub struct Router {
	current_route: Mutable<Route>,
}

impl Router {
	/// Create a new router
	pub fn new() -> Self {
		let initial_route = Self::parse_hash(&Self::get_current_hash());

		Self {
			current_route: Mutable::new(initial_route),
		}
	}

	/// Get the current hash from the window location
	fn get_current_hash() -> String {
		web_sys::window()
			.and_then(|w| w.location().hash().ok())
			.unwrap_or_else(|| "#/".to_string())
	}

	/// Parse a hash string into a Route
	fn parse_hash(hash: &str) -> Route {
		let hash = hash.trim_start_matches('#');
		let path = hash.trim_start_matches('/');

		if path.is_empty() || path == "dashboard" {
			return Route::Dashboard;
		}

		let segments: Vec<&str> = path.split('/').collect();

		match segments.as_slice() {
			[""] => Route::Dashboard,
			[model_name] if !model_name.is_empty() => Route::ModelList {
				model_name: model_name.to_string(),
			},
			[model_name, "create"] => Route::ModelCreate {
				model_name: model_name.to_string(),
			},
			[model_name, id] if !id.is_empty() => Route::ModelDetail {
				model_name: model_name.to_string(),
				id: id.to_string(),
			},
			[model_name, id, "edit"] => Route::ModelEdit {
				model_name: model_name.to_string(),
				id: id.to_string(),
			},
			_ => Route::NotFound,
		}
	}

	/// Get a signal for the current route
	pub fn current_route(&self) -> impl futures_signals::signal::Signal<Item = Route> + use<> {
		self.current_route.signal_cloned()
	}

	/// Start listening to hash change events
	pub fn start_listening(&self) {
		let route = self.current_route.clone();

		let closure = Closure::wrap(Box::new(move |event: HashChangeEvent| {
			let new_hash = event.new_url();
			let hash_part = new_hash
				.split('#')
				.nth(1)
				.map(|h| format!("#{}", h))
				.unwrap_or_else(|| "#/".to_string());

			let new_route = Router::parse_hash(&hash_part);
			route.set(new_route);
		}) as Box<dyn FnMut(_)>);

		if let Some(window) = web_sys::window() {
			window
				.add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref())
				.expect("Failed to add hashchange event listener");
		}

		// Keep the closure alive
		closure.forget();

		// Set initial route
		let initial_route = Self::parse_hash(&Self::get_current_hash());
		self.current_route.set(initial_route);
	}
}

impl Default for Router {
	fn default() -> Self {
		Self::new()
	}
}
