//! Form definitions for examples-tutorial-basis.
//!
//! The static form shape is declared with `form!`; runtime state is derived
//! from the generated form contract through `use_form`.

use reinhardt::pages::{StaticFormMetadata, form, use_form};

macro_rules! vote_form {
	() => {
		form! {
			name: VoteForm,
			action: "/polls/vote/",
			method: Post,
			fields: {
				choice: HiddenField {
					initial: String::new(),
					label: "Choice",
					required,
				}
			}
		}
	};
}

/// Create vote form metadata from the generated form definition.
///
/// This form is primarily used to expose metadata for the voting form.
/// Runtime behavior is still instantiated here through `use_form` so the
/// metadata and runtime contract are generated from the same `form!` source.
pub fn create_vote_form() -> StaticFormMetadata {
	let form = vote_form!();
	let _runtime = use_form(&form).build();
	form.metadata()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_vote_form_metadata() {
		let metadata = create_vote_form();

		assert_eq!(metadata.fields.len(), 1);
		assert_eq!(metadata.fields[0].name, "choice");
		assert!(metadata.fields[0].required);
	}

	#[rstest]
	fn test_vote_form_runtime_contract() {
		let form = vote_form!();
		let runtime = use_form(&form).build();

		assert_eq!(runtime.get_values().choice, String::new());
		assert!(!runtime.form_state().is_dirty.get());
		assert!(!runtime.get_field_state(form.choice_field()).is_dirty);
	}
}
