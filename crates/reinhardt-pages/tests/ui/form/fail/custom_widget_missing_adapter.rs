use reinhardt_pages::form;

fn picker(_: ()) -> reinhardt_pages::Page {
	reinhardt_pages::Page::Fragment(Vec::new())
}

fn main() {
	let _form = form! {
		name: MissingAdapterForm,
		action: "/invalid",
		fields: {
			value: CharField {
				widget: CustomWidget(picker) {
					experimental,
				},
			}
		}
	};
}
