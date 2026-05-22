//! Compile-pass: `use_effect` with an explicit single-element deps tuple
//! is the canonical React-parity shape (spec §4.2).

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_effect;

fn main() {
	let count = Signal::new(0_i32);
	let _e = use_effect(
		{
			let count = count.clone();
			move || {
				let _ = count.get();
			}
		},
		(count.clone(),),
	);
	let _ = count;
}
