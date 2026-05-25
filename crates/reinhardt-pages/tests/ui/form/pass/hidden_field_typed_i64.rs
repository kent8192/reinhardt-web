//! `HiddenField<i64>` should produce `Signal<i64>` and compile cleanly.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: NumericForm,
		action: "/api/numeric",

		fields: {
			question_id: HiddenField {
				initial: 42i64,
			}
		}

	};
}
