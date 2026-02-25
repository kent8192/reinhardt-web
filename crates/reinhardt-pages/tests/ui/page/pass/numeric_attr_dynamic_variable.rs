//! page! macro with numeric attributes using variables

use reinhardt_pages::page;

fn main() {
	let max_len = "100";
	let num_rows = "10";

	let _valid = page!(|| {
		div {
			input {
				r#type: "text",
				maxlength: max_len,
			}
			textarea {
				rows: num_rows,
				cols: 80,
			}
		}
	});
}
