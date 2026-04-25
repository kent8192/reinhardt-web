//! `strip_arguments` keys must not collide with declared form field names
//! (reinhardt-web#3971).
//!
//! A collision would mean the same identifier is both a user-facing input and
//! a stripped server_fn argument, which is ambiguous. The validator must reject
//! this at compile time.

use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: ConflictForm,
		server_fn: submit,
		method: Post,

		fields: {
			tenant_id: IntegerField { required },
		},

		strip_arguments: {
			tenant_id: 0u64,
		},
	};
}

fn submit() {}
