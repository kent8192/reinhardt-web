//! Detail view component for displaying a model instance

use crate::state::{AppState, DetailViewState};
use dominator::{Dom, clone, events, html};
use futures_signals::signal::SignalExt;
use std::sync::Arc;

/// Render the detail view for a specific model instance
pub fn render(state: Arc<AppState>, model_name: String, id: String) -> Dom {
	// Get or create detail view state
	let detail_state = state.get_detail_view_state(&model_name, &id);

	// Load data on initial render
	detail_state.load_data();

	html!("div", {
		.class("detail-view")
		.children(&mut [
			render_header(&detail_state, &model_name, &id),
			render_loading(&detail_state),
			render_error(&detail_state),
			render_data(&detail_state),
			render_actions(&detail_state, &model_name, &id),
		])
	})
}

/// Render the header with title and Back button
fn render_header(_state: &Arc<DetailViewState>, model_name: &str, _id: &str) -> Dom {
	html!("div", {
		.class("detail-view-header")
		.children(&mut [
			html!("button", {
				.class("btn btn-secondary")
				.text("← Back")
				.event(|_: events::Click| {
					// Navigate back to list view
					let window = web_sys::window().unwrap();
					let history = window.history().unwrap();
					history.back().unwrap();
				})
			}),
			html!("h1", {
				.class("detail-view-title")
				.text(&format!("{} Detail", model_name))
			}),
		])
	})
}

/// Render loading indicator
fn render_loading(state: &Arc<DetailViewState>) -> Dom {
	html!("div", {
		.class("loading")
		.visible_signal(state.is_loading.signal())
		.text("Loading...")
	})
}

/// Render error message
fn render_error(state: &Arc<DetailViewState>) -> Dom {
	html!("div", {
		.class("error-message")
		.visible_signal(state.error.signal_cloned().map(|e| e.is_some()))
		.child_signal(state.error.signal_cloned().map(clone!(state => move |error| {
			error.map(|msg| {
				html!("div", {
					.class("error")
					.children(&mut [
						html!("span", {
							.text(&msg)
						}),
						html!("button", {
							.class("btn btn-sm")
							.text("×")
							.event(clone!(state => move |_: events::Click| {
								state.clear_error();
							}))
						}),
					])
				})
			})
		})))
	})
}

/// Render data fields (read-only)
fn render_data(state: &Arc<DetailViewState>) -> Dom {
	html!("div", {
		.class("detail-view-data")
		.visible_signal(state.is_loading.signal().map(|loading| !loading))
		.child_signal(state.data.signal_cloned().map(|data| {
			data.map(|fields| {
				let field_doms: Vec<Dom> = fields
					.iter()
					.map(|(key, value)| {
						html!("div", {
							.class("detail-field")
							.children(&mut [
								html!("label", {
									.class("detail-field-label")
									.text(key)
								}),
								html!("div", {
									.class("detail-field-value")
									.text(&format_value(value))
								}),
							])
						})
					})
					.collect();

				html!("div", {
					.class("detail-fields-container")
					.children(field_doms)
				})
			})
		}))
	})
}

/// Render action buttons (Edit, Delete)
fn render_actions(state: &Arc<DetailViewState>, model_name: &str, id: &str) -> Dom {
	let model_name_clone = model_name.to_string();
	let id_clone = id.to_string();

	html!("div", {
		.class("detail-view-actions")
		.visible_signal(state.is_loading.signal().map(|loading| !loading))
		.children(&mut [
			html!("button", {
				.class("btn btn-primary")
				.text("Edit")
				.event(clone!(model_name_clone, id_clone => move |_: events::Click| {
					// Navigate to edit view
					let window = web_sys::window().unwrap();
					let location = window.location();
					location.set_hash(&format!("#/{}/edit/{}", model_name_clone, id_clone)).unwrap();
				}))
			}),
			html!("button", {
				.class("btn btn-danger")
				.text("Delete")
				.event(clone!(state, model_name_clone => move |_: events::Click| {
					// Show confirmation dialog
					let window = web_sys::window().unwrap();
					let confirmed = window
						.confirm_with_message(&format!(
							"Are you sure you want to delete this {}?",
							model_name_clone
						))
						.unwrap_or(false);

					if confirmed {
						// Delete the item
						state.delete_item(clone!(model_name_clone => move || {
							// Navigate back to list view on success
							let window = web_sys::window().unwrap();
							let location = window.location();
							location.set_hash(&format!("#/{}", model_name_clone)).unwrap();
						}));
					}
				}))
			}),
		])
	})
}

/// Format a JSON value for display
fn format_value(value: &serde_json::Value) -> String {
	match value {
		serde_json::Value::Null => "".to_string(),
		serde_json::Value::Bool(b) => b.to_string(),
		serde_json::Value::Number(n) => n.to_string(),
		serde_json::Value::String(s) => s.clone(),
		serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
		serde_json::Value::Object(obj) => format!("[{} fields]", obj.len()),
	}
}
