//! Dynamic choice accessors keep the public `(value, label)` tuple signal.

use reinhardt_pages::form;

fn main() {
	let form = form! {
		name: ChoiceAccessorForm,
		action: "/api/choice-accessor",
		fields: {
			choice_id: ChoiceField<i64> {
				choices_from: "choices",
				choice_value: "id",
				choice_label: "label",
			}
			tag_ids: MultipleChoiceField<i64> {
				choices_from: "tags",
				choice_value: "id",
				choice_label: "label",
			}
		}
	};

	form.choice_id_choices()
		.set(vec![(1_i64, "First".to_string())]);
	form.tag_ids_choices().set(vec![(2_i64, "Second".to_string())]);
}
