//! server_fn forms do not need a CSRF business argument.

use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: NoCsrfBusinessArgForm,
		server_fn: submit,
		method: Post,
		fields: {
			payload: CharField {
				required,
			}
		}
	};
}

fn submit() {}
