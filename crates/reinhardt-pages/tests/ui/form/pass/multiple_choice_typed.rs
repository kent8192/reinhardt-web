//! `MultipleChoiceField<i64>` yields `Signal<Vec<i64>>`.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: TagForm,
		action: "/api/tags",

		fields: {
			tag_ids: MultipleChoiceField {
				choices_from: "tags",
				choice_value: "id",
				choice_label: "name",
			}
		}

	};
}
