//! page! macro with numeric attributes using integer literals

use reinhardt_pages::page;

fn main() {
	let _valid = page!(|| {
		div {
			input {
				r#type: "text",
				maxlength: 100,
			}
			textarea {
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
