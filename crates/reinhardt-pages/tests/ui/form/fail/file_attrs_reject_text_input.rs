use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidFileAttrsForm,
		action: "/invalid",
		fields: {
			title: CharField {
				widget: TextInput,
				accept: "image/png",
			}
		}
	};
}
