//! form! emits a use_form runtime contract for FileField and ImageField.

use reinhardt_pages::{form, use_form};

fn main() {
	let upload = form! {
		name: FileFieldsUseForm,
		action: "/upload",
		fields: {
			document: FileField {
				required,
			}
			avatar: ImageField {}
		}
	};

	let runtime = use_form(&upload).build();
	let values = runtime.get_values();
	let _document = values.document;
	let _avatar = values.avatar;
}
