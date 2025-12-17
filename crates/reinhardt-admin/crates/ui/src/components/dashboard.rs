//! Dashboard component

use crate::state::AppState;
use dominator::{Dom, clone, events, html};
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::SignalVecExt;
use reinhardt_admin_types::ModelInfo;
use std::sync::Arc;

/// Render the dashboard view
pub fn render(state: Arc<AppState>) -> Dom {
	html!("div", {
		.class("dashboard")
		.children(&mut [
			// Header
			html!("h1", {
				.text_signal(state.site_name.signal_cloned())
			}),

			// Error message
			html!("div", {
				.class("error-message")
				.visible_signal(state.error.signal_cloned().map(|e| e.is_some()))
				.child_signal(state.error.signal_cloned().map(|error| {
					error.map(|msg| {
						html!("div", {
							.class("error")
							.text(&msg)
						})
					})
				}))
			}),

			// Loading indicator
			html!("div", {
				.visible_signal(state.is_loading.signal())
				.text("Loading...")
			}),

			// Models grid
			html!("div", {
				.class("models-grid")
				.children_signal_vec(state.models.signal_vec_cloned().map(move |model| {
					render_model_card(model)
				}))
			})
		])
	})
}

/// Render a single model card
fn render_model_card(model: ModelInfo) -> Dom {
	html!("div", {
		.class("model-card")
		.event(clone!(model => move |_: events::Click| {
			let window = web_sys::window().unwrap();
			let location = window.location();
			location.set_hash(&format!("#/{}", model.name)).unwrap();
		}))
		.children(&mut [
			html!("h2", {
				.text(&model.name)
			}),
			html!("p", {
				.text(&format!("View: {}", model.list_url))
			})
		])
	})
}
