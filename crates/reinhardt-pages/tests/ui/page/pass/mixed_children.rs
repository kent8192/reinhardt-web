//! page! macro with mixed child types (text, elements, expressions)

use reinhardt_pages::page;

fn main() {
	// Text and elements mixed
	let _mixed = page!(|| {
		div {
			span {
				"Hello, "
			}
			strong {
				"World"
			}
			span {
				"!"
			}
		}
	});

	// Nested mixed content with conditional
	let _nested_mixed = page!(|show_extra: bool| {
		div {
			span {
				"Start: "
			}
			if show_extra {
				span {
					"Extra content "
				}
			}
			span {
				"End"
			}
		}
	});

	// Elements with expressions
	let _with_expr = page!(|count: i32| {
		div {
			span {
				"Count: "
			}
			strong {
				{ format!("{}", count) }
			}
		}
	});
}
