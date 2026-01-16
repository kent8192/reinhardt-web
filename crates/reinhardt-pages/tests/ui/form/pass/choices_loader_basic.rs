//! Dynamic ChoiceField basic usage - choices loaded from server

use reinhardt_pages::form;

fn main() {
	// Form with dynamic choice field
	// choices_loader specifies the server function to load choice options.
	// choices_from/choice_value/choice_label map the loaded data to radio/select options.
	let _voting_form = form! {
		name: VotingForm,
		server_fn: submit_vote,
		choices_loader: get_poll_choices,

		fields: {
			_choice: ChoiceField {
				required,
				choices_from: "choices",
				choice_value: "id",
				choice_label: "choice_text",
			},
		},
	};
}
