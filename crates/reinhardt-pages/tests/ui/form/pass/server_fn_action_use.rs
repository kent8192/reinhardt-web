//! Regression test for reinhardt-web#4070.
//!
//! When `server_fn:` is set to a function brought in scope via `use`, the
//! `form!` expansion must reference the function on every target so the
//! `use` statement is not flagged by `unused_imports` on native builds.
#![deny(unused_imports)]

use reinhardt_pages::form;

mod server_fns {
	pub fn submit_vote() {}
}

use server_fns::submit_vote;

fn main() {
	let _vote_form = form! {
		name: VoteForm,
		server_fn: submit_vote,

		fields: {
			_question_id: IntegerField { widget: HiddenInput },
			_choice_id: IntegerField { required },
		},
	};
}
