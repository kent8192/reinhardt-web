//! form! emits local Values/Fields scaffolding for future use_form lowering.

use reinhardt_pages::form;

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _form = form! {
			name: GeneratedValuesFieldsForm,
			action: "/vote",
			fields: {
				question_id: HiddenField<i64> {
					initial: 1_i64,
				}
				choice_id: IntegerField {
					required,
				}
			}
		};
	});
}
