//! Server-side projection of controlled form-element state.

use crate::component::{ControlBinding, ControlKind, ControlValue, Page, PageElement};

pub(crate) struct SsrControlProjection {
	pub value: Option<String>,
	pub checked: bool,
	pub textarea_text: Option<String>,
	pub selected_values: Vec<String>,
}

pub(crate) fn project(binding: Option<&ControlBinding>) -> SsrControlProjection {
	let mut projection = SsrControlProjection {
		value: None,
		checked: false,
		textarea_text: None,
		selected_values: Vec::new(),
	};

	let Some(binding) = binding else {
		return projection;
	};

	match (binding.kind(), binding.read()) {
		(ControlKind::Text | ControlKind::Number, ControlValue::Text(value)) => {
			projection.value = Some(value.clone());
			projection.textarea_text = Some(value);
		}
		(ControlKind::Checkbox | ControlKind::Radio, ControlValue::Checked(checked)) => {
			projection.checked = checked;
		}
		(ControlKind::SelectOne, ControlValue::Text(value)) => {
			projection.selected_values.push(value);
		}
		(ControlKind::SelectMany, ControlValue::SelectedValues(values)) => {
			projection.selected_values = values;
		}
		_ => {}
	}

	projection
}

pub(crate) fn option_selected(element: &PageElement, selected_values: &[String]) -> bool {
	let effective_value = element
		.attrs()
		.iter()
		.find_map(|(name, value)| (name.as_ref() == "value").then_some(value.as_ref()))
		.map(str::to_owned)
		.unwrap_or_else(|| flattened_text_content(element.child_views()));
	selected_values
		.iter()
		.any(|selected| selected == &effective_value)
}

fn flattened_text_content(pages: &[Page]) -> String {
	pages.iter().map(page_text_content).collect()
}

fn page_text_content(page: &Page) -> String {
	match page {
		Page::Element(element) => flattened_text_content(element.child_views()),
		Page::Text(text) => text.clone().into_owned(),
		Page::Fragment(children) => flattened_text_content(children),
		Page::KeyedFragment(children) => children
			.iter()
			.map(|(_, child)| page_text_content(child))
			.collect(),
		Page::Outlet(outlet) => outlet.child().map(page_text_content).unwrap_or_default(),
		Page::Empty => String::new(),
		Page::WithHead { view, .. } => page_text_content(view),
		Page::ReactiveIf(reactive_if) => {
			if reactive_if.condition() {
				page_text_content(&reactive_if.then_view())
			} else {
				page_text_content(&reactive_if.else_view())
			}
		}
		Page::Reactive(reactive) => page_text_content(&reactive.render()),
		Page::Suspense(node) => page_text_content(&node.render_branch()),
		Page::Deferred(node) => page_text_content(&node.render_content()),
	}
}
