//! page! macro with boolean attribute having boolean literal false

use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|| {
		button {
			disabled: false,
			"Submit"
		}
	});
}
