//! Reinhardt Admin Panel UI (WASM)
//!
//! This crate provides a WASM-based client-side rendered (CSR) frontend
//! for the Reinhardt admin panel using dominator framework.

mod api;
mod components;
mod router;
mod state;

use dominator::{Dom, append_dom, body, clone, html};
use futures_signals::signal::SignalExt;
use router::{Route, Router};
use state::AppState;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

/// WASM entry point
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	// Set panic hook for better error messages in console
	console_error_panic_hook::set_once();

	// Initialize logging (optional)
	tracing_wasm::set_as_global_default();

	// Create application state
	let api_base_url = option_env!("API_BASE_URL")
		.unwrap_or("/admin/api")
		.to_string();
	let state = AppState::new(api_base_url);

	// Load dashboard data
	state.load_dashboard();

	// Create router
	let router = Arc::new(Router::new());
	router.start_listening();

	// Render application
	let root_dom = render_app(state, router);

	// Mount to DOM
	append_dom(&body(), root_dom);

	Ok(())
}

/// Render the root application component
fn render_app(state: Arc<AppState>, router: Arc<Router>) -> Dom {
	let route_signal = router.current_route();
	html!("div", {
		.attr("id", "app")
		.child_signal(route_signal.map(clone!(state => move |route| {
			Some(render_route(&state, &route))
		})))
	})
}

/// Render a specific route
fn render_route(state: &Arc<AppState>, route: &Route) -> Dom {
	match route {
		Route::Dashboard => components::render_dashboard(Arc::clone(state)),
		Route::ModelList { model_name } => {
			components::render_list_view(Arc::clone(state), model_name.clone())
		}
		Route::ModelDetail { model_name, id } => {
			components::render_detail_view(Arc::clone(state), model_name.clone(), id.clone())
		}
		Route::ModelCreate { model_name } => {
			components::render_form(Arc::clone(state), model_name.clone(), None)
		}
		Route::ModelEdit { model_name, id } => {
			components::render_form(Arc::clone(state), model_name.clone(), Some(id.clone()))
		}
		Route::NotFound => {
			html!("div", {
				.class("not-found")
				.text("404 - Page Not Found")
			})
		}
	}
}
