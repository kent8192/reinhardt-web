//! `ChoiceField<i64>` bind listener uses `FromStr` conversion for typed signals.

use reinhardt_pages::form;

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _ = form! {
			name: TypedChoiceForm,
			action: "/api/typed-choice",
			fields: {
				priority: ChoiceField<i64> {
					required,
					choices_from: "priorities",
					choice_value: "id",
					choice_label: "name",
				}
			}
		};
	});
}
