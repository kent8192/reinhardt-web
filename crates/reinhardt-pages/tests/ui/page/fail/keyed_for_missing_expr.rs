//! page! macro rejects keyed for loops without a key expression

use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|items: Vec<String>| {
		ul {
			for item in items @key {
				li { { item } }
			}
		}
	});
}
