//! page! macro with numeric attributes using function calls.
//!
//! Spec §3.7 (no implicit captures): single-segment lowercase idents are
//! treated as value bindings. Free functions are referenced via the
//! `self::` prefix so the path is multi-segment.

use reinhardt_pages::page;

fn get_max_len() -> &'static str {
	"100"
}

fn calculate_rows(base: i32) -> String {
	(base * 2).to_string()
}

fn main() {
	let _valid = page!(|| {
		div {
			input {
				r#type: "text",
				maxlength: self::get_max_len(),
			}
			input {
				r#type: "number",
				min: 0,
				max: self::calculate_rows(50),
			}
		}
	});
}
