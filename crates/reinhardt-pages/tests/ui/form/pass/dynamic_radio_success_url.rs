//! Dynamic RadioSelect choices can be rendered with a success_url redirect.

use reinhardt_pages::form;

fn main() {
	let qid = 1_i64;

	let _form = form! {
		name: DynamicRadioSuccessUrlForm,
		server_fn: submit_vote,
		method: Post,
		success_url: |_form| format!("/polls/{qid}/results/"),
		fields: {
			question_id: HiddenField {
				initial: qid.to_string(),
			}
			choice_id: ChoiceField {
				widget: RadioSelect,
				required,
				choices_from: "choices",
				choice_value: "id",
				choice_label: "choice_text",
			}
		}
	};
}

fn submit_vote() {}
