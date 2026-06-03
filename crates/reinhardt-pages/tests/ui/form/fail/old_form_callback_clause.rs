use reinhardt_pages::form;

async fn save_name(_name: String) -> Result<i64, String> {
	Ok(1)
}

fn main() {
	let _form = form! {
		name: LegacyCallbackForm,
		server_fn: save_name,
		fields: {
			name: CharField {
				initial: "",
			}
		},
		on_success: |_result: i64| {},
	};
}
