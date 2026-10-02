//! page! macro with numeric attributes using integer literals

use reinhardt_pages::page;

fn main() {
	let _valid = page!(|| {
		div {
			input {
				r#type: "text",
				aria_label: "Limited input",
				maxlength: 100,
			}
			textarea {
				aria_label: "Sized textarea",
				rows: 10,
				cols: 80,
			}
			td {
				colspan: 2,
				rowspan: 3,
			}
		}
	});
}
