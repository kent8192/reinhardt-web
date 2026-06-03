//! `ambient_arguments` supplies server_fn arguments from ambient context.

use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: TenantForm,
		server_fn: submit,
		method: Post,
		ambient_arguments: {
			tenant_id: 10u64,
		},
		fields: {
			payload: CharField {
				required,
			}
		}
	};
}

fn submit() {}
