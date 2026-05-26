//! `MultipleChoiceField<i64>` bind listener iterates `selectedOptions`.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: TagsForm,
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
