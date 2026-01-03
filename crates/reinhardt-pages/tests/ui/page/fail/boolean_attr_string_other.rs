//! page! macro with boolean attribute having arbitrary string literal

use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|| {
		button {
			disabled: "disabled",
			"Submit"
		}
	});
}
