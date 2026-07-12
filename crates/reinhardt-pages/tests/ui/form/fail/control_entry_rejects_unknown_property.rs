use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidControlPropertyForm,
		action: "/invalid",
		fields: {
			reset: ResetButton {
				label: "Reset",
				accept: "text/csv",
			}
		}
	};
}
