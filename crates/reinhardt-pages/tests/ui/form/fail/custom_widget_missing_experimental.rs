use reinhardt_pages::form;

fn picker(_: ()) -> reinhardt_pages::Page {
	reinhardt_pages::Page::Fragment(Vec::new())
}

struct Adapter;

fn main() {
	let _form = form! {
		name: MissingExperimentalForm,
		action: "/invalid",
		fields: {
			value: CharField {
				widget: CustomWidget(picker) {
					adapter: Adapter,
				},
			}
		}
	};
}
