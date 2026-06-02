//! form! macro with legacy `strip_arguments` (reinhardt-web#3971).
//!
//! `strip_arguments` is a deprecated alias for `ambient_arguments`.
//! It remains accepted for backward compatibility.

use reinhardt_pages::form;

fn main() {
	// Server_fn declares both data fields and a CSRF parameter explicitly.
	// The legacy alias supplies the csrf_token expression at submit time.
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
