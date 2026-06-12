//! Form definitions for examples-tutorial-basis.
//!
//! The static form shape is declared with `form!`; runtime state is derived
//! from the generated form contract through `use_form`.

use crate::apps::polls::server_fn::submit_vote;
use reinhardt::pages::{StaticFormMetadata, form, use_form};

/// Create vote form metadata from the generated form definition.
///
/// This form is primarily used to expose metadata for the voting form.
/// Runtime behavior is still instantiated here through `use_form` so the
/// metadata and runtime contract are generated from the same `form!` source.
pub fn create_vote_form() -> StaticFormMetadata {
	let form = form! {
		name: VoteForm,
		server_fn: submit_vote,
		method: Post,
		fields: {
			question_id: HiddenField {
				initial: String::new(),
			}
			choice_id: HiddenField {
				initial: String::new(),
				label: "Choice",
				required,
			}
		}
	};
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

		assert_eq!(metadata.fields.len(), 2);
		assert_eq!(metadata.fields[0].name, "question_id");
		assert_eq!(metadata.fields[1].name, "choice_id");
		assert!(metadata.fields[1].required);
	}

	#[rstest]
	fn test_vote_form_runtime_contract() {
		let form = form! {
			name: VoteForm,
			server_fn: submit_vote,
			method: Post,
			fields: {
				question_id: HiddenField {
					initial: String::new(),
				}
				choice_id: HiddenField {
					initial: String::new(),
					label: "Choice",
					required,
				}
			}
		};
		let runtime = use_form(&form).build();

		assert_eq!(runtime.get_values().question_id, String::new());
		assert_eq!(runtime.get_values().choice_id, String::new());
		assert!(!runtime.form_state().is_dirty.get());
		assert!(!runtime.get_field_state(form.choice_id_field()).is_dirty);
	}
}
