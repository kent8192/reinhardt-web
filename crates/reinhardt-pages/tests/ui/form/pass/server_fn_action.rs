//! form! macro with server_fn action

use reinhardt_pages::form;

fn main() {
	// Form with server_fn instead of URL action
	let _vote_form = form! {
		name: VoteForm,
		server_fn: submit_vote,

		fields: {
			_question_id: IntegerField { widget: HiddenInput },
			_choice_id: IntegerField { required },
		},
	};
}

// Mock server function (would normally be defined with #[server_fn])
fn submit_vote() {}
