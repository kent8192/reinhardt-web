use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidDatalistChoiceGroupForm,
		action: "/invalid",
		fields: {
			suggestions: Datalist {
				choices_from: "items",
				choice_value: "value",
				choice_label: "label",
				choice_group: "group",
			}
		}
	};
}
