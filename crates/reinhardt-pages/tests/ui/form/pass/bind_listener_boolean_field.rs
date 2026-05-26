//! `BooleanField` bind listener uses `.checked()` directly with `Signal<bool>`.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: ToggleForm,
		action: "/api/toggle",

		fields: {
			enabled: BooleanField {
				initial: false,
			}
		}

	};
}
