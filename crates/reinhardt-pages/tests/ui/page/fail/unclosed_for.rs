//! page! macro with unclosed for block

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Missing closing brace for for block
	let _invalid = page!(|items: Vec<String>| {
	ul {
		for item in items {
			li { {item} }
		// Missing closing brace here
	}
});
}
