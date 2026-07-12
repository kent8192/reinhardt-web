use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: UnknownNativeAttrForm,
		action: "/invalid",
		fields: {
			avatar: FileField {
				captrue: "environment",
			}
		}
	};
}
