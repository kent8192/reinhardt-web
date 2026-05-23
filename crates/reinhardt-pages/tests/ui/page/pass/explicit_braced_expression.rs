//! page! macro with explicit braced expression in element body
//!
//! Verifies that the post-v2 explicit form `div { {name} }` compiles cleanly
//! (spec §3.6). This is the replacement for the removed bare-identifier
//! shorthand `div { name }`.

use reinhardt_pages::page;

fn main() {
	let _ = page!(|name: String| {
		div { {name} }
	})(String::from("world"));
}
