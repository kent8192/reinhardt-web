use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidListForm,
		action: "/invalid",
		fields: {
			enabled: BooleanField {
				widget: CheckboxInput,
				list: suggestions,
			}
		}
	};
}
