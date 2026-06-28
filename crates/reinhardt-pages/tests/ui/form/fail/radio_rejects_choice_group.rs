use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidRadioChoiceGroupForm,
		action: "/invalid",
		fields: {
			choice: ChoiceField {
				widget: RadioSelect,
				choices_from: "choices",
				choice_value: "id",
				choice_label: "label",
				choice_group: "group",
			}
		}
	};
}
