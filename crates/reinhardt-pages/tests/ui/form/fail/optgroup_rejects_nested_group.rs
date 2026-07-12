use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidNestedOptGroupForm,
		action: "/invalid",
		fields: {
			status: ChoiceField<String> {
				widget: Select,
				choices: [OptGroup("Outer") {
					OptGroup("Inner") {
						("open", "Open"),
					},
				}, ],
			}
		}
	};
}
