//! CSRF supplied through ambient_arguments remains accepted but is deprecated.

use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: CsrfAmbientForm,
		server_fn: submit,
		method: Post,
		ambient_arguments: {
			csrf_token: String::new(),
		},
		fields: {
			payload: CharField {
				required,
			}
		}
	};
}

fn submit() {}
