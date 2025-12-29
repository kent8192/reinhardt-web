//! page! macro with unclosed if block

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Missing closing brace for if block
	let _invalid = page!(|show: bool| {
	div {
		if show {
			span { "Visible" }
		// Missing closing brace here
	}
});
}
