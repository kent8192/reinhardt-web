//! page! macro must have a body

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	// Missing body - should fail
	let _invalid = page!(||);
}
