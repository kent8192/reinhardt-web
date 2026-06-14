use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: LegacyStateForm,
		action: "/legacy",
		state: {
			loading,
			error,
			success
		},
		fields: {
			name: CharField {
				initial: "",
			}
		}
	};
}
