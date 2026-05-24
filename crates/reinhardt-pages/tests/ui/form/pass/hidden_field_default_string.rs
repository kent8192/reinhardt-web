//! `HiddenField` without generic args should still compile and produce
//! `Signal<String>` (backward compatibility).

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: LegacyForm,
		action: "/api/legacy",

		fields: {
			note: HiddenField {
				initial: "hello".to_string(),
			},
		},
	};
}
