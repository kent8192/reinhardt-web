//! page! macro with boolean attribute having integer literal

use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|| {
		button {
			disabled: 1,
			"Submit"
		}
	});
}
