//! form! macro without required 'name' property should fail

use reinhardt_pages::form;

fn main() {
	// This should fail - 'name' is required
	let _form = form! {
		action: "/api/submit",
		fields: {
			username: CharField { required },
		},
	};
}
