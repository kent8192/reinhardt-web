use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: ValueFieldListForm,
		action: "/invalid",
		fields: {
			query: CharField {
				widget: SearchInput,
				list: suggestions,
			}
			suggestions: CharField {}
		}
	};
}
