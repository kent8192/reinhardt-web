//! Custom intrinsic events require a string literal name.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|| {
		div { @custom(item_selected): |_| {}, }
	});
}
