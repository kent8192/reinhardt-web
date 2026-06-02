#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::{
	FormFields, FormOptions, FormValidate, FormValidationError, FormValues, Signal, use_form,
};

#[derive(Clone, PartialEq, Debug)]
struct VoteFormValues {
	question_id: i64,
	choice_id: i64,
}

#[derive(Clone)]
struct VoteFormFields {
	question_id: Signal<i64>,
	choice_id: Signal<i64>,
}

impl FormFields for VoteFormFields {
	type Values = VoteFormValues;

	fn from_values(values: &Self::Values) -> Self {
		Self {
			question_id: Signal::new(values.question_id),
			choice_id: Signal::new(values.choice_id),
		}
	}

	fn values(&self) -> Self::Values {
		VoteFormValues {
			question_id: self.question_id.get(),
			choice_id: self.choice_id.get(),
		}
	}

	fn apply_values(&self, values: &Self::Values) {
		self.question_id.set(values.question_id);
		self.choice_id.set(values.choice_id);
	}
}

impl FormValues for VoteFormValues {
	type Fields = VoteFormFields;

	fn field_names() -> &'static [&'static str] {
		&["question_id", "choice_id"]
	}
}

impl FormValidate for VoteFormValues {
	fn validate(&self) -> Result<(), FormValidationError> {
		if self.choice_id <= 0 {
			return Err(FormValidationError::field(
				"choice_id",
				"choice_id must be selected",
			));
		}
		Ok(())
	}
}

#[test]
fn use_form_exposes_typed_fields_and_values() {
	let form = use_form(FormOptions::<VoteFormValues>::new(VoteFormValues {
		question_id: 7,
		choice_id: 0,
	}));

	let fields = form.fields();
	fields.choice_id.set(42);

	assert_eq!(
		form.values(),
		VoteFormValues {
			question_id: 7,
			choice_id: 42,
		}
	);
	assert!(form.dirty().get());
}

#[test]
fn use_form_reset_restores_initial_values() {
	let form = use_form(FormOptions::<VoteFormValues>::new(VoteFormValues {
		question_id: 7,
		choice_id: 0,
	}));

	form.fields().choice_id.set(42);
	assert!(form.dirty().get());

	form.reset();

	assert_eq!(
		form.values(),
		VoteFormValues {
			question_id: 7,
			choice_id: 0,
		}
	);
	assert!(!form.dirty().get());
}

#[test]
fn use_form_validation_blocks_submit_error_free_path() {
	let form = use_form(
		FormOptions::<VoteFormValues>::new(VoteFormValues {
			question_id: 7,
			choice_id: 0,
		})
		.validate(FormValidate::validate),
	);

	let result = form.validate();

	assert!(result.is_err());
	assert_eq!(
		form.field_errors().get().get("choice_id").cloned(),
		Some(vec!["choice_id must be selected".to_string()])
	);
	assert_eq!(
		form.error().get(),
		Some("choice_id must be selected".to_string())
	);
}

#[test]
fn native_submit_runs_validation_and_does_not_await_async_action() {
	let form = use_form(
		FormOptions::<VoteFormValues>::new(VoteFormValues {
			question_id: 7,
			choice_id: 0,
		})
		.validate(FormValidate::validate)
		.on_submit(|_values| async move { Ok::<(), String>(()) }),
	);

	form.submit();

	assert_eq!(form.submit_error().get(), None);
	assert_eq!(form.loading().get(), false);
	assert_eq!(
		form.error().get(),
		Some("choice_id must be selected".to_string())
	);
}
