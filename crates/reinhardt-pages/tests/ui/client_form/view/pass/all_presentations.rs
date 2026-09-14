include!("../support.rs");
fn main() {
	reinhardt_pages::reactive::ReactiveScope::run(|| {
		let definition = RequestClientForm::new();
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let _page = form! {
			client_form: RequestClientForm,
			mutation: &mutation,
			id: "typed",
			customize: {
				text: {
					widget: Textarea,
					placeholder: "Text",
					autocomplete: "off",
					aria_label: "Text"
				},
				count: {
					widget: NumberInput,
					placeholder: "Count"
				},
				enabled: { widget: CheckboxInput },
				optional_enabled: {
					widget: Select,
					empty_label: "Unset",
					false_label: "No",
					true_label: "Yes"
				},
				mode: { widget: Select },
				optional_mode: {
					empty_label: "Unset"
				},
				id: {
					label: "ID"
				},
				submit: {
					label: "Submit"
				},
				mutation: {
					label: "Mutation"
				},
			},
			submit: {
				label: "Save",
				pending_label: "Saving",
				class: ""
			},
		}
		.into_page();
	});
}
