//! Typed binding contracts shared by handwritten and generated ClientForm views.
#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::component::{ControlValue, ControlWriteOutcome};
use reinhardt_pages::control_binding::__private::{
	NumberBinding, SelectOneBinding, into_control_binding,
};
use reinhardt_pages::reactive::ReactiveScope;
use reinhardt_pages::{
	ClientForm, ClientFormChoices, FormRuntimeSource, NumberParseErrorKind, use_form,
};
use serde::Serialize;

#[test]
fn optional_number_clears_parse_error_and_updates_value_as_one_observation() {
	ReactiveScope::run(|| {
		use reinhardt_pages::reactive::{Effect, EffectTiming};
		use std::{cell::RefCell, rc::Rc};
		// Arrange
		let definition = TypedRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let field = definition.count_field();
		let binding = into_control_binding::<NumberBinding, _>(runtime.field(field), ());
		binding.write(ControlValue::Text("invalid".into())).unwrap();
		let observations = Rc::new(RefCell::new(Vec::new()));
		let _effect = Effect::new_with_timing(
			{
				let observations = observations.clone();
				let definition = definition.clone();
				let value = runtime.watch_field::<Option<i32>>(field);
				move || {
					observations.borrow_mut().push((
						value.get(),
						definition.runtime_custom_widget_error(field).is_some(),
					))
				}
			},
			EffectTiming::Layout,
		);
		observations.borrow_mut().clear();
		// Act
		binding.write(ControlValue::Text("8".into())).unwrap();
		// Assert
		assert_eq!(*observations.borrow(), vec![(Some(8), false)]);
	});
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, ClientFormChoices)]
enum Mode {
	#[default]
	#[serde(rename = "")]
	Empty,
	Active,
}

#[derive(Clone, Serialize, ClientForm)]
struct TypedRequest {
	active: bool,
	count: Option<i32>,
	enabled: Option<bool>,
	mode: Mode,
	optional_mode: Option<Mode>,
}

#[test]
fn optional_number_keeps_typed_state_when_an_edit_is_rejected() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = TypedRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let field = definition.count_field();
		let binding = into_control_binding::<NumberBinding, _>(runtime.field(field), ());
		assert_eq!(binding.read(), ControlValue::Text(String::new()));
		// Act
		assert_eq!(
			binding
				.write(ControlValue::Text(String::from("7")))
				.unwrap(),
			ControlWriteOutcome::Committed
		);
		let snapshot = binding.snapshot();
		let outcome = binding
			.write(ControlValue::Text(String::from("1.5")))
			.unwrap();
		// Assert
		let ControlWriteOutcome::Rejected(error) = outcome else {
			panic!("fractional i32 input must be rejected")
		};
		assert_eq!(error.kind(), NumberParseErrorKind::Invalid);
		assert_eq!(runtime.get_values().count, Some(7));
		assert!(definition.runtime_custom_widget_error(field).is_some());
		drop(snapshot);
		assert_eq!(definition.runtime_custom_widget_error(field), None);
		assert_eq!(
			binding.write(ControlValue::Text(String::new())).unwrap(),
			ControlWriteOutcome::Committed
		);
		assert_eq!(runtime.get_values().count, None);
		assert_eq!(binding.read(), ControlValue::Text(String::new()));
	});
}

#[test]
fn optional_boolean_retains_all_three_states() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = TypedRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let binding = into_control_binding::<SelectOneBinding, _>(
			runtime.field(definition.enabled_field()),
			(),
		);
		assert_eq!(binding.read(), ControlValue::Text(String::from("none")));
		// Act / Assert
		for (token, expected) in [
			("choice:0", Some(false)),
			("choice:1", Some(true)),
			("none", None),
		] {
			assert_eq!(
				binding.write(ControlValue::Text(token.into())).unwrap(),
				ControlWriteOutcome::Committed
			);
			assert_eq!(runtime.get_values().enabled, expected);
			assert_eq!(binding.read(), ControlValue::Text(token.into()));
		}
	});
}

#[test]
fn choices_distinguish_empty_serialization_and_ignore_invalid_tokens() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = TypedRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let binding = into_control_binding::<SelectOneBinding, _>(
			runtime.field(definition.optional_mode_field()),
			(),
		);
		// Act / Assert
		binding
			.write(ControlValue::Text(String::from("choice:0")))
			.unwrap();
		assert_eq!(runtime.get_values().optional_mode, Some(Mode::Empty));
		assert_eq!(
			serde_json::to_string(&runtime.get_values().optional_mode).unwrap(),
			"\"\""
		);
		for invalid in [
			"",
			"choice:00",
			"choice:+1",
			"choice:99",
			"Active",
			"other:0",
		] {
			assert_eq!(
				binding.write(ControlValue::Text(invalid.into())).unwrap(),
				ControlWriteOutcome::Ignored
			);
			assert_eq!(runtime.get_values().optional_mode, Some(Mode::Empty));
		}
		let snapshot = binding.snapshot();
		binding
			.write(ControlValue::Text(String::from("none")))
			.unwrap();
		assert_eq!(runtime.get_values().optional_mode, None);
		drop(snapshot);
		assert_eq!(runtime.get_values().optional_mode, Some(Mode::Empty));
		let required =
			into_control_binding::<SelectOneBinding, _>(runtime.field(definition.mode_field()), ());
		assert_eq!(
			required
				.write(ControlValue::Text(String::from("none")))
				.unwrap(),
			ControlWriteOutcome::Ignored
		);
		assert_eq!(runtime.get_values().mode, Mode::Empty);
		assert_eq!(
			serde_json::to_value(TypedRequestClientForm::to_request(&runtime)).unwrap(),
			serde_json::json!({"active":false,"count":null,"enabled":null,"mode":"","optional_mode":""})
		);
	});
}

#[test]
fn explicit_field_writes_win_hydration_and_reset_callbacks_share_an_epoch() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = TypedRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let binding =
			into_control_binding::<NumberBinding, _>(runtime.field(definition.count_field()), ());
		assert!(!binding.source_preferred_on_hydration());
		assert!(binding.has_native_reset());
		// Act
		runtime.reset_field(definition.count_field());
		// Assert
		assert!(binding.source_preferred_on_hydration());
		let previous = definition.runtime_native_reset_epoch();
		binding.notify_native_reset();
		assert_eq!(definition.runtime_native_reset_epoch(), previous + 1);
	});
}
