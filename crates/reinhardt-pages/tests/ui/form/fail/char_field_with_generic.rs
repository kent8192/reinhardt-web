//! `CharField<i32>` must be rejected with the "does not accept a type
//! parameter" diagnostic (per validator.rs `reject_generics` helper).

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: BadForm,
		action: "/x",
		fields: {
			username: CharField<i32> {},
		}
	};
}
