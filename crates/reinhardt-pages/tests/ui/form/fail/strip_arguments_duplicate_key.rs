//! Duplicate keys inside `strip_arguments: { ... }` must be rejected
//! (reinhardt-web#3971). Each server_fn argument may only be supplied once.

use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: DupForm,
		server_fn: submit,
		method: Post,

		fields: {
			payload: CharField { required },
		},

		strip_arguments: {
			csrf_token: String::new(),
			csrf_token: String::from("again"),
		},
	};
}

fn submit() {}
