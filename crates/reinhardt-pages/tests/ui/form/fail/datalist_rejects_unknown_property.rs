use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidDatalistPropertyForm,
		action: "/invalid",
		fields: {
			suggestions: Datalist {
				options: [("a", "A")],
				label: "Suggestions",
			}
		}
	};
}
