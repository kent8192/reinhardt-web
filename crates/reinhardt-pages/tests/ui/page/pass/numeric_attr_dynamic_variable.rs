//! page! macro with numeric attributes using variables.
//!
//! Spec §3.7 (no implicit captures): outer bindings must be declared as
//! explicit closure parameters.

use reinhardt_pages::page;

fn main() {
	let max_len = "100";
	let num_rows = "10";

	let _valid = page!(|max_len: &'static str, num_rows: &'static str| {
		div {
			input {
				r#type: "text",
				aria_label: "Maximum length input",
				maxlength: max_len,
			}
			textarea {
				aria_label: "Sized textarea",
				rows: num_rows,
				cols: 80,
			}
		}
	})(max_len, num_rows);
}
