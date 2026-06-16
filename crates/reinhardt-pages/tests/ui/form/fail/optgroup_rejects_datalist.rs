use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidDatalistGroupForm,
		action: "/invalid",
		fields: {
			suggestions: Datalist {
				options: [OptGroup("Grouped") {
					("a", "A"),
				}, ],
			}
		}
	};
}
