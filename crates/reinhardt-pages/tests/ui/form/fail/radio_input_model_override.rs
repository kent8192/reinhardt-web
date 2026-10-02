use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidModelRadioInput,
		model: Question,
		policy: QuestionFields,
		fields: [answer],
		overrides: {
			answer: { widget: RadioInput },
		},
	};
}
