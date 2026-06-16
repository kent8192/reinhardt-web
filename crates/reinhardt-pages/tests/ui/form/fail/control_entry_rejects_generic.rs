use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidControlGenericForm,
		action: "/invalid",
		fields: {
			reset: ResetButton<String> {
				label: "Reset",
			}
		}
	};
}
