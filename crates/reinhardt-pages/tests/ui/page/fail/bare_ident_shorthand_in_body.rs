//! page! macro with bare-identifier shorthand in element body
//!
//! Verifies that `div { name }` is a hard compile error in v2 (spec §3.6).
//! Users must wrap the expression in braces: `div { {name} }`.

use reinhardt_pages::page;

fn main() {
	let name = String::from("world");
	let _ = page!(|name: String| {
		div {
			name
		}
	})(name);
}
