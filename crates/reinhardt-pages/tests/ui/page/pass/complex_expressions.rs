//! page! macro with complex expressions

use reinhardt_pages::page;

fn main() {
	// Brace expression for dynamic text
	let _expr = page!(|count: i32| {
		div {
			{ format!("Count: {}", count) }
		}
	});

	// Arithmetic expression
	let _arithmetic = page!(|a: i32, b: i32| {
		div {
			{ format!("Sum: {}", a + b) }
		}
	});

	// Conditional expression (ternary-like)
	let _conditional = page!(|is_active: bool| {
		div {
			class: if is_active { "active" } else { "inactive" },
			"Status indicator"
		}
	});

	// Format expression
	let _format = page!(|count: usize| {
		ul {
			{ format!("Total items: {}", count) }
		}
	});
}
