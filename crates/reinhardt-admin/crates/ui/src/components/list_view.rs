//! List view component for displaying model instances

use crate::components::common::render_confirm_modal;
use crate::components::pagination;
use crate::state::{AppState, ListViewState};
use dominator::{Dom, clone, events, html};
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::SignalVecExt;
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::JsCast;

/// Render the list view for a specific model
pub fn render(state: Arc<AppState>, model_name: String) -> Dom {
	// Get or create list view state for this model
	let list_state = state.get_list_view_state(&model_name);

	// Load data on initial render
	list_state.load_data();

	html!("div", {
		.class("list-view")
		.children(&mut [
			render_header(&list_state),
			render_search_bar(&list_state),
			crate::components::filters::render(Arc::clone(&list_state)),
			render_active_filters(&list_state),
			render_table(&list_state),
			render_pagination_section(&list_state),
			// Confirmation modal for delete actions
			render_confirm_modal(Arc::clone(&list_state.confirm_modal)),
		])
	})
}

/// Render the header with title and "Add New" button
fn render_header(state: &Arc<ListViewState>) -> Dom {
	html!("div", {
		.class("list-view-header")
		.children(&mut [
			html!("h1", {
				.class("list-view-title")
				.text(&format!("{} List", state.model_name))
			}),
			html!("button", {
				.class("btn btn-primary")
				.text("Add New")
				.event(clone!(state => move |_: events::Click| {
					let window = web_sys::window().unwrap();
					let location = window.location();
					location.set_hash(&format!("#/{}/create", state.model_name)).unwrap();
				}))
			}),
		])
	})
}

/// Render the search bar
fn render_search_bar(state: &Arc<ListViewState>) -> Dom {
	html!("div", {
		.class("list-view-search")
		.children(&mut [
			html!("input" => web_sys::HtmlInputElement, {
				.class("search-input")
				.attr("type", "text")
				.attr("placeholder", "Search...")
				.prop_signal("value", state.search_query.signal_cloned())
				.event(clone!(state => move |event: events::Input| {
					let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
					state.set_search(input.value());
				}))
			}),
		])
	})
}

/// Render active filters as chips
fn render_active_filters(state: &Arc<ListViewState>) -> Dom {
	html!("div", {
		.class("list-view-filters")
		.child_signal(state.filters.signal_ref(clone!(state => move |filters| {
			if filters.is_empty() {
				None
			} else {
				let chips: Vec<Dom> = filters.iter().map(|(key, value)| {
					let key_clone = key.clone();
					html!("div", {
						.class("filter-chip")
						.children(&mut [
							html!("span", {
								.text(&format!("{}: {}", key, value))
							}),
							html!("button", {
								.class("filter-chip-remove")
								.text("×")
								.event(clone!(state, key_clone => move |_: events::Click| {
									state.clear_filter(&key_clone);
								}))
							}),
						])
					})
				}).collect();
				Some(html!("div", {
					.class("filter-chips-container")
					.children(chips)
				}))
			}
		})))
	})
}

/// Render the table container (with loading/error states)
fn render_table(state: &Arc<ListViewState>) -> Dom {
	html!("div", {
		.class("list-view-table-container")
		.children(&mut [
			// Loading indicator
			html!("div", {
				.class("loading")
				.visible_signal(state.is_loading.signal())
				.text("Loading...")
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
			// Table
			html!("table", {
				.class("list-view-table")
				.visible_signal(state.is_loading.signal().map(|loading| !loading))
				.children(&mut [
					render_table_header(state),
					render_table_body(state),
				])
			}),
		])
	})
}

/// Render table header with dynamic columns from API
fn render_table_header(state: &Arc<ListViewState>) -> Dom {
	html!("thead", {
		.child(html!("tr", {
			// Dynamic columns from API
			.children_signal_vec(state.columns.signal_vec_cloned().map(clone!(state => move |column| {
				let field = column.field.clone();
				html!("th", {
					.text(&column.label)
					.apply_if(column.sortable, clone!(state, field => move |dom| {
						dom.style("cursor", "pointer")
							.event(clone!(state, field => move |_: events::Click| {
								state.set_sort(field.clone(), false);
							}))
					}))
				})
			})))
			// Actions column (always present)
			.child(html!("th", {
				.text("Actions")
			}))
		}))
	})
}

/// Render table body with reactive rows
fn render_table_body(state: &Arc<ListViewState>) -> Dom {
	html!("tbody", {
		.children_signal_vec(state.items.signal_vec_cloned().map(clone!(state => move |item| {
			// Get current columns snapshot for this row
			let columns: Vec<_> = state.columns.lock_ref().iter().cloned().collect();
			render_table_row(&state, item, &columns)
		})))
	})
}

/// Render a single table row with dynamic columns
fn render_table_row(
	state: &Arc<ListViewState>,
	item: HashMap<String, serde_json::Value>,
	columns: &[reinhardt_admin_types::ColumnInfo],
) -> Dom {
	// Extract ID (assuming "id" field exists)
	let id = item
		.get("id")
		.map(format_value)
		.unwrap_or_else(|| "unknown".to_string());

	// Extract display name for delete confirmation (use first non-id column)
	let display_name = columns
		.iter()
		.find(|c| c.field != "id")
		.and_then(|c| item.get(&c.field))
		.map(format_value)
		.unwrap_or_else(|| id.clone());

	// Build dynamic column cells
	let column_cells: Vec<Dom> = columns
		.iter()
		.map(|column| {
			let value = item
				.get(&column.field)
				.map(format_value)
				.unwrap_or_else(|| "-".to_string());

			html!("td", {
				.text(&value)
			})
		})
		.collect();

	html!("tr", {
		.children(column_cells)
		// Actions column
		.child(html!("td", {
			.class("actions")
			.children(&mut [
				html!("button", {
					.class("btn btn-sm btn-view")
					.text("View")
					.event(clone!(state, id => move |_: events::Click| {
						let window = web_sys::window().unwrap();
						let location = window.location();
						location.set_hash(&format!("#/{}/{}", state.model_name, id)).unwrap();
					}))
				}),
				html!("button", {
					.class("btn btn-sm btn-edit")
					.text("Edit")
					.event(clone!(state, id => move |_: events::Click| {
						let window = web_sys::window().unwrap();
						let location = window.location();
						location.set_hash(&format!("#/{}/{}/edit", state.model_name, id)).unwrap();
					}))
				}),
				html!("button", {
					.class("btn btn-sm btn-delete")
					.text("Delete")
					.event(clone!(state, id, display_name => move |_: events::Click| {
						state.show_delete_confirmation(
							id.clone(),
							Some(display_name.clone()),
						);
					}))
				}),
			])
		}))
	})
}

/// Format a JSON value for display in table cells
fn format_value(value: &serde_json::Value) -> String {
	match value {
		serde_json::Value::Null => "-".to_string(),
		serde_json::Value::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
		serde_json::Value::Number(n) => n.to_string(),
		serde_json::Value::String(s) => s.clone(),
		serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
		serde_json::Value::Object(_) => "[Object]".to_string(),
	}
}

/// Render pagination section
fn render_pagination_section(state: &Arc<ListViewState>) -> Dom {
	// Create pagination state that mirrors ListViewState
	let pagination_state = pagination::PaginationState::new();

	// Sync pagination state with list view state
	pagination_state.current_page.set(state.current_page.get());
	pagination_state.total_pages.set(state.total_pages.get());
	pagination_state.page_size.set(state.page_size.get());
	pagination_state.total_items.set(state.total_count.get());

	pagination::render(pagination::PaginationProps {
		state: pagination_state,
		on_page_change: Arc::new(clone!(state => move |page| {
			state.goto_page(page);
		})),
		on_page_size_change: Some(Arc::new(clone!(state => move |size| {
			state.set_page_size(size);
		}))),
	})
}
