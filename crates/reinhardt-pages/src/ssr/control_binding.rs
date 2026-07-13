//! Server-side projection of controlled form-element state.

use std::borrow::Cow;

use crate::component::{ControlBinding, ControlKind, ControlValue};

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

pub(crate) fn option_selected(
	attrs: &[(Cow<'static, str>, Cow<'static, str>)],
	selected_values: &[String],
) -> bool {
	attrs.iter().any(|(name, value)| {
		name.as_ref() == "value"
			&& selected_values
				.iter()
				.any(|selected| selected == value.as_ref())
	})
}
