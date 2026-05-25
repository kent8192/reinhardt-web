//! form! macro with explicit `strip_arguments` (reinhardt-web#3971).
//!
//! `strip_arguments` lets the user supply server_fn arguments that should not
//! appear as user-facing form fields. This test exercises the common case of
//! routing a CSRF token explicitly instead of relying on the deprecated
//! implicit auto-injection path.

use reinhardt_pages::form;

fn main() {
	// Server_fn declares both data fields and a CSRF parameter explicitly.
	// `strip_arguments` supplies the csrf_token expression at submit time.
	let _vote_form = form! {
		name: VoteForm,
		server_fn: submit_vote,
		method: Post,
		strip_arguments: {
			csrf_token: String::new(),
		},

		fields: {
			_question_id: IntegerField {
				widget: HiddenInput,
			}
			_choice_id: IntegerField {
				required,
			}
		}

	};

	// Multiple stripped arguments append in source order.
	let _multi_form = form! {
		name: MultiForm,
		server_fn: submit_multi,
		method: Post,
		strip_arguments: {
			csrf_token: String::new(),
			tenant_id: 0u64,
		},

		fields: {
			_payload: CharField {
				required,
			}
		}

	};
}

// Mock server functions (would normally be defined with #[server_fn]).
fn submit_vote() {}
fn submit_multi() {}
