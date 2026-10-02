//! `ChoiceField<i64>` with typed dynamic choices store.

use reinhardt_pages::form;

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _ = form! {
			name: ChoiceForm,
			action: "/api/choice",
			fields: {
				choice_id: ChoiceField<i64> {
					required,
					choices_from: "choices",
					choice_value: "id",
					choice_label: "label",
				}
			}
		};
	});
}
