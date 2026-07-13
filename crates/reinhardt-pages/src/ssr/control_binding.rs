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
	let effective_value = option_value(element);
	selected_values
		.iter()
		.any(|selected| selected == &effective_value)
}

pub(crate) fn option_value(element: &PageElement) -> String {
	element
		.attrs()
		.iter()
		.find_map(|(name, value)| (name.as_ref() == "value").then_some(value.as_ref()))
		.map(str::to_owned)
		.unwrap_or_else(|| normalize_option_text(&collect_option_text(element.child_views())))
}

fn collect_option_text(pages: &[Page]) -> String {
	pages.iter().map(page_option_text).collect()
}

fn page_option_text(page: &Page) -> String {
	match page {
		Page::Element(element) if is_script(element.tag_name()) => String::new(),
		Page::Element(element) => collect_option_text(element.child_views()),
		Page::Text(text) => text.clone().into_owned(),
		Page::Fragment(children) => collect_option_text(children),
		Page::KeyedFragment(children) => children
			.iter()
			.map(|(_, child)| page_option_text(child))
			.collect(),
		Page::Outlet(outlet) => outlet.child().map(page_option_text).unwrap_or_default(),
		Page::Empty => String::new(),
		Page::WithHead { view, .. } => page_option_text(view),
		Page::ReactiveIf(_) | Page::Reactive(_) | Page::Suspense(_) | Page::Deferred(_) => {
			String::new()
		}
	}
}

fn is_script(tag: &str) -> bool {
	tag == "script" || tag == "svg:script"
}

fn normalize_option_text(text: &str) -> String {
	let mut normalized = String::with_capacity(text.len());
	let mut pending_space = false;

	for character in text.chars() {
		if matches!(character, '\t' | '\n' | '\x0c' | '\r' | ' ') {
			pending_space = !normalized.is_empty();
		} else {
			if pending_space {
				normalized.push(' ');
				pending_space = false;
			}
			normalized.push(character);
		}
	}

	normalized
}

#[cfg(test)]
mod tests {
	use std::cell::Cell;
	use std::rc::Rc;

	use super::*;
	use crate::component::IntoPage;

	#[test]
	fn inferred_value_does_not_invoke_reactive_factory() {
		// Arrange
		let renders = Rc::new(Cell::new(0));
		let render_count = Rc::clone(&renders);
		let option = PageElement::new("option")
			.child("Static")
			.child(Page::reactive(move || {
				render_count.set(render_count.get() + 1);
				Page::text(" Dynamic")
			}));

		// Act
		let value = option_value(&option);

		// Assert
		assert_eq!(value, "Static");
		assert_eq!(renders.get(), 0);
	}

	#[test]
	fn inferred_value_only_skips_html_and_svg_scripts_and_normalizes_ascii_whitespace() {
		// Arrange
		let option = PageElement::new("option")
			.child(" \tAlpha\n")
			.child(PageElement::new("script").child("html ignored").into_page())
			.child(PageElement::new("svg:script").child("ignored").into_page())
			.child(PageElement::new("math:script").child(" Gamma ").into_page())
			.child("\u{a0} Beta\x0c ");

		// Act
		let value = option_value(&option);

		// Assert
		assert_eq!(value, "Alpha Gamma \u{a0} Beta");
	}
}
