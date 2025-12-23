//! page! macro with empty element bodies

use reinhardt_pages::page;

fn main() {
	// Empty div
	let _empty_div = page!(|| { div {} });

	// Empty span with attribute only
	let _empty_span = page!(|| {
		span {
			class: "spacer",
		}
	});

	// Nested empty elements
	let _nested_empty = page!(|| {
		div {
			class: "container",
			div {}
			div {}
		}
	});

	// Empty element in condition
	let _conditional_empty = page!(|show: bool| {
		div {
			if show {
				span {}
			}
		}
	});

	// Empty element in loop
	let _loop_empty = page!(|count: usize| {
		div {
			for _i in 0 .. count {
				br {}
			}
		}
	});
}
