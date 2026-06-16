use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: FileMultipleForm,
		action: "/upload",
		fields: {
			document: FileField {
				multiple,
			}
		}
	};
}
