//! page! macro with numeric attribute having float literal

use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|| {
		textarea {
			rows: 10.5,
			cols: 80,
		}
	});
}
