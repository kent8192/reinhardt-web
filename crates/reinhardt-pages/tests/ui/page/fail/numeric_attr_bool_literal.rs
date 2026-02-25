//! page! macro with numeric attribute having boolean literal

use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|| {
		input {
			r#type: "number",
			min: true,
		}
	});
}
