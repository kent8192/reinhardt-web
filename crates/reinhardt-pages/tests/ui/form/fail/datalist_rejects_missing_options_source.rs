use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidDatalistSourceForm,
		action: "/invalid",
		fields: {
			suggestions: Datalist {}
		}
	};
}
