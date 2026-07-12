use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: ChoiceOptionsForm,
		action: "/choices",
		choices_loader: load_choice_options,
		fields: {
			sprint_week: CharField {
				widget: WeekInput,
				list: sprint_weeks,
			}
			sprint_weeks: Datalist {
				choices_from: "weeks",
				choice_value: "value",
				choice_label: "label",
				choice_disabled: "archived",
			}
			status: ChoiceField<String> {
				widget: Select,
				choices: [OptGroup("Active") {
					("open", "Open"),
					("review", "In review"),
				}, OptGroup("Closed") {
					disabled,
					("done", "Done") { disabled },
				}, ],
			}
			labels: MultipleChoiceField<String> {
				widget: SelectMultiple,
				choices_from: "labels",
				choice_value: "value",
				choice_label: "label",
				choice_group: "group",
				choice_group_disabled: "group_disabled",
				choice_disabled: "disabled",
			}
		}
	};
}

#[allow(dead_code)]
async fn load_choice_options() -> Result<ChoiceOptions, reinhardt_pages::ServerFnError> {
	Ok(ChoiceOptions {
		weeks: Vec::new(),
		labels: Vec::new(),
	})
}

#[allow(dead_code)]
struct ChoiceOptions {
	weeks: Vec<OptionItem>,
	labels: Vec<OptionItem>,
}

#[allow(dead_code)]
struct OptionItem {
	value: String,
	label: String,
	archived: bool,
	disabled: bool,
	group: String,
	group_disabled: bool,
}
