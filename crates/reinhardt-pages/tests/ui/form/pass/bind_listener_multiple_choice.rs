//! `MultipleChoiceField<i64>` bind listener iterates `selectedOptions`.

use reinhardt_pages::form;

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _ = form! {
			name: TagsForm,
			action: "/api/tags",
			fields: {
				tag_ids: MultipleChoiceField<i64> {
					choices_from: "tags",
					choice_value: "id",
					choice_label: "name",
				}
			}
		};
	});
}
