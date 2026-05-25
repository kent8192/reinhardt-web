//! form! macro with duplicate derived item name should fail

use reinhardt_pages::form;

fn main() {
	// This should fail - duplicate derived item name 'value'
	let _form = form! {
		name: DuplicateForm,
		action: "/api/submit",
		derived: {
			value: |form| form.x().get() + form.y().get(),
			value: |form| form.x().get() * form.y().get(),
		},
		fields: {
			x: IntegerField {
				required
			},
			y: IntegerField {
				required
			},
		},
	};
}
