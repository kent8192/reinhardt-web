//! Filters component for list view

use crate::state::ListViewState;
use dominator::{Dom, clone, events, html};
use futures_signals::signal_vec::SignalVecExt;
use reinhardt_admin_types::{FilterChoice, FilterInfo, FilterType};
use std::sync::Arc;
use wasm_bindgen::JsCast;

/// Render filters panel
pub fn render(state: Arc<ListViewState>) -> Dom {
	html!("div", {
		.class("filters-panel")
		.children(&mut [
			html!("h3", {
				.class("filters-title")
				.text("Filters")
			}),
			html!("div", {
				.class("filters-list")
				.children_signal_vec(
					state.available_filters.signal_vec_cloned().map(clone!(state => move |filter| {
						render_filter(&state, filter)
					}))
				)
			}),
		])
	})
}

/// Render a single filter
fn render_filter(state: &Arc<ListViewState>, filter: FilterInfo) -> Dom {
	html!("div", {
		.class("filter-item")
		.children(&mut [
			html!("label", {
				.class("filter-label")
				.text(&filter.title)
			}),
			render_filter_input(state, filter),
		])
	})
}

/// Render filter input based on type
fn render_filter_input(state: &Arc<ListViewState>, filter: FilterInfo) -> Dom {
	match filter.filter_type.clone() {
		FilterType::Boolean => render_boolean_filter(state, filter),
		FilterType::Choice { choices } => render_choice_filter(state, filter, choices),
		FilterType::DateRange { ranges } => render_choice_filter(state, filter, ranges),
		FilterType::NumberRange { ranges } => render_choice_filter(state, filter, ranges),
	}
}

/// Render boolean filter (checkbox)
fn render_boolean_filter(state: &Arc<ListViewState>, filter: FilterInfo) -> Dom {
	let field = filter.field.clone();
	let is_checked = filter.current_value.is_some();

	html!("input" => web_sys::HtmlInputElement, {
		.class("filter-checkbox")
		.attr("type", "checkbox")
		.prop("checked", is_checked)
		.event(clone!(state, field => move |event: events::Change| {
			let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
			if input.checked() {
				state.set_filter(field.clone(), "true".to_string());
			} else {
				state.clear_filter(&field);
			}
		}))
	})
}

/// Render choice filter (dropdown)
fn render_choice_filter(
	state: &Arc<ListViewState>,
	filter: FilterInfo,
	choices: Vec<FilterChoice>,
) -> Dom {
	let field = filter.field.clone();
	let current_value = filter.current_value.clone().unwrap_or_default();

	html!("select" => web_sys::HtmlSelectElement, {
		.class("filter-select")
		.children(&mut [
			// Empty option (no filter)
			html!("option", {
				.attr("value", "")
				.prop("selected", current_value.is_empty())
				.text("---")
			}),
		])
		.children(choices.into_iter().map(|choice| {
			let is_selected = current_value == choice.value;
			html!("option", {
				.attr("value", &choice.value)
				.prop("selected", is_selected)
				.text(&choice.label)
			})
		}).collect::<Vec<_>>())
		.event(clone!(state, field => move |event: events::Change| {
			let select: web_sys::HtmlSelectElement = event.target().unwrap().dyn_into().unwrap();
			let value = select.value();
			if value.is_empty() {
				state.clear_filter(&field);
			} else {
				state.set_filter(field.clone(), value);
			}
		}))
	})
}
