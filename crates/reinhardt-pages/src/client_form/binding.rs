//! Typed adapters for optional numbers and DTO choices.

use super::ClientFormChoice;
use reinhardt_core::reactive::{Signal, batch};
use reinhardt_core::types::page::{
	ControlBinding, ControlBindingError, ControlKind, ControlValue, ControlWriteOutcome,
	NumberParseError, NumberValue,
};

/// Binds an optional primitive without a separate string value store.
pub fn optional_number_binding<T: NumberValue>(
	value: Signal<Option<T>>,
	error: Signal<Option<NumberParseError>>,
) -> ControlBinding {
	ControlBinding::from_parts(
		ControlKind::Number,
		None,
		value.id(),
		move || {
			ControlValue::Text(
				value
					.get()
					.as_ref()
					.map(NumberValue::format_control_value)
					.unwrap_or_default(),
			)
		},
		move |input| {
			let ControlValue::Text(raw) = input else {
				return Err(kind_mismatch(ControlKind::Number, &input));
			};
			let parsed = if raw.is_empty() {
				Ok(None)
			} else {
				T::parse_control_value(&raw).map(Some)
			};
			match parsed {
				Ok(next) => {
					batch(|| {
						value.set(next);
						error.set(None);
					});
					Ok(ControlWriteOutcome::Committed)
				}
				Err(rejected) => {
					error.set(Some(rejected.clone()));
					Ok(ControlWriteOutcome::Rejected(rejected))
				}
			}
		},
		move || {
			let previous = value.get_untracked();
			let previous_error = error.get_untracked();
			Box::new(move || {
				batch(|| {
					value.set(previous);
					error.set(previous_error);
				})
			})
		},
	)
	.with_lifetime_target(value.id())
}

/// Binds a required choice using an opaque ordinal DOM token.
pub fn choice_binding<T: Clone + PartialEq + 'static>(
	value: Signal<T>,
	choices: &'static [ClientFormChoice<T>],
) -> ControlBinding {
	choice_binding_impl(
		value,
		move |value| {
			choices
				.iter()
				.position(|choice| choice.value == *value)
				.map(|index| format!("choice:{index}"))
				.unwrap_or_default()
		},
		move |token| choice_index(token, choices.len()).map(|index| choices[index].value.clone()),
	)
}

/// Binds optional choices, keeping an unset value distinct from every serialized value.
pub fn optional_choice_binding<T: Clone + PartialEq + 'static>(
	value: Signal<Option<T>>,
	choices: &'static [ClientFormChoice<T>],
) -> ControlBinding {
	choice_binding_impl(
		value,
		move |value| match value {
			None => String::from("none"),
			Some(value) => choices
				.iter()
				.position(|choice| choice.value == *value)
				.map(|index| format!("choice:{index}"))
				.unwrap_or_default(),
		},
		move |token| {
			if token == "none" {
				Some(None)
			} else {
				choice_index(token, choices.len()).map(|index| Some(choices[index].value.clone()))
			}
		},
	)
}

/// Fixed typed options for an optional boolean.
pub static BOOLEAN_CHOICES: [ClientFormChoice<bool>; 2] = [
	ClientFormChoice {
		value: false,
		serialized_value: "false",
		label: "False",
	},
	ClientFormChoice {
		value: true,
		serialized_value: "true",
		label: "True",
	},
];

fn choice_index(token: &str, count: usize) -> Option<usize> {
	let ordinal = token.strip_prefix("choice:")?;
	let index: usize = ordinal.parse().ok()?;
	(index < count && ordinal == index.to_string()).then_some(index)
}

fn choice_binding_impl<T: Clone + 'static>(
	value: Signal<T>,
	encode: impl Fn(&T) -> String + 'static,
	decode: impl Fn(&str) -> Option<T> + 'static,
) -> ControlBinding {
	ControlBinding::from_parts(
		ControlKind::SelectOne,
		None,
		value.id(),
		move || ControlValue::Text(encode(&value.get())),
		move |input| {
			let ControlValue::Text(token) = input else {
				return Err(kind_mismatch(ControlKind::SelectOne, &input));
			};
			match decode(&token) {
				Some(next) => {
					value.set(next);
					Ok(ControlWriteOutcome::Committed)
				}
				None => Ok(ControlWriteOutcome::Ignored),
			}
		},
		move || {
			let previous = value.get_untracked();
			Box::new(move || value.set(previous))
		},
	)
	.with_lifetime_target(value.id())
}

fn kind_mismatch(control: ControlKind, value: &ControlValue) -> ControlBindingError {
	ControlBindingError::ValueKindMismatch {
		control,
		actual: match value {
			ControlValue::Text(_) => "text",
			ControlValue::Checked(_) => "checked",
			ControlValue::SelectedValues(_) => "selected-values",
			ControlValue::Files(_) => "files",
			#[cfg(all(target_family = "wasm", target_os = "unknown"))]
			ControlValue::File(_) => "file",
		},
	}
}
